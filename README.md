# CLD

> Secure terminal-based peer-to-peer messaging built in Rust.

[![Build](https://img.shields.io/badge/build-passing-placeholder.svg)](#testing)
[![Rust](https://img.shields.io/badge/rust-2024-placeholder.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-placeholder.svg)](#license)
[![Status](https://img.shields.io/badge/status-experimental-orange.svg)](#security-notes)

CLD is a local-first, terminal-native messaging system for direct communication between trusted peers. It combines async Rust networking, authenticated encryption, SQLite persistence, and an event-driven TUI into a compact systems project.

The project is designed to explore the pieces behind a secure decentralized messenger: framing, identity keys, session derivation, replay resistance, delivery acknowledgements, local persistence, and responsive terminal UI state management.

## Overview

Most messaging systems route traffic through centralized infrastructure. That architecture is convenient, but it also means message delivery, metadata, storage, and availability depend on a third-party service.

CLD takes a different shape: each device runs the same binary and communicates directly with configured peers over TCP, typically on a private network such as Tailscale or WireGuard. There is no account system, no cloud routing layer, and no central message broker in the communication path.

The project was built as a production-style systems programming exercise with a strong focus on:

- Secure decentralized communication between trusted machines.
- Explicit protocol design instead of ad-hoc socket reads.
- Modern cryptographic building blocks with clear module boundaries.
- Local-first persistence using SQLite.
- A keyboard-driven terminal interface for real-time messaging.
- Async Rust architecture using Tokio tasks and channels.

## Why CLD?

CLD is for studying and demonstrating how a secure peer-to-peer messaging stack fits together from the transport layer up to the UI.

It is intentionally small enough to inspect end-to-end, but structured like a real open-source systems project: cryptography, protocol framing, networking, persistence, configuration, and UI are separated into focused modules.

CLD is useful if you care about:

- Understanding encrypted application protocols beyond library-level usage.
- Building terminal applications with real asynchronous event flow.
- Designing local-first applications without a central server.
- Exploring Rust for networking, persistence, and TUI development.
- Reviewing a portfolio project with meaningful systems engineering tradeoffs.

## Features

- End-to-end encrypted peer-to-peer messaging.
- X25519 key exchange for peer session establishment.
- HKDF-SHA256 session key derivation.
- Directional send and receive keys for each session.
- ChaCha20Poly1305 authenticated encryption.
- Monotonic nonce construction using session salt and counters.
- ACK-based delivery confirmation.
- Replay protection using sequence numbers.
- Length-prefixed encrypted wire protocol with frame size limits.
- Async networking powered by Tokio.
- TCP listener and sender networking layers.
- SQLite-based persistent chat history.
- Event-driven TUI architecture using channels.
- Ratatui terminal interface.
- Multi-peer configuration support.
- Profile initialization system.
- Peer management commands.
- Multi-profile config support via `--config`.
- Structured message model for wire and UI events.

## Architecture

CLD separates user interaction, network transport, cryptography, persistence, and rendering into distinct layers.

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

The TUI owns interactive state such as the selected contact, input buffer, and visible messages. The networking layer runs independently, accepting inbound connections and sending outbound messages without coupling itself to rendering logic.

Tokio is used because CLD needs to handle terminal interaction, TCP listeners, peer connections, and message delivery without blocking the whole process. Channels bridge the async networking tasks and the UI event loop, keeping the architecture event-driven rather than tightly synchronous.

This separation keeps the system easier to reason about:

- The TUI renders state and translates keyboard input into actions.
- The sender opens outbound encrypted sessions and waits for ACKs.
- The listener accepts inbound sessions, verifies peer identity, decrypts messages, and emits UI events.
- The protocol module owns frame encoding, decoding, and message structure.
- The database module persists chat history and peer key fingerprints.

## Cryptography

CLD uses modern cryptographic primitives to protect message contents after the handshake phase.

### Handshake

Each CLD profile has a long-lived X25519 identity keypair. During connection setup, peers exchange public keys and session salts. Both sides compute a shared secret using X25519 Diffie-Hellman.

### Key Derivation

The shared secret is passed through HKDF-SHA256 to derive session keys. CLD derives separate directional keys for each connection:

- One key for messages sent from the initiator to the listener.
- One key for messages sent from the listener to the initiator.

Directional keys avoid accidental key/nonce reuse between peers that may both start counters from zero.

### Encryption

Message payloads are encrypted with ChaCha20Poly1305, which provides authenticated encryption. This protects confidentiality and detects tampering before plaintext is accepted by the application.

### Nonces

CLD constructs nonces from session-specific salt material and a monotonic counter. The design avoids random nonce generation for encrypted messages and keeps nonce state deterministic within a session.

### Replay Protection

Encrypted messages carry sequence numbers. The receiver tracks accepted sequence state and rejects stale or duplicate messages. Large sequence gaps can be treated as suspicious and cause the connection to be dropped.

### ACK Protocol

After a message is decrypted and accepted, the receiver returns an ACK containing the accepted sequence number. The sender uses this as delivery confirmation for the encrypted frame.

### Security Notes

This project is educational and has not undergone professional security auditing.

Do not use CLD as-is for high-risk communications. The code is intended to demonstrate protocol design and secure systems concepts, not to replace audited messengers such as Signal.

Important security considerations:

- TOFU protects against key changes after first trust, but cannot prevent first-connection MITM without out-of-band fingerprint verification.
- Forward secrecy is not complete while long-lived identity keys are used directly for session establishment.
- Local database contents are not currently encrypted at rest.
- Endpoint compromise is out of scope.
- Traffic metadata such as timing, peer IPs, and approximate message sizes may still be observable.

## Project Structure

```text
src/
  main.rs              CLI entrypoint and command dispatch
  config.rs            TOML configuration loading, saving, and profile support
  crypto.rs            Identity keys, X25519, HKDF, nonces, and encryption helpers
  protocol.rs          Wire message model and length-prefixed frame codec
  db.rs                SQLite schema, message persistence, and TOFU key storage
  net/
    mod.rs             Networking module exports
    listener.rs        TCP listener, inbound handshake, decryption, ACK handling
    sender.rs          Outbound connection, encryption, send flow, ACK validation
    ratelimit.rs       Token-bucket rate limiter
  tui/
    mod.rs             TUI module exports
    ui.rs              Ratatui event loop, rendering, and user input handling
    state.rs           UI application state
    message.rs         Chat message types and delivery status
    events.rs          Events passed from networking to the TUI
```

Additional project files:

```text
Cargo.toml             Rust package metadata and dependencies
prd.md                 Product requirements and protocol notes
suraj.toml             Example local profile configuration
```

## Installation

Install Rust using `rustup`:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Clone and build the project:

```bash
git clone https://github.com/your-org/cld.git
cd cld
cargo build
```

Run the binary through Cargo:

```bash
cargo run -- --help
```

For a release build:

```bash
cargo build --release
```

## Usage

Initialize a CLD profile:

```bash
cargo run -- init
```

Start the terminal UI:

```bash
cargo run -- tui
```

List configured peers:

```bash
cargo run -- peers
```

Add a peer:

```bash
cargo run -- add-peer friend 127.0.0.1:7800
```

Remove a peer:

```bash
cargo run -- remove-peer friend
```

Send a one-shot message:

```bash
cargo run -- send 127.0.0.1:7799 "hello from CLD"
```

Run with a specific config profile:

```bash
cargo run -- --config suraj.toml tui
```

Run two local profiles for testing:

```bash
cargo run -- --config suraj.toml tui
cargo run -- --config friend.toml tui
```

Get your identity details for sharing with a new peer:

```bash
cargo run -- identity
```

Example config:

```toml
username = "suraj"
listen_port = 7799

[[peers]]
name = "friend"
address = "127.0.0.1:7800"
expected_fingerprint = ""
```

## Adding a Friend

Here is the simplest end-to-end setup flow for two people who want to talk over a private network such as Tailscale.

1. Both people initialize CLD on their own machine:

```bash
cargo run -- init
```

2. Both people print their local identity details:

```bash
cargo run -- identity
```

3. Exchange these details out-of-band:

- Your Tailscale IP and CLD listen port.
- Your fingerprint from `cld identity`.

4. Add each other as peers using the shared address:

```bash
cargo run -- add-peer faiz 100.64.0.12:7799
```

5. Open `config.toml` and set the peer's `expected_fingerprint` if you want explicit first-connection verification instead of relying only on TOFU:

```toml
[[peers]]
name = "faiz"
address = "100.64.0.12:7799"
expected_fingerprint = "peer-fingerprint-from-cld-identity"
```

6. Start the TUI on both machines:

```bash
cargo run -- tui
```

At that point, each person should see the other in the contacts list and can begin chatting directly over the private network.


## Testing

Run the test suite:

```bash
cargo test
```

Run Clippy:

```bash
cargo clippy
```

Build the project:

```bash
cargo build
```

Recommended checks before opening a pull request:

```bash
cargo fmt
cargo clippy
cargo test
cargo build
```

## Technical Highlights

- Async Rust: Tokio powers the listener, outbound sender flow, and event-oriented task structure.
- Systems programming: CLD works directly with TCP streams, frame boundaries, local files, and SQLite.
- Protocol design: Messages use explicit wire enums and length-prefixed frames.
- Event-driven architecture: Networking emits events into the TUI instead of rendering directly.
- State management: UI state tracks contacts, selected peer, message history, input, and delivery status.
- Encrypted transport: X25519, HKDF, ChaCha20Poly1305, nonces, sequence numbers, and ACKs form the secure messaging path.

## Known Limitations

- The project is experimental and not security audited.
- Forward secrecy is not yet equivalent to Signal-style session ratcheting.
- NAT traversal is not implemented; CLD expects reachable peers, usually over a private network.
- Offline message queues are not implemented.
- Database encryption at rest is not implemented.
- Group messaging is not implemented.
- File transfer is not implemented.
- The TUI is still evolving and may not yet expose every protocol/security state visually.

## Development Roadmap

- Group chats.
- File transfer with progress reporting.
- Message search.
- Markdown rendering in the terminal UI.
- NAT traversal.
- Optional relay servers for unreachable peers.
- Signal-style double ratchet sessions.
- Encrypted local database.
- Mobile client.
- Better presence and reconnect behavior.
- Richer protocol test coverage and fuzzing.

## Contributing

Contributions are welcome. CLD is especially well suited for contributors interested in Rust networking, terminal UI design, cryptography-adjacent engineering, SQLite-backed applications, and protocol testing.

Suggested contribution flow:

1. Open an issue describing the bug, improvement, or design question.
2. Keep changes focused and easy to review.
3. Add tests for protocol, crypto, database, or state-management changes where practical.
4. Run `cargo fmt`, `cargo clippy`, and `cargo test` before submitting.
5. Open a pull request with a clear description of behavior changes and tradeoffs.

Security-related changes should include a short explanation of the threat model impact.

## License

MIT License placeholder.

Copyright (c) 2026 CLD contributors.

See `LICENSE` for details once the license file is added.
