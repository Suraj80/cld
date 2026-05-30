# CLD Getting Started

This guide keeps the detailed setup and usage steps out of the main README while documenting the full flow for running CLD.

## What CLD Does

CLD is a terminal-based peer-to-peer messenger. Each machine runs the same binary and talks directly to trusted peers over TCP, usually on a private network such as Tailscale or WireGuard.

## Installation

Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Clone and build:

```bash
git clone https://github.com/your-org/cld.git
cd cld
cargo build
```

## Main Commands

Initialize a profile:

```bash
cargo run -- init
```

Show your identity details:

```bash
cargo run -- identity
```

Start the terminal UI:

```bash
cargo run -- tui
```

List peers:

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

Reset a stored peer key:

```bash
cargo run -- reset-peer-key friend
```

Send a one-shot message:

```bash
cargo run -- send 127.0.0.1:7799 "hello from CLD"
```

Use a specific config profile:

```bash
cargo run -- --config alice.toml tui
```

## Adding a Friend

1. Run profile setup on both machines:

```bash
cargo run -- init
```

2. Print identity details on both machines:

```bash
cargo run -- identity
```

3. Exchange these details out-of-band:

- Tailscale IP
- CLD listen port
- Fingerprint from `cld identity`

4. Add each other as peers:

```bash
cargo run -- add-peer faiz 100.64.0.12:7799
```

5. Optionally pin the peer fingerprint in `config.toml`:

```toml
username = "suraj"
listen_port = 7799

[[peers]]
name = "faiz"
address = "100.64.0.12:7799"
expected_fingerprint = "peer-fingerprint-from-cld-identity"
```

6. Start the TUI on both machines:

```bash
cargo run -- tui
```

## Example Config

```toml
username = "suraj"
listen_port = 7799

[[peers]]
name = "friend"
address = "127.0.0.1:7800"
expected_fingerprint = ""
```

## More Details

For architecture, security notes, and project limitations, see [project-details.md](project-details.md).

## Development Checks

```bash
cargo fmt
cargo clippy
cargo test
cargo build
```
