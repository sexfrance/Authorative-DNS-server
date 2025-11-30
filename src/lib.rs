pub mod dns_server;
pub mod domain_manager;
pub mod dns_handler;
pub mod database;  // <-- ADD THIS LINE
pub mod supabase_client;
pub mod config;
pub mod http_redirect;

pub use dns_server::DnsServer;
pub use domain_manager::{DomainManager, DomainRecord, VerificationStatus};
pub use dns_handler::CybertempHandler;
pub use database::Database;  // <-- ADD THIS LINE
pub use supabase_client::SupabaseClient;
pub use config::DnsConfig;
pub use http_redirect::start_http_redirect_server;