# multiplayer-fps - Maze Wars

A native Rust recreation of the classic Maze Wars concept for the 01-edu `multiplayer-fps` subject.

The project intentionally uses a real UDP client/server architecture. The graphical client is native (Macroquad), so there is no browser WebSocket bridge between the player and the game server.

## Features

### Required scope

- Native first-person maze renderer inspired by Maze Wars.
- Full-world minimap with live player position and facing direction.
- On-screen FPS counter.
- UDP client/server architecture.
- Server binds to `0.0.0.0:34254` by default and supports up to 64 network clients.
- GUI asks for server IP/port and username before joining.
- Three deterministic levels with increasing dimensions and a guaranteed increasing number of dead ends.
- Server-authoritative movement, wall collision, shooting, damage, score, death and respawn.
- Client-side movement prediction plus snapshot correction/interpolation for smooth play.
- 60 Hz server simulation and 30 Hz world snapshots.

### Bonuses

- **Level editor:** edit wall cells visually and save/load `custom_level.json`.
- **Procedural generation:** randomized depth-first-search / recursive-backtracker maze generator.
- **AI players:** server-side eye bots use line-of-sight combat and BFS maze pathfinding.
- **GUI host history:** successful hosts are remembered in `hosts.json` with an optional alias and username.

## Requirements

- Rust stable (edition 2021).
- A desktop environment supported by Macroquad.

## Run

### 1. Start the UDP server

```bash
cargo run --release --bin server
```

Default server address:

```text
0.0.0.0:34254
```

Optional server flags:

```bash
cargo run --release --bin server -- --bind 0.0.0.0:34254 --bots 3 --level 1
```

- `--bind`: UDP bind address.
- `--bots`: number of AI players, `0..32` (default `3`).
- `--level`: initial level, `1..3`.

### 2. Start a client

```bash
cargo run --release --bin client
```

Or simply:

```bash
cargo run --release
```

The client opens the graphical connection screen. Enter:

- server address, for example `127.0.0.1:34254`;
- username;
- optional host alias.

For another computer on the LAN, enter the server machine's LAN address, for example `192.168.1.25:34254`.

## Controls

| Key | Action |
| --- | --- |
| `W / S` | Move forward / backward |
| `A / D` | Strafe left / right |
| `Q / E` or `← / →` | Turn |
| `Space` | Shoot |
| `N` | Ask the server to switch to the next level |
| `Esc` | Disconnect |

## Level editor

Open **LEVEL EDITOR** from the connection screen.

- Left mouse: toggle a wall cell.
- `G`: generate a new procedural maze.
- Size button: cycle through 15×15, 21×21 and 27×27.
- `S`: save `custom_level.json`.
- `L`: load `custom_level.json`.
- `Esc`: return to the connection screen.

## Multiplayer / audit setup

The audit permits multiple local clients if several physical machines cannot communicate. A simple local test is:

```bash
cargo run --release --bin server -- --bots 0
```

Then launch 10+ client processes and connect all of them to:

```text
127.0.0.1:34254
```

For a LAN test, keep the default `0.0.0.0:34254` server bind and connect clients to the server machine's LAN IP.

## Tests and checks

```bash
cargo test
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
```

The level tests also verify that the three built-in mazes have strictly increasing dead-end counts.

## Architecture

```text
src/
├── lib.rs          shared crate
├── maze.rs         maze generation, collision, ray casting, BFS
├── protocol.rs     UDP packet encoding/decoding
└── bin/
    ├── server.rs   authoritative 60 Hz UDP game server + AI
    └── client.rs   Macroquad GUI, renderer, minimap, host manager, editor
```

See [`PROTOCOL.md`](PROTOCOL.md) for the UDP wire format.
