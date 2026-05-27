use crate::crypto::{
    generate_salt, key_debug_fingerprint, load_or_create_identity, parse_salt_base64, salt_base64,
    Role,
};
use crate::protocol::{read_message, write_message, WireMessage};
use anyhow::Result;
use chrono::Utc;
use tokio::net::TcpStream;
use uuid::Uuid;

pub async fn send(address: &str, username: &str, message: &str) -> Result<()> {
    let mut stream = TcpStream::connect(address).await?;

    let identity = load_or_create_identity()?;
    let my_salt = generate_salt();

    let handshake = WireMessage::Handshake {
        public_key: identity.public_key_base64(),
        username: username.to_string(),
        version: 1,
        session_salt: salt_base64(&my_salt),
    };

    write_message(&mut stream, &handshake).await?;

    let response = read_message(&mut stream).await?;

    let (peer_public_key, peer_salt) = match response {
        WireMessage::Handshake {
            public_key,
            session_salt,
            username,
            ..
        } => {
            println!("Handshake completed with {username}");
            (public_key, parse_salt_base64(&session_salt)?)
        }
        _ => anyhow::bail!("Expected handshake response"),
    };

    let session_keys =
        identity.derive_session_keys(&peer_public_key, &my_salt, &peer_salt, Role::Initiator)?;

    println!(
        "Derived send key fingerprint: {}",
        key_debug_fingerprint(&session_keys.send_key)
    );

    let wire_message = WireMessage::Text {
        id: Uuid::new_v4(),
        from: username.to_string(),
        content: message.to_string(),
        timestamp: Utc::now().timestamp(),
    };

    write_message(&mut stream, &wire_message).await?;

    println!("Message sent to {address}");

    Ok(())
}