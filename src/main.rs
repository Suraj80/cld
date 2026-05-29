mod config;
mod crypto;
mod db;
mod net;
mod protocol;
mod tui;
use anyhow::Result;
use std::env;
use std::path::Path;
use tokio::sync::mpsc;

fn load_app_config(config_override: Option<&String>) -> Result<config::Config> {
    if let Some(path) = config_override {
        config::load_config_from(Path::new(path))
    } else {
        config::load_or_create_config()
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let _conn = db::connect()?;
    let args: Vec<String> = env::args().collect();

    let mut config_override = None;
    let mut command_index = 1;

    if args.len() > 3 && args[1] == "--config" {
        config_override = Some(args[2].clone());
        command_index = 3;
    }

    match args.get(command_index).map(String::as_str) {
        Some("init") => {
            let config_path = config::config_path()?;
            let _config = config::load_or_create_config()?;

            let identity = crypto::load_or_create_identity()?;
            let identity_path = crypto::identity_key_path()?;

            println!("CLD profile initialized.");
            println!();
            println!("Config: {}", config_path.display());
            println!("Identity: {}", identity_path.display());
            println!();
            println!("Your public key:");
            println!("{}", identity.public_key_base64());
            println!();
            println!("Edit config.toml to update your username and peers.");
        }
        Some("tui") => {
            let config = load_app_config(config_override.as_ref())?;
            tui::ui::run_tui(config).await?;
        }

        Some("listen") => {
            let config = load_app_config(config_override.as_ref())?;
            let (tx, mut rx) = mpsc::unbounded_channel();

            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    println!("event: {event:?}");
                }
            });

            net::listener::listen(config.listen_port, config.username.clone(), tx).await?;
        }

        Some("send") => {
            let config = load_app_config(config_override.as_ref())?;

            let address = args.get(2).map(String::as_str).unwrap_or("127.0.0.1:7799");

            let message = args.get(3).map(String::as_str).unwrap_or("hello from CLD");

            net::sender::send(address, &config.username, "manual", None, message, 0).await?;
        }

        Some("peers") => {
            let config = load_app_config(config_override.as_ref())?;

            if config.peers.is_empty() {
                println!("No peers configured.");
            } else {
                println!("Configured peers:");

                for peer in config.peers {
                    println!("- {} ({})", peer.name, peer.address);
                }
            }
        }

        Some("add-peer") => {
            let name = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("Missing peer name"))?;

            let address = args
                .get(3)
                .ok_or_else(|| anyhow::anyhow!("Missing peer address"))?;

            let mut config = load_app_config(config_override.as_ref())?;

            config.peers.push(config::PeerConfig {
                name: name.to_string(),
                address: address.to_string(),
                expected_fingerprint: None,
            });

            config::save_config(&config)?;

            println!("Added peer: {name} ({address})");
        }

        Some("remove-peer") => {
            let name = args
                .get(2)
                .ok_or_else(|| anyhow::anyhow!("Missing peer name"))?;

            let mut config = load_app_config(config_override.as_ref())?;

            let before = config.peers.len();

            config.peers.retain(|peer| peer.name != *name);

            if config.peers.len() == before {
                println!("Peer not found: {name}");
            } else {
                config::save_config(&config)?;
                println!("Removed peer: {name}");
            }
        }

        Some("reset-peer-key") => {
            let name = args
                .get(command_index + 1)
                .ok_or_else(|| anyhow::anyhow!("Missing peer name"))?;

            let conn = db::connect()?;
            db::reset_peer_key(&conn, name)?;

            println!("Reset stored key for peer: {name}");
        }

        _ => {
            println!("Usage:");
            println!("  cld init");
            println!("  cld tui");
            println!("  cld listen");
            println!("  cld peers");
            println!("  cld add-peer <name> <address>");
            println!("  cld remove-peer <name>");
            println!("  cld reset-peer-key <name>");
            println!("  cld send <address> <message>");
        }
    }

    Ok(())
}
