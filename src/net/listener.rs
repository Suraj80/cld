use crate::crypto::{
    Role, decrypt_payload, generate_salt, key_debug_fingerprint, load_or_create_identity,
    parse_salt_base64, salt_base64,
};
use crate::protocol::{WireMessage, read_message, write_message};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
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
                Ok(WireMessage::Encrypted {
                    seq,
                    nonce,
                    ciphertext,
                }) => {
                    println!("Received encrypted message with seq: {seq}");

                    let nonce_bytes = match general_purpose::STANDARD.decode(nonce) {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            eprintln!("Invalid nonce encoding: {error}");
                            return;
                        }
                    };

                    let nonce_array: [u8; 12] = match nonce_bytes.try_into() {
                        Ok(value) => value,
                        Err(_) => {
                            eprintln!("Invalid nonce length");
                            return;
                        }
                    };

                    let ciphertext_bytes = match general_purpose::STANDARD.decode(ciphertext) {
                        Ok(bytes) => bytes,
                        Err(error) => {
                            eprintln!("Invalid ciphertext encoding: {error}");
                            return;
                        }
                    };

                    let plaintext = match decrypt_payload(
                        &session_keys.recv_key,
                        &nonce_array,
                        &ciphertext_bytes,
                    ) {
                        Ok(value) => value,
                        Err(error) => {
                            eprintln!("Failed to decrypt message: {error}");
                            return;
                        }
                    };

                    let inner_message = match serde_json::from_slice::<WireMessage>(&plaintext) {
                        Ok(message) => message,
                        Err(error) => {
                            eprintln!("Failed to deserialize decrypted message: {error}");
                            return;
                        }
                    };

                    match inner_message {
                        WireMessage::Text {
                            id,
                            from,
                            content,
                            timestamp,
                        } => {
                            println!("Decrypted message:");
                            println!("Message ID: {id}");
                            println!("From: {from}");
                            println!("Content: {content}");
                            println!("Timestamp: {timestamp}");

                            if let Ok(conn) = crate::db::connect() {
                                if let Err(error) = crate::db::insert_message(
                                    &conn, id, &from, "in", &content, timestamp,
                                ) {
                                    eprintln!("Failed to save message: {error}");
                                }
                            }
                        }

                        other => {
                            println!("Decrypted non-text message: {other:?}");
                        }
                    }
                }

                Ok(other) => {
                    println!("Expected encrypted message, got: {other:?}");
                }

                Err(error) => {
                    eprintln!("Failed to read encrypted message: {error}");
                }
            }
        });
    }
}
