use crate::protocol::{write_message, WireMessage};
use anyhow::Result;
use chrono::Utc;
use tokio::net::TcpStream;
use uuid::Uuid;

pub async fn send(address: &str, username: &str, message: &str) -> Result<()> {
    let mut stream = TcpStream::connect(address).await?;

    let wire_message = WireMessage::Text {
        id: Uuid::new_v4(),
        from: username.to_string(),
        content: message.to_string(),
        timestamp: Utc::now().timestamp(),
    };

    write_message(&mut stream, &wire_message).await?;

    println!("JSON message sent to {address}");

    Ok(())
}