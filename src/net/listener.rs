use crate::protocol::{read_message, WireMessage};
use anyhow::Result;
use tokio::net::TcpListener;

pub async fn listen(port: u16) -> Result<()> {
    let address = format!("127.0.0.1:{port}");
    let listener = TcpListener::bind(&address).await?;

    println!("CLD listening on {address}");

    loop {
        let (mut socket, peer_addr) = listener.accept().await?;
        println!("Connection from {peer_addr}");

        tokio::spawn(async move {
            match read_message(&mut socket).await {
                Ok(WireMessage::Text {
                    id,
                    from,
                    content,
                    timestamp,
                }) => {
                    println!("Message ID: {id}");
                    println!("From: {from}");
                    println!("Content: {content}");
                    println!("Timestamp: {timestamp}");
                }

                Ok(other) => {
                    println!("Received non-text message: {other:?}");
                }

                Err(error) => {
                    eprintln!("Failed to read message: {error}");
                }
            }
        });
    }
}