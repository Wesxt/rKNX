use rknx::config::Config;
use rknx::connection::KnxService;
use rknx::connection::router::Router;
use rknx::connection::server::KnxNetIpServer;
use rknx::connection::tunneling::KnxTunneling;
use rknx::utils::logger::Logger;
use rknx::utils::logger::setup_logger;
use std::env;
use std::process;
use std::sync::Arc;

fn print_help() {
    println!("rKNX Daemon CLI");
    println!("Usage:");
    println!("  rknx [options]");
    println!();
    println!("Options:");
    println!(
        "  -c, --config <file>  Path to the TOML configuration file (default: /etc/rknx/config.toml)"
    );
    println!("  -h, --help           Show this help information");
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let mut config_path = "/etc/rknx/config.toml".to_string();
    let logger = Logger::new("Main");

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--config" => {
                if i + 1 < args.len() {
                    config_path = args[i + 1].clone();
                    i += 2;
                } else {
                    logger.error("Error: Missing value for config option");
                    print_help();
                    process::exit(1);
                }
            }
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            other => {
                logger.error(&format!("Error: Unknown option '{}'", other));
                print_help();
                process::exit(1);
            }
        }
    }

    logger.info(&format!("Loading configuration from '{}'...", config_path));
    let config = match Config::load_from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            logger.error(&format!("Failed to load configuration: {}", e));
            process::exit(1);
        }
    };

    if let Some(ref log_cfg) = config.logging {
        setup_logger(
            log_cfg.level.clone(),
            log_cfg.log_to_file,
            log_cfg.log_dir.clone(),
            log_cfg.log_filename.clone(),
            log_cfg.indications,
            log_cfg.indications_raw,
            log_cfg.node_format,
        );
    }

    let mut has_service = false;
    // Start router if configured
    let router = if let Some(router_opts) = config.to_router_options() {
        logger.info(&format!(
            "Starting KNXnet/IP Router on IA: {}...",
            router_opts.individual_address,
        ));
        let r = Router::new(router_opts);
        if let Err(e) = r.connect_all().await {
            logger.error(&format!("Failed to start KNXnet/IP Router: {:?}", e));
            process::exit(1);
        }
        logger.info(&format!(
            "KNXnet/IP Router is running successfully with {} links.",
            r.link_count()
        ));
        has_service = true;
        Some(r)
    } else {
        None
    };

    // Start server if configured
    let server = if let Some(server_opts) = config.to_server_options() {
        logger.info(&format!(
            "Starting KNXnet/IP Server on {}:{} (IA: {}, friendly name: '{}')...",
            server_opts.local_ip,
            server_opts.port,
            server_opts.individual_address,
            server_opts.friendly_name
        ));
        let s = KnxNetIpServer::new(server_opts);
        if let Err(e) = s.connect().await {
            logger.error(&format!("Failed to start KNXnet/IP Server: {:?}", e));
            process::exit(1);
        }
        logger.info("KNXnet/IP Server is running successfully.");
        has_service = true;
        Some(s)
    } else {
        None
    };

    // Start client tunneling if configured
    let client = if let Some(client_opts) = config.to_tunneling_options() {
        logger.info(&format!(
            "Connecting to KNXnet/IP Gateway at {}:{}...",
            client_opts.ip, client_opts.port
        ));
        let c = KnxTunneling::new(client_opts);
        if let Err(e) = c.connect().await {
            logger.error(&format!("Failed to connect to KNXnet/IP Gateway: {:?}", e));
            process::exit(1);
        }
        logger.info("Connected to KNXnet/IP Gateway successfully.");
        has_service = true;
        Some(c)
    } else {
        None
    };

    // Expose WebSocket and MQTT APIs if configured
    let mut api_manager = None;
    if let Some(ref api_cfg) = config.api {
        let db_path = api_cfg
            .db_path
            .clone()
            .unwrap_or_else(|| "rknx_cache.db".to_string());
        match rknx::api::db::DbManager::new(&db_path) {
            Ok(db) => {
                let manager = rknx::api::manager::ApiManager::new(db);

                // WebSocket Server
                let ws_port = api_cfg.ws_port.unwrap_or(8080);
                let ws_server =
                    rknx::api::websocket::WebSocketServer::new(Arc::clone(&manager), ws_port);
                Arc::new(ws_server).start();

                // MQTT Client Adapter
                if let Some(ref mqtt_host) = api_cfg.mqtt_host {
                    let mqtt_port = api_cfg.mqtt_port.unwrap_or(1883);
                    let mqtt_client_id = api_cfg
                        .mqtt_client_id
                        .clone()
                        .unwrap_or_else(|| "rknx_daemon".to_string());
                    let mqtt_adapter = rknx::api::mqtt::MqttAdapter::new(
                        Arc::clone(&manager),
                        mqtt_host.clone(),
                        mqtt_port,
                        mqtt_client_id,
                    );
                    Arc::new(mqtt_adapter).start();
                }

                api_manager = Some(manager);
                has_service = true;
            }
            Err(e) => {
                logger.error(&format!("Failed to initialize database: {}", e));
            }
        }
    }

    if !has_service {
        logger.error("No server, client, router, or API services configured. Exiting.");
        process::exit(1);
    }

    logger.info("rKNX daemon is running. Press Ctrl+C to terminate.");
    tokio::signal::ctrl_c().await.ok();
    logger.info("Shutting down services...");

    if let Some(r) = router {
        r.disconnect_all().await;
    }
    if let Some(s) = server {
        let _ = s.disconnect().await;
    }
    if let Some(c) = client {
        let _ = c.disconnect().await;
    }
    if let Some(am) = api_manager {
        let _ = am.disconnect().await;
    }

    logger.info("Shutdown complete.");
}
