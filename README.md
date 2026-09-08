# multiplayer-fps - Maze Wars

A Rust recreation of the classic Maze Wars concept for the 01-edu `multiplayer-fps` subject, plus the supplied React/TypeScript frontend port.

The audited multiplayer path intentionally uses a real UDP client/server architecture. The authoritative graphical multiplayer client is native Macroquad, so there is no browser WebSocket bridge between the player and the game server. The `frontend/` directory contains the ported browser UI and local-bot simulation.

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

- **Level editor:** edit wall cells visually and save/load `custom_level.json`; the web frontend also has an interactive grid painter with immediate local play.
- **Procedural generation:** randomized depth-first-search / recursive-backtracker maze generator.
- **AI players:** server-side eye bots use line-of-sight combat and BFS maze pathfinding; the web frontend also has local tactical bots.
- **GUI host history:** successful native hosts are remembered in `hosts.json`; the web Host Manager keeps aliases/history in browser storage.
- **Ported web UI:** React 19 + TypeScript + Vite + Tailwind frontend with DDA raycasting, HUD, minimap, themes, audio synthesis and mouse look.

## Requirements

- Rust **1.87+** (edition 2021).
- A desktop environment supported by Macroquad for the authoritative multiplayer client.
- Node.js 22+ only if you want to run/build the optional `frontend/` port.

## Run authoritative UDP multiplayer

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

### 2. Start a native client

```bash
cargo run --release --bin client
```

Or simply:

```bash
cargo run --release
```

The client opens the graphical connection screen. Enter:

- server address, for example `127.0.0.1:34254` or just `127.0.0.1`;
- username;
- optional host alias.

For another computer on the LAN, enter the server machine's LAN address, for example `192.168.1.25:34254`.

Duplicate display names are allowed; the server identifies players by its numeric player ID rather than by username.

## Native client controls

| Key | Action |
| --- | --- |
| `W / S` | Move forward / backward |
| `A / D` | Strafe left / right |
| `Q / E` or `← / →` | Turn |
| `Space` | Shoot |
| `N` | Ask the server to switch to the next level |
| `Esc` | Disconnect and return to the connection screen |

The connection screen keeps the last server and username when returning with `Esc`.

## Ported web frontend

The browser frontend is intentionally separate from the authoritative UDP client because browsers cannot open raw UDP sockets directly.

```bash
cd frontend
npm ci
npm run dev
```

Open `http://localhost:3000`.

Web controls include WASD, mouse look using Pointer Lock, click-drag fallback when Pointer Lock is unavailable, left mouse/Space to fire, `R` to reload and `Tab` for the scoreboard.

See [`frontend/README.md`](frontend/README.md) for details.

## Level editor

Open **LEVEL EDITOR** from the native connection screen.

- Left mouse: toggle a wall cell.
- `G`: generate a new procedural maze.
- Size button: cycle through 15x15, 21x21 and 27x27.
- `S`: save `custom_level.json`.
- `L`: load `custom_level.json`.
- `Esc`: return to the connection screen.

The web frontend also includes a separate visual editor that can launch the custom map immediately in local mode and export JSON/Rust matrix data.

## Multiplayer / audit setup

The audit permits multiple local clients if several physical machines cannot communicate. A simple local test is:

```bash
cargo run --release --bin server -- --bots 0
```

Then launch 10+ native client processes and connect all of them to:

```text
127.0.0.1:34254
```

For a LAN test, keep the default `0.0.0.0:34254` server bind and connect clients to the server machine's LAN IP.

## Tests and checks

```bash
cargo test --locked
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --bins --locked

cd frontend
npm ci
npm run build
```

CI additionally checks compilation on the declared Rust 1.87 MSRV. The UDP integration test starts 10 independent clients using the same display name, proving both the minimum connection capacity and ID-based duplicate-name handling.

The level tests verify that the three built-in mazes have strictly increasing dead-end counts.

## Architecture

```text
src/
├── lib.rs          shared crate
├── maze.rs         maze generation, collision, ray casting, BFS
├── protocol.rs     UDP packet encoding/decoding
└── bin/
    ├── server.rs   authoritative 60 Hz UDP game server + AI
    └── client.rs   Macroquad UDP client, GUI, renderer, minimap, host manager, editor

frontend/
├── src/
│   ├── components/ React HUD, minimap, host manager, editor, settings
│   ├── engine/     browser DDA raycaster, bot AI, levels, maze generator
│   ├── audio/      Web Audio effects
│   └── App.tsx
└── package.json
```

See [`PROTOCOL.md`](PROTOCOL.md) for the UDP wire format.
