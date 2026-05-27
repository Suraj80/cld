use crate::crypto::{
    generate_salt, key_debug_fingerprint, load_or_create_identity, parse_salt_base64, salt_base64,
    Role,
};
use crate::protocol::{read_message, write_message, WireMessage};
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
            let identity = match load_or_create_identity() {
                Ok(identity) => identity,
                Err(error) => {
                    eprintln!("Failed to load identity: {error}");
                    return;
                }
            };

            let my_salt = generate_salt();

            let incoming = match read_message(&mut socket).await {
                Ok(message) => message,
                Err(error) => {
                    eprintln!("Failed to read handshake: {error}");
                    return;
                }
            };

            let (peer_public_key, peer_salt, peer_username) = match incoming {
                WireMessage::Handshake {
                    public_key,
                    username,
                    session_salt,
                    ..
                } => {
                    let salt = match parse_salt_base64(&session_salt) {
                        Ok(salt) => salt,
                        Err(error) => {
                            eprintln!("Invalid peer salt: {error}");
                            return;
                        }
                    };

                    (public_key, salt, username)
                }
                _ => {
                    eprintln!("Expected handshake");
                    return;
                }
            };

            let response = WireMessage::Handshake {
                public_key: identity.public_key_base64(),
                username: "listener".to_string(),
                version: 1,
                session_salt: salt_base64(&my_salt),
            };

            if let Err(error) = write_message(&mut socket, &response).await {
                eprintln!("Failed to send handshake response: {error}");
                return;
            }

            let session_keys = match identity.derive_session_keys(
                &peer_public_key,
                &my_salt,
                &peer_salt,
                Role::Listener,
            ) {
                Ok(keys) => keys,
                Err(error) => {
                    eprintln!("Failed to derive session keys: {error}");
                    return;
                }
            };

            println!("Handshake completed with {peer_username}");
            println!(
                "Derived recv key fingerprint: {}",
                key_debug_fingerprint(&session_keys.recv_key)
            );

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

                    if let Ok(conn) = crate::db::connect() {
                        if let Err(error) =
                            crate::db::insert_message(&conn, id, &from, "in", &content, timestamp)
                        {
                            eprintln!("Failed to save message: {error}");
                        }
                    }
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