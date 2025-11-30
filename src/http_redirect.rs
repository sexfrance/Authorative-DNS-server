use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;
use std::net::SocketAddr;
use tracing::{info, error};
use crate::domain_manager::DomainManager;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn start_http_redirect_server(
    bind_addr: &str, 
    port: u16, 
    redirect_target: &str,
    domain_manager: Arc<RwLock<DomainManager>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr: SocketAddr = format!("{}:{}", bind_addr, port).parse()?;
    
    let redirect_target = redirect_target.to_string();
    
    let make_svc = make_service_fn(move |_conn| {
        let domain_manager = Arc::clone(&domain_manager);
        let redirect_target = redirect_target.clone();
        
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                handle_http_request(req, Arc::clone(&domain_manager), redirect_target.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    
    info!("HTTP redirect server running on http://{}", addr);
    
    if let Err(e) = server.await {
        error!("HTTP server error: {}", e);
    }

    Ok(())
}

async fn handle_http_request(
    req: Request<Body>,
    domain_manager: Arc<RwLock<DomainManager>>,
    redirect_target: String,
) -> Result<Response<Body>, Infallible> {
    let host = req.uri().host().unwrap_or("").to_lowercase();
    
    // Check if this is one of our domains
    let manager = domain_manager.read().await;
    let is_our_domain = manager.get_domain(&host).await.is_some();
    
    if is_our_domain {
        // Redirect to cybertemp.xyz
        let response = Response::builder()
            .status(StatusCode::MOVED_PERMANENTLY)
            .header("Location", &redirect_target)
            .body(Body::empty())
            .unwrap();
        
        info!("Redirecting {} to {}", host, redirect_target);
        Ok(response)
    } else {
        // Not our domain, return 404
        let response = Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not found"))
            .unwrap();
        Ok(response)
    }
}