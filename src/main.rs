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

        _ => {
            println!("Usage:");
            println!("  cld tui");
            println!("  cld listen");
            println!("  cld send <address> <message>");
        }
    }

    Ok(())
}
