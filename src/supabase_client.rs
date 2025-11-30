use anyhow::Result;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tracing::{info, warn, error};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SupabaseDomain {
    pub id: String,
    pub user_id: String,
    pub domain: String,
    pub cloudflare_domain: bool,
    pub pending_ns_check: bool,
    pub added_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub active: bool,
    pub discord: bool,
    pub stripe_payment_id: Option<String>,
    pub payment_status: String,
    pub amount_paid: Option<f64>,
    pub is_one_time_purchase: bool,
}

pub struct SupabaseClient {
    client: reqwest::Client,
    url: String,
    key: String,
}

impl SupabaseClient {
    pub fn new(url: String, key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            url,
            key,
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.url.is_empty() && !self.key.is_empty()
    }

    pub async fn sync_from_supabase(&self, database: &super::database::Database) -> Result<()> {
        if !self.is_configured() {
            return Ok(());
        }

        let domains = self.get_all_domains().await?;
        
        for supabase_domain in &domains {
            if supabase_domain.active {
                // Convert Cybertemp domain to our internal format
                let ip = if supabase_domain.discord {
                    "37.114.41.81".to_string()
                } else {
                    "45.134.39.50".to_string()
                };
                
                // Add to our internal PostgreSQL database
                database.add_domain(&supabase_domain.domain, &ip, supabase_domain.discord).await?;
                
                // Update pending_ns_check based on our verification status
                if let Some(internal_domain) = database.get_domain(&supabase_domain.domain).await? {
                    let mut updates = HashMap::new();
                    updates.insert("pending_ns_check", serde_json::Value::Bool(!internal_domain.verified));
                    updates.insert("updated_at", serde_json::Value::String(Utc::now().to_rfc3339()));
                    
                    self.update_domain(&supabase_domain.id, updates).await?;
                }
            }
        }
        
        info!("Synced {} domains from Supabase to internal database", domains.len());
        Ok(())
    }

    pub async fn sync_to_supabase(&self, database: &super::database::Database) -> Result<()> {
        if !self.is_configured() {
            return Ok(());
        }

        let internal_domains = database.get_all_domains().await?;
        let supabase_domains = self.get_all_domains().await?;
        
        // Create a map of existing Supabase domains for quick lookup
        let supabase_domain_map: HashMap<String, SupabaseDomain> = supabase_domains
            .into_iter()
            .map(|d| (d.domain.clone(), d))
            .collect();
        
        for internal_domain in internal_domains {
            if let Some(supabase_domain) = supabase_domain_map.get(&internal_domain.domain) {
                // Update existing Supabase domain
                let mut updates = HashMap::new();
                updates.insert("pending_ns_check", serde_json::Value::Bool(!internal_domain.verified));
                updates.insert("discord", serde_json::Value::Bool(internal_domain.discord));
                updates.insert("updated_at", serde_json::Value::String(Utc::now().to_rfc3339()));
                
                self.update_domain(&supabase_domain.id, updates).await?;
            } else {
                // This domain exists in our internal DB but not in Supabase
                // We might want to create it in Supabase or just log it
                warn!("Domain {} exists in internal DB but not in Supabase", internal_domain.domain);
            }
        }
        
        info!("Synced internal database state to Supabase");
        Ok(())
    }

    pub async fn get_all_domains(&self) -> Result<Vec<SupabaseDomain>> {
        if !self.is_configured() {
            return Ok(Vec::new());
        }

        let response = self.client
            .get(&format!("{}/rest/v1/domains", self.url))
            .header("apikey", &self.key)
            .header("Authorization", &format!("Bearer {}", self.key))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Supabase API error: {}", error_text);
            return Err(anyhow::anyhow!("Supabase API error: {}", error_text));
        }

        let domains: Vec<SupabaseDomain> = response.json().await?;
        info!("Loaded {} domains from Supabase", domains.len());
        Ok(domains)
    }

    pub async fn update_domain(&self, domain_id: &str, updates: HashMap<&str, serde_json::Value>) -> Result<()> {
        if !self.is_configured() {
            return Ok(());
        }

        let response = self.client
            .patch(&format!("{}/rest/v1/domains?id=eq.{}", self.url, domain_id))
            .header("apikey", &self.key)
            .header("Authorization", &format!("Bearer {}", self.key))
            .header("Content-Type", "application/json")
            .header("Prefer", "return=minimal")
            .json(&updates)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Supabase update error: {}", error_text);
            return Err(anyhow::anyhow!("Supabase update error: {}", error_text));
        }

        Ok(())
    }

    pub async fn delete_domain(&self, domain_id: &str) -> Result<()> {
        if !self.is_configured() {
            return Ok(());
        }

        let response = self.client
            .delete(&format!("{}/rest/v1/domains?id=eq.{}", self.url, domain_id))
            .header("apikey", &self.key)
            .header("Authorization", &format!("Bearer {}", self.key))
            .header("Prefer", "return=minimal")
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Supabase delete error: {}", error_text);
            return Err(anyhow::anyhow!("Supabase delete error: {}", error_text));
        }

        info!("Deleted domain {} from Supabase", domain_id);
        Ok(())
    }

    pub async fn get_domains_pending_ns_check(&self) -> Result<Vec<SupabaseDomain>> {
        if !self.is_configured() {
            return Ok(Vec::new());
        }

        let response = self.client
            .get(&format!("{}/rest/v1/domains?pending_ns_check=eq.true", self.url))
            .header("apikey", &self.key)
            .header("Authorization", &format!("Bearer {}", self.key))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Supabase API error: {}", error_text);
            return Err(anyhow::anyhow!("Supabase API error: {}", error_text));
        }

        let domains: Vec<SupabaseDomain> = response.json().await?;
        Ok(domains)
    }
}