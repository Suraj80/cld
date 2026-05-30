# CLD Project Details

This document covers the broader design and technical details of CLD beyond the basic setup flow.

## Overview

CLD is a terminal-based peer-to-peer messenger built for direct communication between trusted peers. It focuses on local-first communication, explicit protocol design, and a small but production-style Rust codebase.

## Architecture

```text
User Input
    |
    v
TUI Event System
    |
    v
Sender / Listener Networking Layer
    |
    v
Encrypted Wire Protocol
    |
    v
SQLite Persistence
    |
    v
UI Rendering
```

### Main Layers

- The TUI handles keyboard input, contact selection, and rendering visible chat state.
- The networking layer manages outbound connections, inbound listeners, encrypted sessions, and acknowledgements.
- The protocol layer defines the wire message model and framing rules.
- The database layer stores messages and remembered peer fingerprints.
- The crypto layer handles identity keys, session derivation, fingerprints, and payload encryption.

## Crypto Design

- X25519 is used for peer key agreement.
- Session keys are derived with HKDF-SHA256.
- CLD creates directional send/receive keys for each session.
- Messages are encrypted with ChaCha20Poly1305.
- Nonces are built from session salt plus a monotonic counter.
- Replay protection uses message sequence tracking.

## Peer Verification

- CLD supports explicit fingerprint verification through `expected_fingerprint`.
- If no expected fingerprint is configured, CLD falls back to TOFU.
- TOFU protects against later key changes, but not first-connection MITM by itself.

## Persistence

- Chat history is stored locally in SQLite.
- Stored peer fingerprints are used to detect unexpected key changes.
- Messages received while another chat is selected remain buffered per peer in the UI state.

## Project Structure

```text
src/
  main.rs              CLI entrypoint and command dispatch
  config.rs            TOML configuration loading and saving
  crypto.rs            Identity keys, session keys, nonces, encryption
  protocol.rs          Wire message model and framing
  db.rs                SQLite schema, chat persistence, peer fingerprints
  net/
    listener.rs        Inbound handshake, decrypt, ACK handling
    sender.rs          Outbound connection and send flow
    ratelimit.rs       Token-bucket rate limiting
  tui/
    ui.rs              Event loop and rendering
    state.rs           Per-peer UI state
    message.rs         Chat message model
    events.rs          Events sent into the UI
```

## Known Limitations

- No NAT traversal
- No offline message queue
- No encrypted local database
- No group messaging
- No file transfer
- No formal security audit

## Development Notes

- The project uses Tokio for async networking and task coordination.
- Ratatui powers the terminal interface.
- SQLite keeps the project simple and local-first.
- The codebase is intentionally small enough to inspect end-to-end.
