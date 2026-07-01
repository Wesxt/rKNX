use std::env;
use std::process;
use rknx::config::Config;
use rknx::connection::KnxService;
use rknx::connection::server::KnxNetIpServer;
use rknx::connection::tunneling::KnxTunneling;
use rknx::connection::router::Router;
use rknx::utils::logger::setup_logger;

fn print_help() {
    println!("rKNX Daemon CLI");
    println!("Usage:");
    println!("  rknx [options]");
    println!();
    println!("Options:");
    println!("  -c, --config <file>  Path to the TOML configuration file (default: /etc/rknx/config.toml)");
    println!("  -h, --help           Show this help information");
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut config_path = "/etc/rknx/config.toml".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--config" => {
                if i + 1 < args.len() {
                    config_path = args[i + 1].clone();
                    i += 2;
                } else {
                    eprintln!("Error: Missing value for config option");
                    print_help();
                    process::exit(1);
                }
            }
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            other => {
                eprintln!("Error: Unknown option '{}'", other);
                print_help();
                process::exit(1);
            }
        }
    }

    println!("[INFO] Loading configuration from '{}'...", config_path);
    let config = match Config::load_from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[ERROR] Failed to load configuration: {}", e);
            process::exit(1);
        }
    };

    if let Some(ref log_cfg) = config.logging {
        setup_logger(
            log_cfg.level.clone(),
            log_cfg.log_to_file,
            log_cfg.log_dir.clone(),
            log_cfg.log_filename.clone(),
        );
    }

    let mut has_service = false;

    // Start router if configured
    let router = if let Some(router_opts) = config.to_router_options() {
        println!(
            "[INFO] Starting KNXnet/IP Router on IA: {}...",
            router_opts.individual_address
        );
        let r = Router::new(router_opts);
        if let Err(e) = r.connect_all().await {
            eprintln!("[ERROR] Failed to start KNXnet/IP Router: {:?}", e);
            process::exit(1);
        }
        println!("[INFO] KNXnet/IP Router is running successfully with {} links.", r.link_count());
        has_service = true;
        Some(r)
    } else {
        None
    };

    // Start server if configured
    let server = if let Some(server_opts) = config.to_server_options() {
        println!(
            "[INFO] Starting KNXnet/IP Server on {}:{} (IA: {}, friendly name: '{}')...",
            server_opts.local_ip, server_opts.port, server_opts.individual_address, server_opts.friendly_name
        );
        let s = KnxNetIpServer::new(server_opts);
        if let Err(e) = s.connect().await {
            eprintln!("[ERROR] Failed to start KNXnet/IP Server: {:?}", e);
            process::exit(1);
        }
        println!("[INFO] KNXnet/IP Server is running successfully.");
        has_service = true;
        Some(s)
    } else {
        None
    };

    // Start client tunneling if configured
    let client = if let Some(client_opts) = config.to_tunneling_options() {
        println!(
            "[INFO] Connecting to KNXnet/IP Gateway at {}:{}...",
            client_opts.ip, client_opts.port
        );
        let c = KnxTunneling::new(client_opts);
        if let Err(e) = c.connect().await {
            eprintln!("[ERROR] Failed to connect to KNXnet/IP Gateway: {:?}", e);
            process::exit(1);
        }
        println!("[INFO] Connected to KNXnet/IP Gateway successfully.");
        has_service = true;
        Some(c)
    } else {
        None
    };

    if !has_service {
        eprintln!("[ERROR] No server, client, or router services configured. Exiting.");
        process::exit(1);
    }

    println!("[INFO] rKNX daemon is running. Press Ctrl+C to terminate.");
    tokio::signal::ctrl_c().await.ok();
    println!("[INFO] Shutting down services...");

    if let Some(r) = router {
        r.disconnect_all().await;
    }
    if let Some(s) = server {
        let _ = s.disconnect().await;
    }
    if let Some(c) = client {
        let _ = c.disconnect().await;
    }

    println!("[INFO] Shutdown complete.");
}
