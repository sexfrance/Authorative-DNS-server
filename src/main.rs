use clap::{Arg, Command};
use cybertemp_dns::DnsServer;
use tracing::{info, error};
use tracing_subscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables from .env file
    dotenv::dotenv().ok();
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();
    
    let matches = Command::new("cybertemp-dns")
        .version("0.1.0")
        .about("Production DNS server for Cybertemp.xyz mail service")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file")
                .default_value("config/dns.toml"),
        )
        .arg(
            Arg::new("daemon")
                .short('d')
                .long("daemon")
                .help("Run as daemon")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let config_path = matches.get_one::<String>("config").unwrap();
    let daemon_mode = matches.get_flag("daemon");
    
    if daemon_mode {
        info!("Starting Cybertemp DNS server in daemon mode...");
    } else {
        info!("Starting Cybertemp DNS server...");
    }
    
    // Handle panics gracefully
    std::panic::set_hook(Box::new(|panic_info| {
        error!("Panic occurred: {}", panic_info);
        std::process::exit(1);
    }));
    
    match DnsServer::new(config_path).await {
        Ok(mut server) => {
            info!("DNS server initialized successfully");
            if let Err(e) = server.run().await {
                error!("DNS server error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            error!("Failed to initialize DNS server: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}