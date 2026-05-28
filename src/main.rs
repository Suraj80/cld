mod config;
mod crypto;
mod db;
mod net;
mod protocol;
mod tui;
use anyhow::Result;
use std::env;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<()> {
    let _conn = db::connect()?;
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
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
            tui::ui::run_tui().await?;
        }

        Some("listen") => {
            let config = config::load_or_create_config()?;
            let (tx, mut rx) = mpsc::unbounded_channel();

            tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    println!("event: {event:?}");
                }
            });

            net::listener::listen(config.listen_port, tx).await?;
        }

        Some("send") => {
            let config = config::load_or_create_config()?;

            let address = args.get(2).map(String::as_str).unwrap_or("127.0.0.1:7799");

            let message = args.get(3).map(String::as_str).unwrap_or("hello from CLD");

            net::sender::send(address, &config.username, message).await?;
        }

        Some("peers") => {
            let config = config::load_or_create_config()?;

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

            let mut config = config::load_or_create_config()?;

            config.peers.push(config::PeerConfig {
                name: name.to_string(),
                address: address.to_string(),
                expected_fingerprint: None,
            });

            config::save_config(&config)?;

            println!("Added peer: {name} ({address})");
        }

        _ => {
            println!("Usage:");
            println!("  cld init");
            println!("  cld tui");
            println!("  cld listen");
            println!("  cld peers");
            println!("  cld add-peer <name> <address>");
            println!("  cld send <address> <message>");
        }
    }

    Ok(())
}
