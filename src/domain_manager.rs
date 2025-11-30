use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{info, warn, error};
use trust_dns_resolver::config::*;
use trust_dns_resolver::TokioAsyncResolver;
use trust_dns_proto::rr::RecordType;
use chrono::{DateTime, Utc};

use crate::database::Database;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DomainRecord {
    pub domain: String,
    pub ip: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_verified: Option<DateTime<Utc>>,
    pub nameservers: Vec<String>,
    pub verification_status: VerificationStatus,
    pub grace_period_ends: Option<DateTime<Utc>>,
    pub discord: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum VerificationStatus {
    Verified,
    PendingVerification,
    FailedVerification,
    GracePeriod,
}

pub struct DomainManager {
    domains: HashMap<String, DomainRecord>,
    resolver: TokioAsyncResolver,
    verification_interval: Duration,
    grace_period: Duration,
    database: Option<Arc<Database>>,
}

impl DomainManager {
    pub fn new() -> Self {
        let resolver = TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        );
        
        Self {
            domains: HashMap::new(),
            resolver,
            verification_interval: Duration::from_secs(3600),
            grace_period: Duration::from_secs(48 * 3600),
            database: None,
        }
    }
    
    pub fn with_database(mut self, database: Arc<Database>) -> Self {
        self.database = Some(database);
        self
    }
    
    pub async fn load_from_database(&mut self) -> Result<()> {
        if let Some(db) = &self.database {
            let db_domains = db.get_all_domains().await?;
            
            for domain in db_domains {
                let record = DomainRecord {
                    domain: domain.domain.clone(),
                    ip: domain.ip_address,
                    enabled: domain.enabled,
                    created_at: domain.created_at,
                    last_verified: domain.last_verified,
                    nameservers: domain.nameservers.unwrap_or_default(),
                    verification_status: if domain.verified { 
                        VerificationStatus::Verified 
                    } else { 
                        VerificationStatus::PendingVerification 
                    },
                    grace_period_ends: None,
                    discord: domain.discord,
                };
                
                self.domains.insert(domain.domain, record);
            }
            
            info!("Loaded {} domains from database", self.domains.len());
        }
        
        Ok(())
    }
    
    pub async fn discover_domain(&mut self, domain: &str) -> Result<()> {
        let domain = domain.to_lowercase();
        
        // Check if domain already exists
        if self.domains.contains_key(&domain) {
            return Ok(());
        }
        
        // Try to auto-discover the domain by checking if it points to our nameservers
        match self.resolver.lookup(domain.clone(), RecordType::NS).await {
            Ok(ns_lookup) => {
                let current_ns: Vec<String> = ns_lookup.iter()
                    .filter_map(|r| r.as_ns().map(|ns| ns.to_string()))
                    .collect();
                
                // Check if domain points to our nameservers
                let our_ns = vec!["ns1.cybertemp.xyz".to_string(), "ns2.cybertemp.xyz".to_string()];
                let has_our_ns = current_ns.iter().any(|ns| {
                    our_ns.iter().any(|our_ns| ns.contains(our_ns))
                });
                
                if has_our_ns {
                    // Auto-add this domain to our database
                    let ip = if domain.contains("discord") { 
                        "37.114.41.81".to_string() 
                    } else { 
                        "45.134.39.50".to_string() 
                    };
                    
                    let discord = domain.contains("discord");
                    
                    if let Some(db) = &self.database {
                        db.add_domain(&domain, &ip, discord).await?;
                    }
                    
                    let record = DomainRecord {
                        domain: domain.clone(),
                        ip,
                        enabled: true,
                        created_at: Utc::now(),
                        last_verified: Some(Utc::now()),
                        nameservers: current_ns,
                        verification_status: VerificationStatus::Verified,
                        grace_period_ends: None,
                        discord,
                    };
                    
                    self.domains.insert(domain.clone(), record);
                    info!("Discovered and added domain: {}", domain);
                }
            }
            Err(e) => {
                warn!("Failed to discover domain {}: {}", domain, e);
            }
        }
        
        Ok(())
    }
    
    pub async fn verify_domain(&mut self, domain: &str) -> bool {
        let domain = domain.to_lowercase();
        
        match self.resolver.lookup(domain.clone(), RecordType::NS).await {
            Ok(ns_lookup) => {
                let current_ns: Vec<String> = ns_lookup.iter()
                    .filter_map(|r| r.as_ns().map(|ns| ns.to_string()))
                    .collect();
                
                if let Some(record) = self.domains.get_mut(&domain) {
                    record.nameservers = current_ns.clone();
                    record.last_verified = Some(Utc::now());
                    
                    // Check if our nameservers are configured
                    let our_ns = vec!["ns1.cybertemp.xyz".to_string(), "ns2.cybertemp.xyz".to_string()];
                    let has_our_ns = current_ns.iter().any(|ns| {
                        our_ns.iter().any(|our_ns| ns.contains(our_ns))
                    });
                    
                    if has_our_ns {
                        record.verification_status = VerificationStatus::Verified;
                        record.grace_period_ends = None;
                        
                        // Update database
                        if let Some(db) = &self.database {
                            if let Err(e) = db.update_domain_verification(&domain, true, &current_ns).await {
                                error!("Failed to update database for domain {}: {}", domain, e);
                            }
                        }
                        
                        info!("Domain {} verified with correct nameservers", domain);
                    } else {
                        if record.verification_status == VerificationStatus::Verified {
                            // Domain was verified but now lost nameservers - start grace period
                            record.verification_status = VerificationStatus::GracePeriod;
                            record.grace_period_ends = Some(Utc::now() + chrono::Duration::from_std(self.grace_period).unwrap());
                            warn!("Domain {} lost nameservers, starting 48h grace period", domain);
                        } else if record.verification_status == VerificationStatus::GracePeriod {
                            // Check if grace period expired
                            if let Some(grace_end) = record.grace_period_ends {
                                if Utc::now() > grace_end {
                                    record.enabled = false;
                                    record.verification_status = VerificationStatus::FailedVerification;
                                    
                                    // Remove from database
                                    if let Some(db) = &self.database {
                                        if let Err(e) = db.remove_domain(&domain).await {
                                            error!("Failed to remove domain {} from database: {}", domain, e);
                                        }
                                    }
                                    
                                    warn!("Domain {} grace period expired, disabling", domain);
                                }
                            }
                        } else {
                            record.verification_status = VerificationStatus::PendingVerification;
                        }
                    }
                    
                    return has_our_ns;
                }
            }
            Err(e) => {
                if let Some(record) = self.domains.get_mut(&domain) {
                    record.verification_status = VerificationStatus::FailedVerification;
                    record.last_verified = Some(Utc::now());
                    warn!("Failed to verify domain {}: {}", domain, e);
                }
            }
        }
        
        false
    }
    
    pub async fn start_verification_loop(manager: Arc<RwLock<Self>>) {
        let mut interval = interval(manager.read().await.verification_interval);
        
        loop {
            interval.tick().await;
            if let Err(e) = manager.write().await.verify_all_domains().await {
                error!("Verification loop error: {}", e);
            }
        }
    }
    
    pub async fn verify_all_domains(&mut self) -> Result<()> {
        let domains: Vec<String> = self.domains.keys().cloned().collect();
        
        for domain in domains {
            self.verify_domain(&domain).await;
        }
        
        Ok(())
    }
    
    pub async fn get_domain(&self, domain: &str) -> Option<DomainRecord> {
        let domain = domain.to_lowercase();
        self.domains.get(&domain).cloned()
    }
    
    pub async fn get_all_domains(&self) -> Vec<DomainRecord> {
        self.domains.values().cloned().collect()
    }
    
    pub async fn list_domains(&self) -> Vec<String> {
        self.domains.keys().cloned().collect()
    }
    
    pub async fn add_domain(&mut self, domain: &str, ip: &str, discord: bool) -> Result<()> {
        let domain = domain.to_lowercase();
        
        let record = DomainRecord {
            domain: domain.clone(),
            ip: if discord { "37.114.41.81".to_string() } else { ip.to_string() },
            enabled: true,
            created_at: Utc::now(),
            last_verified: None,
            nameservers: Vec::new(),
            verification_status: VerificationStatus::PendingVerification,
            grace_period_ends: None,
            discord,
        };
        
        // Add to database
        if let Some(db) = &self.database {
            db.add_domain(&domain, &record.ip, discord).await?;
        }
        
        self.domains.insert(domain.clone(), record);
        
        info!("Added domain: {} -> {} (discord: {})", domain, ip, discord);
        Ok(())
    }
    
    pub async fn auto_discover_domains(&mut self) -> Result<()> {
        // TODO: Implement auto-discovery logic
        Ok(())
    }
    
    pub async fn remove_domain(&mut self, domain: &str) -> Result<()> {
        let domain = domain.to_lowercase();
        
        if self.domains.remove(&domain).is_some() {
            // Remove from database
            if let Some(db) = &self.database {
                db.remove_domain(&domain).await?;
            }
            
            info!("Removed domain: {}", domain);
            Ok(())
        } else {
            warn!("Domain not found: {}", domain);
            Err(anyhow::anyhow!("Domain not found: {}", domain))
        }
    }
}

impl Default for DomainManager {
    fn default() -> Self {
        Self::new()
    }
}