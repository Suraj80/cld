mod net;
mod protocol;

use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("listen") => {
            net::listener::listen(7799).await?;
        }

        Some("send") => {
            let address = args
                .get(2)
                .map(String::as_str)
                .unwrap_or("127.0.0.1:7799");

            let message = args
                .get(3)
                .map(String::as_str)
                .unwrap_or("hello from CLD");

            net::sender::send(address, message).await?;
        }

        _ => {
            println!("Usage:");
            println!("  cld listen");
            println!("  cld send <address> <message>");
        }
    }

    Ok(())
}