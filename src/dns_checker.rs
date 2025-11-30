use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tracing::{info, warn, error};
use trust_dns_resolver::config::*;
use trust_dns_resolver::TokioAsyncResolver;

use crate::domain_manager::DomainManager;
use crate::dns_handler::CybertempHandler;
use crate::config::DnsConfig;

pub struct DnsChecker {
    resolver: TokioAsyncResolver,
    domain_manager: Arc<RwLock<DomainManager>>,
    dns_handler: CybertempHandler,
    check_interval: Duration,
}

impl DnsChecker {
    pub fn new(
        config: DnsConfig,
        domain_manager: Arc<RwLock<DomainManager>>,
        dns_handler: CybertempHandler,
    ) -> Self {
        let resolver = TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        );
        
        Self {
            resolver,
            domain_manager,
            dns_handler,
            check_interval: Duration::from_secs(config.dns_check_interval_seconds),
        }
    }
    
    pub async fn start_check_loop(self: Arc<Self>) {
        let mut interval = interval(self.check_interval);
        
        info!("Starting DNS checker loop");
        
        loop {
            interval.tick().await;
            if let Err(e) = self.check_all_domains().await {
                error!("DNS checker error: {}", e);
            }
        }
    }
    
    async fn check_all_domains(&self) -> Result<()> {
        let domains = {
            let manager = self.domain_manager.read().await;
            manager.get_all_domains().await
        };
        
        for domain_record in domains {
            if !domain_record.enabled {
                continue;
            }
            
            let domain = domain_record.domain.clone();
            let expected_ip = domain_record.ip.clone();
            let is_discord = domain_record.discord;
            
            // Check A record
            if let Err(e) = self.check_and_fix_a_record(&domain, &expected_ip).await {
                warn!("Failed to check A record for {}: {}", domain, e);
            }
            
            // Check MX records
            if let Err(e) = self.check_and_fix_mx_records(&domain, is_discord).await {
                warn!("Failed to check MX records for {}: {}", domain, e);
            }
            
            // Check TXT records
            if let Err(e) = self.check_and_fix_txt_records(&domain).await {
                warn!("Failed to check TXT records for {}: {}", domain, e);
            }
        }
        
        Ok(())
    }
    
    async fn check_and_fix_a_record(&self, domain: &str, expected_ip: &str) -> Result<()> {
        match self.resolver.lookup_ip(domain).await {
            Ok(lookup) => {
                let has_correct_ip = lookup.iter().any(|ip| ip.to_string() == expected_ip);
                
                if !has_correct_ip {
                    warn!("Domain {} has incorrect A record, expected {}", domain, expected_ip);
                    // Here you would implement the actual DNS record update
                    // This would depend on your DNS provider's API
                } else {
                    info!("Domain {} has correct A record", domain);
                }
            }
            Err(e) => {
                warn!("Domain {} has no A record: {}", domain, e);
                // Implement DNS record creation here
            }
        }
        
        Ok(())
    }
    
    async fn check_and_fix_mx_records(&self, domain: &str, is_discord: bool) -> Result<()> {
        match self.resolver.mx_lookup(domain).await {
            Ok(mx_lookup) => {
                let expected_mail_server = if is_discord {
                    format!("mail.{}.discord.cybertemp.xyz", domain)
                } else {
                    format!("mail.{}", domain)
                };
                
                let has_correct_mx = mx_lookup.iter().any(|mx| {
                    mx.exchange().to_ascii().contains(&expected_mail_server)
                });
                
                if !has_correct_mx {
                    warn!("Domain {} has incorrect MX records, expected {}", domain, expected_mail_server);
                    // Implement MX record update here
                } else {
                    info!("Domain {} has correct MX records", domain);
                }
            }
            Err(e) => {
                warn!("Domain {} has no MX records: {}", domain, e);
                // Implement MX record creation here
            }
        }
        
        Ok(())
    }
    
    async fn check_and_fix_txt_records(&self, domain: &str) -> Result<()> {
        match self.resolver.txt_lookup(domain).await {
            Ok(txt_lookup) => {
                let has_spf = txt_lookup.iter().any(|txt| {
                    txt.to_string().contains("v=spf1")
                });
                
                if !has_spf {
                    warn!("Domain {} is missing SPF record", domain);
                    // Implement SPF record creation here
                }
                
                // Check DMARC
                let dmarc_domain = format!("_dmarc.{}", domain);
                match self.resolver.txt_lookup(&dmarc_domain).await {
                    Ok(dmarc_lookup) => {
                        let has_dmarc = dmarc_lookup.iter().any(|txt| {
                            txt.to_string().contains("v=DMARC1")
                        });
                        
                        if !has_dmarc {
                            warn!("Domain {} is missing DMARC record", domain);
                            // Implement DMARC record creation here
                        }
                    }
                    Err(_) => {
                        warn!("Domain {} is missing DMARC record", domain);
                        // Implement DMARC record creation here
                    }
                }
            }
            Err(e) => {
                warn!("Domain {} has no TXT records: {}", domain, e);
                // Implement TXT record creation here
            }
        }
        
        Ok(())
    }
}