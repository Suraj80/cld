use crate::crypto::{
    Role, decrypt_payload, generate_salt, load_or_create_identity, parse_salt_base64,
    public_key_fingerprint_base64, salt_base64,
};
use crate::net::ratelimit::RateLimiter;
use crate::protocol::{WireMessage, read_message, write_message};
use crate::tui::events::ChatEvent;
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc::UnboundedSender;

pub async fn listen(port: u16, username: String, tx: UnboundedSender<ChatEvent>) -> Result<()> {
    let address = format!("0.0.0.0:{port}");
    let listener = TcpListener::bind(&address).await?;

    loop {
        let (mut socket, _peer_addr) = listener.accept().await?;

        let tx_for_connection = tx.clone();
        let username_for_connection = username.clone();

        tokio::spawn(async move {
            let tx = tx_for_connection;
            let username = username_for_connection;

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

            let peer_fingerprint = match public_key_fingerprint_base64(&peer_public_key) {
                Ok(value) => value,
                Err(error) => {
                    eprintln!("Failed to compute peer fingerprint: {error}");
                    return;
                }
            };

            let conn = match crate::db::connect() {
                Ok(conn) => conn,
                Err(error) => {
                    eprintln!("Failed to open database: {error}");
                    return;
                }
            };

            match crate::db::verify_or_store_peer_fingerprint(
                &conn,
                &peer_username,
                &peer_fingerprint,
            ) {
                Ok(true) => {}
                Ok(false) => {
                    eprintln!("SECURITY WARNING: peer key mismatch for {peer_username}");
                    eprintln!("Connection rejected.");
                    return;
                }
                Err(error) => {
                    eprintln!("Failed to verify peer fingerprint: {error}");
                    return;
                }
            }

            let response = WireMessage::Handshake {
                public_key: identity.public_key_base64(),
                username: username.clone(),
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

            let mut last_seq: Option<u64> = None;
            let mut rate_limiter = RateLimiter::new(60, Duration::from_secs(1));

            loop {
                match read_message(&mut socket).await {
                    Ok(WireMessage::Encrypted {
                        seq,
                        nonce,
                        ciphertext,
                    }) => {
                        if !rate_limiter.allow() {
                            let _ = tx
                                .send(ChatEvent::SystemMessage("rate limit exceeded".to_string()));
                            break;
                        }

                        if let Some(last) = last_seq {
                            if seq <= last {
                                let _ = tx.send(ChatEvent::SystemMessage(
                                    "replay detected; message dropped".to_string(),
                                ));
                                break;
                            }

                            if seq > last + 100 {
                                let _ = tx.send(ChatEvent::SystemMessage(
                                    "sequence gap too large; connection dropped".to_string(),
                                ));
                                break;
                            }
                        }

                        last_seq = Some(seq);

                        let nonce_bytes = match general_purpose::STANDARD.decode(nonce) {
                            Ok(bytes) => bytes,
                            Err(error) => {
                                let _ = tx.send(ChatEvent::SystemMessage(format!(
                                    "invalid nonce encoding: {error}"
                                )));
                                break;
                            }
                        };

                        let nonce_array: [u8; 12] = match nonce_bytes.try_into() {
                            Ok(value) => value,
                            Err(_) => {
                                let _ = tx.send(ChatEvent::SystemMessage(
                                    "invalid nonce length".to_string(),
                                ));
                                break;
                            }
                        };

                        let ciphertext_bytes = match general_purpose::STANDARD.decode(ciphertext) {
                            Ok(bytes) => bytes,
                            Err(error) => {
                                let _ = tx.send(ChatEvent::SystemMessage(format!(
                                    "invalid ciphertext encoding: {error}"
                                )));
                                break;
                            }
                        };

                        let plaintext = match decrypt_payload(
                            &session_keys.recv_key,
                            &nonce_array,
                            &ciphertext_bytes,
                        ) {
                            Ok(value) => value,
                            Err(error) => {
                                let _ = tx.send(ChatEvent::SystemMessage(format!(
                                    "failed to decrypt message: {error}"
                                )));
                                break;
                            }
                        };

                        let inner_message = match serde_json::from_slice::<WireMessage>(&plaintext)
                        {
                            Ok(message) => message,
                            Err(error) => {
                                let _ = tx.send(ChatEvent::SystemMessage(format!(
                                    "failed to deserialize decrypted message: {error}"
                                )));
                                break;
                            }
                        };

                        match inner_message {
                            WireMessage::Text {
                                id,
                                from,
                                content,
                                timestamp,
                            } => {
                                if content.len() > 4096 {
                                    let _ = tx.send(ChatEvent::SystemMessage(
                                        "message rejected: content too large".to_string(),
                                    ));
                                    break;
                                }

                                let _ = tx.send(ChatEvent::IncomingMessage {
                                    from: from.clone(),
                                    content: content.clone(),
                                });

                                if let Ok(conn) = crate::db::connect() {
                                    if let Err(error) = crate::db::insert_message(
                                        &conn, id, &from, "in", &content, timestamp,
                                    ) {
                                        let _ = tx.send(ChatEvent::SystemMessage(format!(
                                            "failed to save message: {error}"
                                        )));
                                    }
                                }

                                let ack = WireMessage::Ack { seq };

                                if let Err(error) = write_message(&mut socket, &ack).await {
                                    let _ = tx.send(ChatEvent::SystemMessage(format!(
                                        "failed to send ACK: {error}"
                                    )));
                                    break;
                                }
                            }

                            other => {
                                let _ = tx.send(ChatEvent::SystemMessage(format!(
                                    "unexpected decrypted message: {other:?}"
                                )));
                                break;
                            }
                        }
                    }

                    Ok(other) => {
                        let _ = tx.send(ChatEvent::SystemMessage(format!(
                            "unexpected message: {other:?}"
                        )));
                        break;
                    }

                    Err(_) => {
                        break;
                    }
                }
            }
        });
    }
}
