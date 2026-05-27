mod net;
mod protocol;
mod config;

use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("listen") => {
            let config = config::load_or_create_config()?;
            net::listener::listen(config.listen_port).await?;
        }

        Some("send") => {
            let config = config::load_or_create_config()?;

            let address = args
                .get(2)
                .map(String::as_str)
                .unwrap_or("127.0.0.1:7799");

            let message = args
                .get(3)
                .map(String::as_str)
                .unwrap_or("hello from CLD");

            net::sender::send(address, &config.username, message).await?;
        }

        _ => {
            println!("Usage:");
            println!("  cld listen");
            println!("  cld send <address> <message>");
        }
    }

    Ok(())
}