use anyhow::Result;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use tracing::{info, error, warn};

use crate::config::DnsConfig;
use crate::domain_manager::DomainManager;
use crate::dns_handler::CybertempHandler;
use crate::database::Database;
use crate::supabase_client::SupabaseClient;
use crate::http_redirect::start_http_redirect_server;

use hyper::{Body, Request, Response, Method, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::convert::Infallible;
use serde_json::json;

pub struct DnsServer {
    config: DnsConfig,
    domain_manager: Arc<RwLock<DomainManager>>,
    supabase_client: Option<Arc<SupabaseClient>>,
    database: Arc<Database>,
}

impl DnsServer {
    pub async fn new(config_path: &str) -> Result<Self> {
        let settings = config::Config::builder()
            .add_source(config::Config::try_from(&DnsConfig::default())?)
            .add_source(config::File::with_name(config_path).required(false))
            .build()?;
            
        let config: DnsConfig = settings.try_deserialize()?;
        
        info!("Initializing DNS server...");
        
        // Initialize internal PostgreSQL database
        let database = Database::new(&config.database_url).await?;
        let database_arc = Arc::new(database);
        
        // Initialize Supabase client if configured
        let supabase_client = if let (Some(url), Some(key)) = (&config.supabase_url, &config.supabase_key) {
            let client = SupabaseClient::new(url.clone(), key.clone());
            if client.is_configured() {
                info!("Supabase client configured for URL: {}", url);
                Some(Arc::new(client))
            } else {
                warn!("Supabase configuration incomplete, skipping Supabase integration");
                None
            }
        } else {
            info!("No Supabase configuration found, running in standalone mode");
            None
        };
        
        // Sync from Supabase if available
        if let Some(supabase) = &supabase_client {
            info!("Syncing domains from Supabase...");
            match supabase.sync_from_supabase(&database_arc).await {
                Ok(_) => info!("Successfully synced domains from Supabase"),
                Err(e) => error!("Failed to sync from Supabase: {}", e),
            }
        }
        
        let mut domain_manager = DomainManager::new().with_database(database_arc.clone());
        
        // Load domains from internal database
        info!("Loading domains from internal database...");
        domain_manager.load_from_database().await?;
        
        let domain_manager = Arc::new(RwLock::new(domain_manager));
        
        Ok(Self {
            config,
            domain_manager,
            supabase_client,
            database: database_arc,
        })
    }
    
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting DNS server components...");
        
        // Start domain verification loop
        let verification_manager = self.domain_manager.clone();
        let verification_interval = self.config.verification_interval_seconds;
        tokio::spawn(async move {
            info!("Starting domain verification loop (interval: {}s)", verification_interval);
            let mut interval = interval(Duration::from_secs(verification_interval));
            
            loop {
                interval.tick().await;
                if let Err(e) = verification_manager.write().await.verify_all_domains().await {
                    error!("Domain verification error: {}", e);
                }
            }
        });
        
        // Start Supabase sync loop if configured
        if let Some(supabase) = self.supabase_client.clone() {
            let database = self.database.clone();
            let domain_manager = self.domain_manager.clone();
            
            tokio::spawn(async move {
                info!("Starting Supabase sync loop (interval: 300s)");
                let mut interval = interval(Duration::from_secs(300)); // Sync every 5 minutes
                
                loop {
                    interval.tick().await;
                    info!("Syncing to Supabase...");
                    if let Err(e) = supabase.sync_to_supabase(&database).await {
                        error!("Supabase sync error: {}", e);
                    } else {
                        info!("Successfully synced to Supabase");
                    }
                    
                    // Reload domains from database after sync
                    if let Err(e) = domain_manager.write().await.load_from_database().await {
                        error!("Failed to reload domains after sync: {}", e);
                    }
                }
            });
        }
        
        // Start HTTP redirect server if enabled
        if self.config.http_redirect_enabled {
            let redirect_manager = self.domain_manager.clone();
            let bind_addr = self.config.bind_address.clone();
            let port = self.config.http_redirect_port;
            let target = self.config.redirect_target.clone();
            
            tokio::spawn(async move {
                info!("Starting HTTP redirect server on {}:{}", bind_addr, port);
                if let Err(e) = start_http_redirect_server(&bind_addr, port, &target, redirect_manager).await {
                    error!("HTTP redirect server error: {}", e);
                }
            });
        }
        
        // Start auto-discovery loop if enabled
        if self.config.auto_discovery_enabled {
            let discovery_manager = self.domain_manager.clone();
            let discovery_interval = self.config.verification_interval_seconds;
            
            tokio::spawn(async move {
                info!("Starting auto-discovery loop (interval: {}s)", discovery_interval);
                let mut interval = interval(Duration::from_secs(discovery_interval));
                
                loop {
                    interval.tick().await;
                    if let Err(e) = discovery_manager.write().await.auto_discover_domains().await {
                        error!("Auto-discovery error: {}", e);
                    }
                }
            });
        }
        
        // Start main DNS server
        self.start_dns_server().await
    }
    
    async fn start_dns_server(&self) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", self.config.bind_address, self.config.port)
            .parse()?;
            
        let handler = CybertempHandler::new(
            self.config.clone(),
            self.domain_manager.clone(),
        );
        
        info!("Starting DNS server on {}", addr);
        
        let socket = tokio::net::UdpSocket::bind(&addr).await?;
        info!("DNS server bound to {}", addr);
        
        let mut buf = [0u8; 512];
        
        loop {
            match socket.recv_from(&mut buf).await {
                Ok((len, src)) => {
                    let data = buf[..len].to_vec();
                    
                    if let Ok(response_data) = handler.handle_request(&data).await {
                        if let Err(e) = socket.send_to(&response_data, src).await {
                            error!("Error sending DNS response: {}", e);
                        }
                    } else {
                        error!("Error handling DNS request");
                    }
                }
                Err(e) => {
                    error!("Error receiving DNS packet: {}", e);
                }
            }
        }
    }
    
    // Domain management API methods
    pub async fn add_domain(&self, domain: &str, ip: &str, discord: bool) -> Result<()> {
        let mut manager = self.domain_manager.write().await;
        manager.add_domain(domain, ip, discord).await?;
        
        // Sync to Supabase if configured
        if let Some(supabase) = &self.supabase_client {
            if let Err(e) = supabase.sync_to_supabase(&self.database).await {
                error!("Failed to sync new domain to Supabase: {}", e);
            }
        }
        
        Ok(())
    }
    
    pub async fn remove_domain(&self, domain: &str) -> Result<()> {
        let mut manager = self.domain_manager.write().await;
        manager.remove_domain(domain).await?;
        
        // Sync to Supabase if configured
        if let Some(supabase) = &self.supabase_client {
            if let Err(e) = supabase.sync_to_supabase(&self.database).await {
                error!("Failed to sync domain removal to Supabase: {}", e);
            }
        }
        
        Ok(())
    }
    
    pub async fn discover_domain(&self, domain: &str) -> Result<()> {
        let mut manager = self.domain_manager.write().await;
        manager.discover_domain(domain).await?;
        
        // Sync to Supabase if configured
        if let Some(supabase) = &self.supabase_client {
            if let Err(e) = supabase.sync_to_supabase(&self.database).await {
                error!("Failed to sync discovered domain to Supabase: {}", e);
            }
        }
        
        Ok(())
    }
    
    pub async fn list_domains(&self) -> Vec<String> {
        let manager = self.domain_manager.read().await;
        manager.list_domains().await
    }
    
    pub async fn get_domain_info(&self, domain: &str) -> Option<crate::domain_manager::DomainRecord> {
        let manager = self.domain_manager.read().await;
        manager.get_domain(domain).await
    }
    
    pub async fn force_verification(&self, domain: &str) -> Result<bool> {
        let mut manager = self.domain_manager.write().await;
        let verified = manager.verify_domain(domain).await;
        
        // Sync to Supabase if configured
        if let Some(supabase) = &self.supabase_client {
            if let Err(e) = supabase.sync_to_supabase(&self.database).await {
                error!("Failed to sync verification status to Supabase: {}", e);
            }
        }
        
        Ok(verified)
    }
    
    pub async fn get_stats(&self) -> DomainStats {
        let manager = self.domain_manager.read().await;
        let domains = manager.get_all_domains().await;
        
        let total = domains.len();
        let verified = domains.iter().filter(|d| d.enabled && d.verification_status == crate::domain_manager::VerificationStatus::Verified).count();
        let pending = domains.iter().filter(|d| d.enabled && d.verification_status == crate::domain_manager::VerificationStatus::PendingVerification).count();
        let grace_period = domains.iter().filter(|d| d.enabled && d.verification_status == crate::domain_manager::VerificationStatus::GracePeriod).count();
        let discord = domains.iter().filter(|d| d.discord).count();
        
        DomainStats {
            total_domains: total,
            verified_domains: verified,
            pending_verification: pending,
            grace_period: grace_period,
            discord_domains: discord,
            supabase_connected: self.supabase_client.is_some(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DomainStats {
    pub total_domains: usize,
    pub verified_domains: usize,
    pub pending_verification: usize,
    pub grace_period: usize,
    pub discord_domains: usize,
    pub supabase_connected: bool,
}

// API server for remote management
pub struct DnsApiServer {
    dns_server: Arc<DnsServer>,
}

impl DnsApiServer {
    pub fn new(dns_server: Arc<DnsServer>) -> Self {
        Self { dns_server }
    }
    
    pub async fn run(&self, bind_addr: &str, port: u16) -> Result<()> {
        let addr: SocketAddr = format!("{}:{}", bind_addr, port).parse()?;
        let dns_server = Arc::clone(&self.dns_server);
        
        let make_svc = make_service_fn(move |_conn| {
            let dns_server = Arc::clone(&dns_server);
            async move {
                Ok::<_, Infallible>(service_fn(move |req| {
                    Self::handle_api_request(req, Arc::clone(&dns_server))
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);
        info!("DNS API server running on http://{}", addr);
        
        server.await?;
        Ok(())
    }
    
    async fn handle_api_request(
        req: Request<Body>,
        dns_server: Arc<DnsServer>,
    ) -> Result<Response<Body>, Infallible> {
        let path = req.uri().path();
        let method = req.method();
        
        match (method, path) {
            (&Method::GET, "/health") => {
                Ok(Response::new(Body::from(json!({"status": "healthy"}).to_string())))
            }
            (&Method::GET, "/stats") => {
                let stats = dns_server.get_stats().await;
                Ok(Response::new(Body::from(serde_json::to_string(&stats).unwrap())))
            }
            (&Method::GET, "/domains") => {
                let domains = dns_server.list_domains().await;
                Ok(Response::new(Body::from(json!(domains).to_string())))
            }
            (&Method::POST, "/domains") => {
                // Parse domain addition request
                let body = hyper::body::to_bytes(req.into_body()).await.unwrap();
                if let Ok(data) = serde_json::from_slice::<serde_json::Value>(&body) {
                    if let (Some(domain), Some(ip)) = (data.get("domain"), data.get("ip")) {
                        let discord = data.get("discord").and_then(|d| d.as_bool()).unwrap_or(false);
                        if let (Some(domain_str), Some(ip_str)) = (domain.as_str(), ip.as_str()) {
                            match dns_server.add_domain(domain_str, ip_str, discord).await {
                                Ok(_) => Ok(Response::new(Body::from(json!({"status": "added"}).to_string()))),
                                Err(e) => Ok(Response::builder()
                                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                                    .body(Body::from(json!({"error": e.to_string()}).to_string()))
                                    .unwrap()),
                            }
                        } else {
                            Ok(Response::builder()
                                .status(StatusCode::BAD_REQUEST)
                                .body(Body::from(json!({"error": "Invalid domain or ip"}).to_string()))
                                .unwrap())
                        }
                    } else {
                        Ok(Response::builder()
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from(json!({"error": "Missing domain or ip"}).to_string()))
                            .unwrap())
                    }
                } else {
                    Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from(json!({"error": "Invalid JSON"}).to_string()))
                        .unwrap())
                }
            }
            (&Method::DELETE, path) if path.starts_with("/domains/") => {
                let domain = path.trim_start_matches("/domains/");
                match dns_server.remove_domain(domain).await {
                    Ok(_) => Ok(Response::new(Body::from(json!({"status": "removed"}).to_string()))),
                    Err(e) => Ok(Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(json!({"error": e.to_string()}).to_string()))
                        .unwrap()),
                }
            }
            _ => {
                Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .body(Body::from(json!({"error": "Not found"}).to_string()))
                    .unwrap())
            }
        }
    }
}