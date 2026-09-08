# multiplayer-fps - Maze Wars

A native Rust recreation of Maze Wars for the 01-edu `multiplayer-fps` subject. Multiplayer uses a central authoritative UDP server and a Macroquad desktop client.

## Features

- First-person Maze Wars-style renderer with eye-shaped opponents.
- Full-world minimap with live position and facing direction.
- On-screen FPS and frame-time display.
- Real UDP client/server networking.
- Server accepts up to 64 network clients.
- GUI connection screen for server IP, username and saved host aliases.
- Three built-in levels with strictly increasing dead-end counts.
- Server-authoritative movement, collision, shooting, damage, score, death and respawn.
- Client prediction and remote-player interpolation.
- Mouse look, left-click/Space fire, reload and weapon HUD.
- Scoreboard, settings and multiple visual themes.
- Native level editor and procedural maze generation.
- Server-side AI players with maze pathfinding and line-of-sight combat.

## Requirements

- Rust 1.74+ (edition 2021).
- Desktop environment supported by Macroquad.

## Run

Start the server:

```bash
cargo run --release --bin server
```

Default bind:

```text
0.0.0.0:34254
```

Optional:

```bash
cargo run --release --bin server -- --bind 0.0.0.0:34254 --bots 3 --level 1
```

Start each client:

```bash
cargo run --release --bin client
```

or:

```bash
cargo run --release
```

The connection screen accepts either a bare IP such as `127.0.0.1` or an explicit endpoint such as `127.0.0.1:34254`.

Duplicate display names are supported; network identity uses server-assigned player IDs.

## Controls

| Input | Action |
| --- | --- |
| `W / S` | Move forward / backward |
| `A / D` | Strafe |
| Mouse | Look |
| `Q / E` or arrow keys | Keyboard turning |
| Left mouse / `Space` | Shoot |
| `R` | Reload |
| `Tab` | Scoreboard |
| `Esc` | Release mouse / return to connection screen |

Use the top bar for built-in levels, procedural generation, gateway, editor, scoreboard and settings.

## Level editor

Open **LEVEL EDITOR** from the connection screen.

- Left mouse: toggle a wall cell.
- `G`: generate a procedural maze.
- Size control: cycle maze sizes.
- `S`: save `custom_level.json`.
- `L`: load `custom_level.json`.
- `Esc`: return.

## Multiplayer test

For a local multi-client run:

```bash
cargo run --release --bin server -- --bots 0
```

Launch 10+ clients and connect them to:

```text
127.0.0.1:34254
```

For LAN play, keep the server on `0.0.0.0:34254` and connect clients to the server machine's LAN IP.

## Checks

```bash
cargo fmt --all -- --check
cargo check --all-targets --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --bins --locked
```

CI also compiles all targets on Rust 1.74. The UDP integration test starts 10 independent clients with the same display name, validating connection capacity and server-assigned identity. Level tests verify strictly increasing dead-end counts.

## Architecture

```text
src/
├── lib.rs
├── maze.rs         maze generation, collision, ray casting, BFS
├── protocol.rs     UDP packet encoding/decoding
└── bin/
    ├── server.rs   authoritative 60 Hz UDP server + AI
    └── client.rs   native Macroquad UDP client + UI/rendering/editor
```

See `PROTOCOL.md` for the UDP wire format.
