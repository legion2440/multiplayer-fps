from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing pattern: {label}")
    return text.replace(old, new, 1)


# --- protocol: absolute angle state + connected custom mazes ---
path = Path("src/protocol.rs")
text = path.read_text()
text = text.replace("look_delta", "angle")

old = '''    Some(Maze {
        name: "Custom Maze".to_string(),
        difficulty: "Custom".to_string(),
        width,
        height,
        seed,
        cells,
    })
}'''
new = '''    let maze = Maze {
        name: "Custom Maze".to_string(),
        difficulty: "Custom".to_string(),
        width,
        height,
        seed,
        cells,
    };
    if !maze.is_connected() {
        return None;
    }
    Some(maze)
}'''
text = replace_once(text, old, new, "protocol connected maze validation")

old = '''    #[test]
    fn username_is_protocol_safe() {
        assert_eq!(sanitize_username("a|b,c;d"), "abcd");
        assert_eq!(sanitize_username(""), "Agent");
    }
}'''
new = '''    #[test]
    fn protocol_rejects_disconnected_custom_level() {
        let mut cells = vec![b'1'; 7 * 7];
        cells[1 * 7 + 1] = b'0';
        cells[5 * 7 + 5] = b'0';
        let walls = String::from_utf8(cells).unwrap();
        let packet = format!("CUSTOM|7|7|1|{walls}\\n");
        assert!(parse_client(&packet).is_none());
    }

    #[test]
    fn username_is_protocol_safe() {
        assert_eq!(sanitize_username("a|b,c;d"), "abcd");
        assert_eq!(sanitize_username(""), "Agent");
    }
}'''
text = replace_once(text, old, new, "protocol disconnected test")
path.write_text(text)


# --- maze: reusable connectivity check ---
path = Path("src/maze.rs")
text = path.read_text()
marker = '''    pub fn dead_ends(&self) -> usize {
'''
insert = '''    pub fn is_connected(&self) -> bool {
        let Some(expected) = self.width.checked_mul(self.height) else {
            return false;
        };
        if self.width == 0 || self.height == 0 || self.cells.len() != expected {
            return false;
        }

        let Some(start) = self.cells.iter().position(|cell| *cell == 0) else {
            return false;
        };
        let open_total = self.cells.iter().filter(|cell| **cell == 0).count();
        let mut seen = vec![false; expected];
        let mut stack = vec![start];
        seen[start] = true;
        let mut reached = 0usize;

        while let Some(index) = stack.pop() {
            reached += 1;
            let x = index % self.width;
            let y = index / self.width;
            let mut visit = |next: usize| {
                if !seen[next] && self.cells[next] == 0 {
                    seen[next] = true;
                    stack.push(next);
                }
            };
            if x > 0 {
                visit(index - 1);
            }
            if x + 1 < self.width {
                visit(index + 1);
            }
            if y > 0 {
                visit(index - self.width);
            }
            if y + 1 < self.height {
                visit(index + self.width);
            }
        }

        reached == open_total
    }

'''
if marker not in text:
    raise SystemExit("missing maze dead_ends marker")
text = text.replace(marker, insert + marker, 1)
path.write_text(text)


# --- server: absolute client angle self-heals after UDP loss ---
path = Path("src/bin/server.rs")
text = path.read_text()
old = '''            ClientMessage::Input {
                seq: _,
                forward,
                strafe,
                turn,
                look_delta,
            } => {
                if let Some(player) = player_for_source_mut(players, address_to_id, source) {
                    player.last_seen = Instant::now();
                    if look_delta.is_finite() {
                        player.angle = normalize_angle(player.angle + look_delta);
                    }
                    player.input = InputState {
                        forward,
                        strafe,
                        turn,
                    };
                }
            }'''
new = '''            ClientMessage::Input {
                seq: _,
                forward,
                strafe,
                turn,
                angle,
            } => {
                if let Some(player) = player_for_source_mut(players, address_to_id, source) {
                    player.last_seen = Instant::now();
                    if angle.is_finite() {
                        player.angle = normalize_angle(angle);
                    }
                    player.input = InputState {
                        forward,
                        strafe,
                        turn,
                    };
                }
            }'''
text = replace_once(text, old, new, "server absolute angle")
path.write_text(text)


# --- client: absolute angle, level recovery, clean HUD ---
path = Path("src/bin/client.rs")
text = path.read_text()
text = replace_once(
    text,
    "    pending_mouse_look: f32,\n",
    "    level_resync_sent: Option<Instant>,\n",
    "client state field",
)
text = replace_once(
    text,
    "                pending_mouse_look: 0.0,\n",
    "                level_resync_sent: None,\n",
    "client state init",
)

old = '''            if look_delta.is_finite() {
                game.angle = normalize_angle(game.angle + look_delta);
                game.pending_mouse_look += look_delta;
            }'''
new = '''            if look_delta.is_finite() {
                game.angle = normalize_angle(game.angle + look_delta);
            }'''
text = replace_once(text, old, new, "client local mouse angle")

old = '''        let packet = encode_client(&ClientMessage::Input {
            seq: game.network.seq,
            forward,
            strafe,
            turn,
            look_delta: game.pending_mouse_look,
        });
        if game.network.socket.send(packet.as_bytes()).is_ok() {
            game.pending_mouse_look = 0.0;
            game.network.last_input_sent = Instant::now();
        }'''
new = '''        let packet = encode_client(&ClientMessage::Input {
            seq: game.network.seq,
            forward,
            strafe,
            turn,
            angle: game.angle,
        });
        if game.network.socket.send(packet.as_bytes()).is_ok() {
            game.network.last_input_sent = Instant::now();
        }'''
text = replace_once(text, old, new, "client absolute angle packet")

old = '''                if level < game.levels.len() && level != game.level_index {
                    game.level_index = level;
                    game.remotes.clear();
                }
'''
new = '''                if level < game.levels.len() {
                    game.level_resync_sent = None;
                    if level != game.level_index {
                        game.level_index = level;
                        game.remotes.clear();
                    }
                } else {
                    request_level_resync(&mut game);
                }
'''
text = replace_once(text, old, new, "state level recovery")

old = '''            ServerMessage::Level(level) => {
                if level < game.levels.len() {
                    game.level_index = level;
                    game.remotes.clear();
                    game.status = format!("Level changed to {}", level + 1);
                }
            }'''
new = '''            ServerMessage::Level(level) => {
                if level < game.levels.len() {
                    game.level_resync_sent = None;
                    game.level_index = level;
                    game.remotes.clear();
                    game.status = format!("Level changed to {}", level + 1);
                } else {
                    request_level_resync(&mut game);
                }
            }'''
text = replace_once(text, old, new, "level packet recovery")

old = '''                if game.levels.len() > CUSTOM_INDEX {
                    game.level_index = CUSTOM_INDEX;
                    game.remotes.clear();
                    game.status = "Custom level activated.".to_string();
                }
'''
new = '''                if game.levels.len() > CUSTOM_INDEX {
                    game.level_index = CUSTOM_INDEX;
                    game.level_resync_sent = None;
                    game.remotes.clear();
                    game.status = "Custom level activated.".to_string();
                }
'''
text = replace_once(text, old, new, "custom clears resync")

old = '''fn request_custom_level(game: &mut GameState, maze: Maze, status: &str) {
    let packet = encode_client(&ClientMessage::CustomLevel(maze));
    match game.network.socket.send(packet.as_bytes()) {
        Ok(_) => game.status = status.to_string(),
        Err(error) => game.status = format!("Custom level send failed: {error}"),
    }
}

fn load_custom_level_file() -> io::Result<Maze> {'''
new = '''fn request_custom_level(game: &mut GameState, maze: Maze, status: &str) {
    let packet = encode_client(&ClientMessage::CustomLevel(maze));
    match game.network.socket.send(packet.as_bytes()) {
        Ok(_) => game.status = status.to_string(),
        Err(error) => game.status = format!("Custom level send failed: {error}"),
    }
}

fn request_level_resync(game: &mut GameState) {
    if game
        .level_resync_sent
        .map(|sent| sent.elapsed() < Duration::from_millis(500))
        .unwrap_or(false)
    {
        return;
    }

    let packet = encode_client(&ClientMessage::Join(game.username.clone()));
    match game.network.socket.send(packet.as_bytes()) {
        Ok(_) => {
            game.level_resync_sent = Some(Instant::now());
            game.status = "Recovering missing level data...".to_string();
        }
        Err(error) => game.status = format!("Level recovery failed: {error}"),
    }
}

fn load_custom_level_file() -> io::Result<Maze> {'''
text = replace_once(text, old, new, "client resync helper")

old = '''    if maze.cells.len() != maze.width.saturating_mul(maze.height)
        || !(7..=31).contains(&maze.width)
        || !(7..=31).contains(&maze.height)
        || maze.width % 2 == 0
        || maze.height % 2 == 0
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid maze dimensions",
        ));
    }
'''
new = '''    if maze.cells.len() != maze.width.saturating_mul(maze.height)
        || !(7..=31).contains(&maze.width)
        || !(7..=31).contains(&maze.height)
        || maze.width % 2 == 0
        || maze.height % 2 == 0
        || !maze.is_connected()
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "invalid or disconnected maze",
        ));
    }
'''
text = replace_once(text, old, new, "client custom connectivity")

old = '''    let hull = Rect::new(view.x + 16.0, view.y + view.h - 104.0, 245.0, 80.0);
    draw_hud_panel(hull, palette);
    draw_text("HULL", hull.x + 12.0, hull.y + 20.0, 12.0, palette.muted);
    draw_text(
        format!("{}/100", game.health.max(0)),
        hull.x + hull.w - 58.0,
        hull.y + 20.0,
        12.0,
        WHITE,
    );
    draw_bar(
        Rect::new(hull.x + 12.0, hull.y + 27.0, hull.w - 24.0, 8.0),
        game.health.max(0) as f32 / 100.0,
        palette.health,
    );
    draw_text("WEAPON", hull.x + 12.0, hull.y + 54.0, 12.0, palette.muted);
    draw_text(
        "READY",
        hull.x + hull.w - 58.0,
        hull.y + 54.0,
        12.0,
        palette.accent,
    );
    draw_bar(
        Rect::new(hull.x + 12.0, hull.y + 61.0, hull.w - 24.0, 8.0),
        1.0,
        palette.accent,
    );'''
new = '''    let hull = Rect::new(view.x + 16.0, view.y + view.h - 76.0, 245.0, 52.0);
    draw_hud_panel(hull, palette);
    draw_text("HULL", hull.x + 12.0, hull.y + 20.0, 12.0, palette.muted);
    draw_text(
        format!("{}/100", game.health.max(0)),
        hull.x + hull.w - 58.0,
        hull.y + 20.0,
        12.0,
        WHITE,
    );
    draw_bar(
        Rect::new(hull.x + 12.0, hull.y + 31.0, hull.w - 24.0, 8.0),
        game.health.max(0) as f32 / 100.0,
        palette.health,
    );'''
text = replace_once(text, old, new, "remove fake weapon bar")
path.write_text(text)


# --- protocol docs ---
path = Path("PROTOCOL.md")
text = path.read_text()
text = text.replace(
    "clients send movement intent and accumulated mouse-look deltas, not trusted world coordinates.",
    "clients send movement intent and their current view angle, not trusted world coordinates.",
)
text = text.replace(
    "INPUT|<sequence>|<forward>|<strafe>|<keyboard_turn>|<mouse_look_delta>",
    "INPUT|<sequence>|<forward>|<strafe>|<keyboard_turn>|<view_angle>",
)
text = text.replace(
    "Movement axes are clamped to `[-1, 1]`. `mouse_look_delta` is an accumulated angular impulse in radians since the previous INPUT packet, so mouse sensitivity is independent of render FPS. Usernames are reduced to 16 protocol-safe ASCII characters.",
    "Movement axes are clamped to `[-1, 1]`. `view_angle` is the client's absolute current angle in radians; sending absolute state makes a later INPUT repair any angle packet lost by UDP. Mouse sensitivity remains independent of render FPS because frame mouse displacement is applied directly to the local angle. Usernames are reduced to 16 protocol-safe ASCII characters.",
)
text = text.replace(
    "Custom mazes are bounded to odd dimensions from 7 through 31 and use one `0`/`1` bit per cell. The server validates them, installs them as level index 3, and broadcasts the maze to every client.",
    "Custom mazes are bounded to odd dimensions from 7 through 31, use one `0`/`1` bit per cell, keep solid outer walls, and must have one connected open region. The server validates them, installs them as level index 3, and broadcasts the maze to every client.",
)
text = text.replace(
    "- If the active level is custom, the server also sends its maze payload during the handshake.\n",
    "- If the active level is custom, the server also sends its maze payload during the handshake. A client that sees an unknown level index re-sends `JOIN`; periodic `STATE` packets therefore recover a lost `CUSTOMLEVEL` datagram.\n",
)
path.write_text(text)
