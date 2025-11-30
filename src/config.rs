use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DnsConfig {
    pub bind_address: String,
    pub port: u16,
    pub default_ttl: u32,
    pub mx_priority: u16,
    pub mail_server: String,
    pub nameservers: Vec<String>,
    pub verification_interval_seconds: u64,
    pub grace_period_hours: i64,
    
    // Database configuration
    pub database_url: String,
    
    // Mail server IPs
    pub mail_server_ips: Vec<String>,
    
    // HTTP redirect configuration
    pub http_redirect_enabled: bool,
    pub http_redirect_port: u16,
    pub redirect_target: String,
    
    // Supabase configuration
    pub supabase_url: Option<String>,
    pub supabase_key: Option<String>,
    
    // Auto-discovery
    pub auto_discovery_enabled: bool,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0".to_string(),
            port: 53,
            default_ttl: 300,
            mx_priority: 10,
            mail_server: "mail.{domain}".to_string(),
            nameservers: vec!["ns1.cybertemp.xyz".to_string(), "ns2.cybertemp.xyz".to_string()],
            verification_interval_seconds: 3600,
            grace_period_hours: 48,
            database_url: "postgresql://dns_user:dns_password@localhost/dns_server".to_string(),
            mail_server_ips: vec!["45.134.39.50".to_string(), "37.114.41.81".to_string()],
            http_redirect_enabled: true,
            http_redirect_port: 80,
            redirect_target: "https://cybertemp.xyz".to_string(),
            auto_discovery_enabled: true,
            supabase_url: None,
            supabase_key: None,
        }
    }
}