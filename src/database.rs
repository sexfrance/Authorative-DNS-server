use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use chrono::{DateTime, Utc};
use tracing::info;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Domain {
    pub id: String,
    pub domain: String,
    pub ip_address: String,
    pub mail_server: String,
    pub mx_priority: i32,
    pub enabled: bool,
    pub verified: bool,
    pub last_verified: Option<DateTime<Utc>>,
    pub nameservers: Option<Vec<String>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub discord: bool,
    pub description: Option<String>,
    pub tags: Option<Vec<String>>,
}

pub struct Database {
    pool: Pool<Postgres>,
}

impl Database {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
            
        info!("Connected to PostgreSQL database");
        Ok(Self { pool })
    }
    
    pub async fn get_all_domains(&self) -> Result<Vec<Domain>> {
        let rows = sqlx::query(
            r#"
            SELECT 
                id::text as id,
                domain,
                ip_address::text as ip_address,
                mail_server,
                mx_priority,
                enabled,
                verified,
                last_verified,
                nameservers,
                created_at,
                updated_at,
                discord,
                description,
                tags
            FROM domains 
            WHERE enabled = true
            ORDER BY domain
            "#
        )
        .fetch_all(&self.pool)
        .await?;
        
        let domains = rows.into_iter().map(|row| Domain {
            id: row.get("id"),
            domain: row.get("domain"),
            ip_address: row.get("ip_address"),
            mail_server: row.get("mail_server"),
            mx_priority: row.get("mx_priority"),
            enabled: row.get("enabled"),
            verified: row.get("verified"),
            last_verified: row.get("last_verified"),
            nameservers: row.get("nameservers"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            discord: row.get("discord"),
            description: row.get("description"),
            tags: row.get("tags"),
        }).collect();
        
        Ok(domains)
    }
    
    pub async fn get_domain(&self, domain_name: &str) -> Result<Option<Domain>> {
        let row = sqlx::query(
            r#"
            SELECT 
                id::text as id,
                domain,
                ip_address::text as ip_address,
                mail_server,
                mx_priority,
                enabled,
                verified,
                last_verified,
                nameservers,
                created_at,
                updated_at,
                discord,
                description,
                tags
            FROM domains 
            WHERE domain = $1 AND enabled = true
            "#
        )
        .bind(domain_name.to_lowercase())
        .fetch_optional(&self.pool)
        .await?;
        
        let domain = row.map(|row| Domain {
            id: row.get("id"),
            domain: row.get("domain"),
            ip_address: row.get("ip_address"),
            mail_server: row.get("mail_server"),
            mx_priority: row.get("mx_priority"),
            enabled: row.get("enabled"),
            verified: row.get("verified"),
            last_verified: row.get("last_verified"),
            nameservers: row.get("nameservers"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            discord: row.get("discord"),
            description: row.get("description"),
            tags: row.get("tags"),
        });
        
        Ok(domain)
    }
    
    pub async fn add_domain(&self, domain: &str, ip_address: &str, discord: bool) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO domains (domain, ip_address, discord)
            VALUES ($1, $2::inet, $3)
            ON CONFLICT (domain) DO UPDATE 
            SET ip_address = $2::inet, discord = $3, updated_at = NOW()
            "#
        )
        .bind(domain.to_lowercase())
        .bind(ip_address)
        .bind(discord)
        .execute(&self.pool)
        .await?;
        
        info!("Added/updated domain: {} -> {} (discord: {})", domain, ip_address, discord);
        Ok(())
    }
    
    pub async fn remove_domain(&self, domain: &str) -> Result<()> {
        sqlx::query(
            "UPDATE domains SET enabled = false, updated_at = NOW() WHERE domain = $1"
        )
        .bind(domain.to_lowercase())
        .execute(&self.pool)
        .await?;
        
        info!("Disabled domain: {}", domain);
        Ok(())
    }
    
    pub async fn update_domain_verification(&self, domain: &str, verified: bool, nameservers: &[String]) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE domains 
            SET verified = $1, last_verified = NOW(), nameservers = $2, updated_at = NOW()
            WHERE domain = $3
            "#
        )
        .bind(verified)
        .bind(nameservers)
        .bind(domain.to_lowercase())
        .execute(&self.pool)
        .await?;
        
        Ok(())
    }
}