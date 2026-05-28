use crate::crypto::build_nonce;
use crate::crypto::encrypt_payload;
use crate::crypto::{
    Role, generate_salt, load_or_create_identity, parse_salt_base64, public_key_fingerprint_base64,
    salt_base64,
};
use crate::protocol::{WireMessage, read_message, write_message};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;
use tokio::net::TcpStream;
use uuid::Uuid;

pub async fn send(address: &str, username: &str, message: &str) -> Result<()> {
    if message.len() > 4096 {
        anyhow::bail!("Message too large. Max allowed size is 4096 bytes.");
    }

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
            username: _,
            ..
        } => (public_key, parse_salt_base64(&session_salt)?),
        _ => anyhow::bail!("Expected handshake response"),
    };

    let peer_fingerprint = public_key_fingerprint_base64(&peer_public_key)?;

    let conn = crate::db::connect()?;

    let verified =
        crate::db::verify_or_store_peer_fingerprint(&conn, "listener", &peer_fingerprint)?;

    if !verified {
        anyhow::bail!("SECURITY WARNING: listener key mismatch. Connection rejected.");
    }

    let session_keys =
        identity.derive_session_keys(&peer_public_key, &my_salt, &peer_salt, Role::Initiator)?;

    let plain_message = WireMessage::Text {
        id: Uuid::new_v4(),
        from: username.to_string(),
        content: message.to_string(),
        timestamp: Utc::now().timestamp(),
    };

    let plaintext = serde_json::to_vec(&plain_message)?;

    let seq = 0u64;
    let nonce = build_nonce(&my_salt, seq);

    let ciphertext = encrypt_payload(&session_keys.send_key, &nonce, &plaintext)?;

    let encrypted_message = WireMessage::Encrypted {
        seq,
        nonce: general_purpose::STANDARD.encode(nonce),
        ciphertext: general_purpose::STANDARD.encode(ciphertext),
    };

    write_message(&mut stream, &encrypted_message).await?;

    match read_message(&mut stream).await? {
        WireMessage::Ack { seq: ack_seq } if ack_seq == seq => {
            // delivered
        }
        other => {
            anyhow::bail!("Expected ACK, got: {other:?}");
        }
    }

    Ok(())
}
