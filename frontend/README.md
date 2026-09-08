# Maze Wars 3D frontend

React/TypeScript port of the supplied Maze Wars UI.

## Run

```bash
npm install
npm run dev
```

Open `http://localhost:3000`.

## Controls

- `W` / `S` - forward / back
- `A` / `D` - strafe
- Mouse - look after pointer lock
- Left mouse - fire after pointer lock
- Click-drag - mouse-look fallback when Pointer Lock is blocked by an embedded preview
- `Space` - fire
- `R` - reload
- `Tab` - scoreboard
- `Esc` - browser releases Pointer Lock

## Current role in the repository

This frontend is the ported browser UI and local-bot simulation. The authoritative UDP multiplayer implementation remains the Rust `server` + native Rust `client` in the repository root. Browsers cannot open raw UDP sockets directly, so the Host Gateway UI is not presented as a raw browser UDP implementation.

Included bonuses: visual level editor, procedural maze generation, AI bot mode, host aliases/history, themes and audio synthesis.
