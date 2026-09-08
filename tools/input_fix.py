from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    if old not in text:
        raise SystemExit(f"missing marker: {label}")
    return text.replace(old, new, 1)


# protocol.rs
path = Path("src/protocol.rs")
text = path.read_text()
text = replace_once(text, "#[derive(Debug, Clone)]\npub enum ClientMessage", "use crate::maze::Maze;\n\n#[derive(Debug, Clone)]\npub enum ClientMessage", "protocol Maze import")
text = replace_once(
    text,
    "        turn: f32,\n    },\n    Shoot(u32),",
    "        turn: f32,\n        look_delta: f32,\n    },\n    Shoot(u32),",
    "input look_delta field",
)
text = replace_once(
    text,
    "    SetLevel(usize),\n    Leave,",
    "    SetLevel(usize),\n    CustomLevel(Maze),\n    Leave,",
    "client custom level variant",
)
text = replace_once(
    text,
    "    Level(usize),\n    Pong,",
    "    Level(usize),\n    CustomLevel(Maze),\n    Pong,",
    "server custom level variant",
)
text = replace_once(
    text,
    '''        ClientMessage::Input {\n            seq,\n            forward,\n            strafe,\n            turn,\n        } => format!(\n            "INPUT|{}|{:.3}|{:.3}|{:.3}\\n",\n            seq,\n            forward.clamp(-1.0, 1.0),\n            strafe.clamp(-1.0, 1.0),\n            turn.clamp(-1.0, 1.0)\n        ),''',
    '''        ClientMessage::Input {\n            seq,\n            forward,\n            strafe,\n            turn,\n            look_delta,\n        } => {\n            let look_delta = if look_delta.is_finite() {\n                *look_delta\n            } else {\n                0.0\n            };\n            format!(\n                "INPUT|{}|{:.3}|{:.3}|{:.3}|{:.6}\\n",\n                seq,\n                forward.clamp(-1.0, 1.0),\n                strafe.clamp(-1.0, 1.0),\n                turn.clamp(-1.0, 1.0),\n                look_delta\n            )\n        }''',
    "encode input",
)
text = replace_once(
    text,
    '        ClientMessage::SetLevel(level) => format!("SETLEVEL|{}\\n", level),\n        ClientMessage::Leave => "LEAVE\\n".to_string(),',
    '        ClientMessage::SetLevel(level) => format!("SETLEVEL|{}\\n", level),\n        ClientMessage::CustomLevel(maze) => encode_maze("CUSTOM", maze),\n        ClientMessage::Leave => "LEAVE\\n".to_string(),',
    "encode custom level",
)
text = replace_once(
    text,
    '''        "INPUT" if parts.len() >= 5 => Some(ClientMessage::Input {\n            seq: parts[1].parse().ok()?,\n            forward: parts[2].parse::<f32>().ok()?.clamp(-1.0, 1.0),\n            strafe: parts[3].parse::<f32>().ok()?.clamp(-1.0, 1.0),\n            turn: parts[4].parse::<f32>().ok()?.clamp(-1.0, 1.0),\n        }),''',
    '''        "INPUT" if parts.len() >= 5 => {\n            let look_delta = parts\n                .get(5)\n                .and_then(|value| value.parse::<f32>().ok())\n                .unwrap_or(0.0);\n            if !look_delta.is_finite() {\n                return None;\n            }\n            Some(ClientMessage::Input {\n                seq: parts[1].parse().ok()?,\n                forward: parts[2].parse::<f32>().ok()?.clamp(-1.0, 1.0),\n                strafe: parts[3].parse::<f32>().ok()?.clamp(-1.0, 1.0),\n                turn: parts[4].parse::<f32>().ok()?.clamp(-1.0, 1.0),\n                look_delta,\n            })\n        }''',
    "parse input",
)
text = replace_once(
    text,
    '        "SETLEVEL" => Some(ClientMessage::SetLevel(parts.get(1)?.parse().ok()?)),\n        "LEAVE" => Some(ClientMessage::Leave),',
    '        "SETLEVEL" => Some(ClientMessage::SetLevel(parts.get(1)?.parse().ok()?)),\n        "CUSTOM" => Some(ClientMessage::CustomLevel(parse_maze(&parts)?)),\n        "LEAVE" => Some(ClientMessage::Leave),',
    "parse custom client",
)
text = replace_once(
    text,
    '''pub fn level(index: usize) -> String {\n    format!("LEVEL|{}\\n", index)\n}\n\npub fn shot''',
    '''pub fn level(index: usize) -> String {\n    format!("LEVEL|{}\\n", index)\n}\n\npub fn custom_level(maze: &Maze) -> String {\n    encode_maze("CUSTOMLEVEL", maze)\n}\n\npub fn shot''',
    "custom level encoder function",
)
text = replace_once(
    text,
    '        "LEVEL" if parts.len() >= 2 => Some(ServerMessage::Level(parts[1].parse().ok()?)),\n        "PONG" => Some(ServerMessage::Pong),',
    '        "LEVEL" if parts.len() >= 2 => Some(ServerMessage::Level(parts[1].parse().ok()?)),\n        "CUSTOMLEVEL" => Some(ServerMessage::CustomLevel(parse_maze(&parts)?)),\n        "PONG" => Some(ServerMessage::Pong),',
    "parse custom server",
)
helpers = r'''
fn encode_maze(prefix: &str, maze: &Maze) -> String {
    let cells: String = maze
        .cells
        .iter()
        .map(|cell| if *cell == 0 { '0' } else { '1' })
        .collect();
    format!(
        "{}|{}|{}|{}|{}\n",
        prefix, maze.width, maze.height, maze.seed, cells
    )
}

fn parse_maze(parts: &[&str]) -> Option<Maze> {
    if parts.len() < 5 {
        return None;
    }
    let width = parts[1].parse::<usize>().ok()?;
    let height = parts[2].parse::<usize>().ok()?;
    let seed = parts[3].parse::<u64>().ok()?;
    if !(7..=31).contains(&width)
        || !(7..=31).contains(&height)
        || width % 2 == 0
        || height % 2 == 0
    {
        return None;
    }
    let expected = width.checked_mul(height)?;
    let raw = parts[4].as_bytes();
    if raw.len() != expected {
        return None;
    }
    let mut cells = Vec::with_capacity(expected);
    for byte in raw {
        cells.push(match byte {
            b'0' => 0,
            b'1' => 1,
            _ => return None,
        });
    }
    for x in 0..width {
        if cells[x] == 0 || cells[(height - 1) * width + x] == 0 {
            return None;
        }
    }
    for y in 0..height {
        if cells[y * width] == 0 || cells[y * width + width - 1] == 0 {
            return None;
        }
    }
    if cells.iter().filter(|cell| **cell == 0).count() < 2 {
        return None;
    }
    Some(Maze {
        name: "Custom Maze".to_string(),
        difficulty: "Custom".to_string(),
        width,
        height,
        seed,
        cells,
    })
}

'''
text = replace_once(text, "#[cfg(test)]\nmod tests {", helpers + "#[cfg(test)]\nmod tests {", "maze protocol helpers")
text = replace_once(
    text,
    '''            turn: 0.25,\n        });''',
    '''            turn: 0.25,\n            look_delta: -0.123456,\n        });''',
    "input test construction",
)
text = replace_once(
    text,
    '''                turn,\n            } => {\n                assert_eq!(seq, 7);\n                assert_eq!(forward, 1.0);\n                assert_eq!(strafe, -0.5);\n                assert_eq!(turn, 0.25);''',
    '''                turn,\n                look_delta,\n            } => {\n                assert_eq!(seq, 7);\n                assert_eq!(forward, 1.0);\n                assert_eq!(strafe, -0.5);\n                assert_eq!(turn, 0.25);\n                assert!((look_delta + 0.123456).abs() < 0.000001);''',
    "input test match",
)
custom_test = r'''
    #[test]
    fn protocol_round_trips_custom_level() {
        let maze = Maze::generated("Test", "Custom", 15, 15, 42);
        let encoded = encode_client(&ClientMessage::CustomLevel(maze.clone()));
        match parse_client(&encoded).unwrap() {
            ClientMessage::CustomLevel(decoded) => {
                assert_eq!(decoded.width, maze.width);
                assert_eq!(decoded.height, maze.height);
                assert_eq!(decoded.seed, maze.seed);
                assert_eq!(decoded.cells, maze.cells);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

'''
text = replace_once(text, "    #[test]\n    fn username_is_protocol_safe()", custom_test + "    #[test]\n    fn username_is_protocol_safe()", "custom protocol test")
path.write_text(text)


# server.rs
path = Path("src/bin/server.rs")
text = path.read_text()
text = replace_once(
    text,
    '''use multiplayer_fps::protocol::{\n    level as level_packet, parse_client, pong, reject, shot, state, welcome, ClientMessage,\n    NetPlayer,\n};''',
    '''use multiplayer_fps::protocol::{\n    custom_level as custom_level_packet, level as level_packet, parse_client, pong, reject, shot,\n    state, welcome, ClientMessage, NetPlayer,\n};''',
    "server imports",
)
text = replace_once(text, "    let levels = builtin_levels();", "    let mut levels = builtin_levels();", "mutable levels")
text = replace_once(text, "            &levels,\n            &mut level_index,", "            &mut levels,\n            &mut level_index,", "receive mutable levels")
text = replace_once(text, "    levels: &[Maze],\n    level_index: &mut usize,", "    levels: &mut Vec<Maze>,\n    level_index: &mut usize,", "receive signature")
text = replace_once(
    text,
    '''                        let _ = socket.send_to(\n                            welcome(player.id, *level_index, player.x, player.y).as_bytes(),\n                            source,\n                        );''',
    '''                        let _ = socket.send_to(\n                            welcome(player.id, *level_index, player.x, player.y).as_bytes(),\n                            source,\n                        );\n                        send_active_custom(socket, source, levels, *level_index);''',
    "repeat welcome custom",
)
text = replace_once(
    text,
    '''                let _ = socket.send_to(\n                    welcome(id, *level_index, spawn.0, spawn.1).as_bytes(),\n                    source,\n                );\n                println!("[join] {name} from {source} as #{id}");''',
    '''                let _ = socket.send_to(\n                    welcome(id, *level_index, spawn.0, spawn.1).as_bytes(),\n                    source,\n                );\n                send_active_custom(socket, source, levels, *level_index);\n                println!("[join] {name} from {source} as #{id}");''',
    "new welcome custom",
)
text = replace_once(
    text,
    '''            ClientMessage::Input {\n                seq: _,\n                forward,\n                strafe,\n                turn,\n            } => {\n                if let Some(player) = player_for_source_mut(players, address_to_id, source) {\n                    player.last_seen = Instant::now();\n                    player.input = InputState {\n                        forward,\n                        strafe,\n                        turn,\n                    };\n                }\n            }''',
    '''            ClientMessage::Input {\n                seq: _,\n                forward,\n                strafe,\n                turn,\n                look_delta,\n            } => {\n                if let Some(player) = player_for_source_mut(players, address_to_id, source) {\n                    player.last_seen = Instant::now();\n                    if look_delta.is_finite() {\n                        player.angle = normalize_angle(player.angle + look_delta);\n                    }\n                    player.input = InputState {\n                        forward,\n                        strafe,\n                        turn,\n                    };\n                }\n            }''',
    "server input look",
)
custom_arm = r'''            ClientMessage::CustomLevel(maze) => {
                if address_to_id.contains_key(&source)
                    && Instant::now() >= *level_change_allowed_at
                {
                    const CUSTOM_INDEX: usize = 3;
                    if levels.len() == CUSTOM_INDEX {
                        levels.push(maze);
                    } else if levels.len() > CUSTOM_INDEX {
                        levels[CUSTOM_INDEX] = maze;
                    } else {
                        continue;
                    }
                    *level_index = CUSTOM_INDEX;
                    reset_for_level(players, &levels[CUSTOM_INDEX]);
                    *level_change_allowed_at = Instant::now() + Duration::from_secs(2);
                    broadcast_raw(
                        socket,
                        players,
                        &custom_level_packet(&levels[CUSTOM_INDEX]),
                    );
                    broadcast_raw(socket, players, &level_packet(CUSTOM_INDEX));
                    println!(
                        "[level] custom - {}x{} ({} dead ends)",
                        levels[CUSTOM_INDEX].width,
                        levels[CUSTOM_INDEX].height,
                        levels[CUSTOM_INDEX].dead_ends()
                    );
                }
            }
'''
text = replace_once(text, "            ClientMessage::Shoot(_) => {", custom_arm + "            ClientMessage::Shoot(_) => {", "custom server arm")
helper = r'''fn send_active_custom(
    socket: &UdpSocket,
    source: SocketAddr,
    levels: &[Maze],
    level_index: usize,
) {
    if level_index >= 3 {
        if let Some(maze) = levels.get(level_index) {
            let _ = socket.send_to(custom_level_packet(maze).as_bytes(), source);
        }
    }
}

'''
text = replace_once(text, "fn player_for_source_mut<'a>(", helper + "fn player_for_source_mut<'a>(", "custom handshake helper")
path.write_text(text)


# client.rs
path = Path("src/bin/client.rs")
text = path.read_text()
text = replace_once(text, "const MAX_AMMO: i32 = 24;", "const MOUSE_RADIANS_PER_PIXEL: f32 = 0.0025;", "mouse constant")
text = replace_once(
    text,
    '''struct PendingConnection {\n    network: NetworkClient,\n    started: Instant,\n}''',
    '''struct PendingConnection {\n    network: NetworkClient,\n    started: Instant,\n    custom_level: Option<Maze>,\n}''',
    "pending custom",
)
text = replace_once(text, "    ammo: i32,\n    reload_timer: f32,", "    pending_mouse_look: f32,", "game look accumulator")
text = replace_once(
    text,
    '''                screen.pending = Some(PendingConnection {\n                    network,\n                    started: Instant::now(),\n                });''',
    '''                screen.pending = Some(PendingConnection {\n                    network,\n                    started: Instant::now(),\n                    custom_level: None,\n                });''',
    "pending init",
)
text = replace_once(
    text,
    '''                ServerMessage::Welcome { id, level, x, y } => joined = Some((id, level, x, y)),\n                ServerMessage::Reject(reason) => rejected = Some(reason),\n                _ => {}''',
    '''                ServerMessage::Welcome { id, level, x, y } => joined = Some((id, level, x, y)),\n                ServerMessage::CustomLevel(maze) => pending.custom_level = Some(maze),\n                ServerMessage::Reject(reason) => rejected = Some(reason),\n                _ => {}''',
    "pending custom receive",
)
text = replace_once(
    text,
    '''            return AppScreen::Game(GameState {\n                server: normalized_server(&screen.server),\n                username: screen.username.clone(),\n                network: pending.network,\n                levels: builtin_levels(),\n                level_index: level.min(2),''',
    '''            let mut levels = builtin_levels();\n            if let Some(custom) = pending.custom_level.take() {\n                levels.push(custom);\n            }\n            let initial_level = if level < levels.len() {\n                level\n            } else {\n                level.min(2)\n            };\n            return AppScreen::Game(GameState {\n                server: normalized_server(&screen.server),\n                username: screen.username.clone(),\n                network: pending.network,\n                levels,\n                level_index: initial_level,''',
    "game custom init",
)
text = replace_once(text, "                ammo: MAX_AMMO,\n                reload_timer: 0.0,", "                pending_mouse_look: 0.0,", "look accumulator init")
reload_block = '''    if game.reload_timer > 0.0 {\n        game.reload_timer = (game.reload_timer - dt).max(0.0);\n        if game.reload_timer == 0.0 {\n            game.ammo = MAX_AMMO;\n        }\n    }\n\n'''
text = replace_once(text, reload_block, "", "remove reload timer")
text = replace_once(
    text,
    '''                TopAction::Procedural => {\n                    disconnect(&game);\n                    return AppScreen::Editor(EditorState {\n                        maze: Maze::generated("Procedural Maze", "Generated", 21, 21, time_seed()),\n                        status: "Generated procedural maze. G regenerates, S saves.".to_string(),\n                        generation_size: 21,\n                    });\n                }\n                TopAction::Gateway => {\n                    disconnect(&game);\n                    return connect_after_game(&game, "Returned to gateway.");\n                }\n                TopAction::Editor => {\n                    disconnect(&game);\n                    return AppScreen::Editor(EditorState {\n                        maze: Maze::generated("Editor Maze", "Custom", 21, 21, time_seed()),\n                        status: "Left click toggles walls. G generates, S saves, L loads."\n                            .to_string(),\n                        generation_size: 21,\n                    });\n                }''',
    '''                TopAction::Procedural => {\n                    let maze = Maze::generated(\n                        "Procedural Maze",\n                        "Generated",\n                        21,\n                        21,\n                        time_seed(),\n                    );\n                    request_custom_level(&mut game, maze, "Procedural level requested.");\n                }\n                TopAction::Gateway => {\n                    game.status =\n                        "Gateway disabled during a live match. Press Esc twice to disconnect."\n                            .to_string();\n                }\n                TopAction::Editor => match load_custom_level_file() {\n                    Ok(maze) => {\n                        request_custom_level(&mut game, maze, "Saved custom level requested.");\n                    }\n                    Err(error) => {\n                        game.status = format!(\n                            "Custom level unavailable: {error}. Edit and save it from the gateway."\n                        );\n                    }\n                },''',
    "safe top actions",
)
input_old = '''    let controls_enabled = game.overlay == Overlay::None;\n    let mut forward = 0.0;\n    let mut strafe = 0.0;\n    let mut turn = 0.0;\n    if controls_enabled {\n        forward = axis(KeyCode::W, KeyCode::S);\n        strafe = axis(KeyCode::D, KeyCode::A);\n        turn = axis(KeyCode::Right, KeyCode::Left) + axis(KeyCode::E, KeyCode::Q);\n        if game.cursor_grabbed {\n            let mouse = mouse_delta_position();\n            turn += (-mouse.x * 120.0 * game.settings.mouse_sensitivity).clamp(-1.0, 1.0);\n        }\n        turn = turn.clamp(-1.0, 1.0);\n\n        let maze = &game.levels[game.level_index];\n        move_entity(\n            maze,\n            &mut game.x,\n            &mut game.y,\n            &mut game.angle,\n            forward,\n            strafe,\n            turn,\n            dt,\n        );\n\n        if is_key_pressed(KeyCode::R) && game.ammo < MAX_AMMO && game.reload_timer == 0.0 {\n            game.reload_timer = 1.0;\n        }\n\n        let mouse_fire = game.cursor_grabbed && is_mouse_button_pressed(MouseButton::Left);\n        if is_key_pressed(KeyCode::Space) || mouse_fire {\n            fire_weapon(&mut game);\n        }\n    }\n\n    if game.network.last_input_sent.elapsed() >= INPUT_SEND_INTERVAL {\n        game.network.seq = game.network.seq.wrapping_add(1);\n        let packet = encode_client(&ClientMessage::Input {\n            seq: game.network.seq,\n            forward,\n            strafe,\n            turn,\n        });\n        let _ = game.network.socket.send(packet.as_bytes());\n        game.network.last_input_sent = Instant::now();\n    }\n'''
input_new = '''    let controls_enabled = game.overlay == Overlay::None;\n    let mut forward = 0.0;\n    let mut strafe = 0.0;\n    let mut turn = 0.0;\n    let mut wants_fire = false;\n    if controls_enabled {\n        forward = axis(KeyCode::W, KeyCode::S);\n        strafe = axis(KeyCode::D, KeyCode::A);\n        turn = (axis(KeyCode::Right, KeyCode::Left) + axis(KeyCode::E, KeyCode::Q))\n            .clamp(-1.0, 1.0);\n        if game.cursor_grabbed {\n            let mouse = mouse_delta_position();\n            let mouse_pixels_x = mouse.x * screen_width() * 0.5;\n            let look_delta =\n                -mouse_pixels_x * MOUSE_RADIANS_PER_PIXEL * game.settings.mouse_sensitivity;\n            if look_delta.is_finite() {\n                game.angle = normalize_angle(game.angle + look_delta);\n                game.pending_mouse_look += look_delta;\n            }\n        }\n\n        let maze = &game.levels[game.level_index];\n        move_entity(\n            maze,\n            &mut game.x,\n            &mut game.y,\n            &mut game.angle,\n            forward,\n            strafe,\n            turn,\n            dt,\n        );\n\n        let mouse_fire = game.cursor_grabbed && is_mouse_button_pressed(MouseButton::Left);\n        wants_fire = is_key_pressed(KeyCode::Space) || mouse_fire;\n    }\n\n    if game.network.last_input_sent.elapsed() >= INPUT_SEND_INTERVAL || wants_fire {\n        game.network.seq = game.network.seq.wrapping_add(1);\n        let packet = encode_client(&ClientMessage::Input {\n            seq: game.network.seq,\n            forward,\n            strafe,\n            turn,\n            look_delta: game.pending_mouse_look,\n        });\n        if game.network.socket.send(packet.as_bytes()).is_ok() {\n            game.pending_mouse_look = 0.0;\n            game.network.last_input_sent = Instant::now();\n        }\n    }\n\n    if wants_fire {\n        fire_weapon(&mut game);\n    }\n'''
text = replace_once(text, input_old, input_new, "frame-independent mouse input")
level_handler = '''            ServerMessage::Level(level) => {\n                if level < game.levels.len() {\n                    game.level_index = level;\n                    game.remotes.clear();\n                    game.status = format!("Level changed to {}", level + 1);\n                }\n            }\n'''
custom_handler = '''            ServerMessage::CustomLevel(maze) => {\n                const CUSTOM_INDEX: usize = 3;\n                if game.levels.len() == CUSTOM_INDEX {\n                    game.levels.push(maze);\n                } else if game.levels.len() > CUSTOM_INDEX {\n                    game.levels[CUSTOM_INDEX] = maze;\n                }\n                if game.levels.len() > CUSTOM_INDEX {\n                    game.level_index = CUSTOM_INDEX;\n                    game.remotes.clear();\n                    game.status = "Custom level activated.".to_string();\n                }\n            }\n'''
text = replace_once(text, level_handler, level_handler + custom_handler, "client custom level handler")
hud_old = '''    draw_text("CHARGE", hull.x + 12.0, hull.y + 54.0, 12.0, palette.muted);\n    let ammo_text = if game.reload_timer > 0.0 {\n        "RELOADING".to_string()\n    } else {\n        format!("{}/{}", game.ammo, MAX_AMMO)\n    };\n    draw_text(\n        ammo_text,\n        hull.x + hull.w - 78.0,\n        hull.y + 54.0,\n        12.0,\n        palette.accent,\n    );\n    let ammo_ratio = if game.reload_timer > 0.0 {\n        1.0 - game.reload_timer.clamp(0.0, 1.0)\n    } else {\n        game.ammo as f32 / MAX_AMMO as f32\n    };\n    draw_bar(\n        Rect::new(hull.x + 12.0, hull.y + 61.0, hull.w - 24.0, 8.0),\n        ammo_ratio,\n        palette.accent,\n    );\n'''
hud_new = '''    draw_text("WEAPON", hull.x + 12.0, hull.y + 54.0, 12.0, palette.muted);\n    draw_text(\n        "READY",\n        hull.x + hull.w - 58.0,\n        hull.y + 54.0,\n        12.0,\n        palette.accent,\n    );\n    draw_bar(\n        Rect::new(hull.x + 12.0, hull.y + 61.0, hull.w - 24.0, 8.0),\n        1.0,\n        palette.accent,\n    );\n'''
text = replace_once(text, hud_old, hud_new, "remove ammo HUD")
text = replace_once(text, '"WASD Move   Mouse / arrows Turn   Space / Click Fire   R Reload   TAB Leaderboard";', '"WASD Move   Mouse / arrows Turn   Space / Click Fire   TAB Leaderboard";', "helper text")
text = replace_once(
    text,
    '        "L{} / 3   {} dead ends",\n        game.level_index + 1,\n        maze.dead_ends()',
    '        "L{} / {}   {} dead ends",\n        game.level_index + 1,\n        game.levels.len(),\n        maze.dead_ends()',
    "dynamic level count",
)
text = replace_once(
    text,
    '    draw_dark_button(editor_button_rect(), "EDITOR", false);',
    '    draw_dark_button(\n        editor_button_rect(),\n        "CUSTOM",\n        game.level_index == 3,\n    );',
    "custom header button",
)
text = replace_once(
    text,
    '''fn fire_weapon(game: &mut GameState) {\n    if game.health <= 0 || game.reload_timer > 0.0 {\n        return;\n    }\n    if game.ammo <= 0 {\n        game.reload_timer = 1.0;\n        return;\n    }\n    game.network.seq = game.network.seq.wrapping_add(1);\n    let packet = encode_client(&ClientMessage::Shoot(game.network.seq));\n    let _ = game.network.socket.send(packet.as_bytes());\n    game.ammo -= 1;\n    game.shot_flash = 0.12;\n    if game.ammo == 0 {\n        game.reload_timer = 1.0;\n    }\n}\n''',
    '''fn fire_weapon(game: &mut GameState) {\n    if game.health <= 0 {\n        return;\n    }\n    game.network.seq = game.network.seq.wrapping_add(1);\n    let packet = encode_client(&ClientMessage::Shoot(game.network.seq));\n    let _ = game.network.socket.send(packet.as_bytes());\n    game.shot_flash = 0.12;\n}\n\nfn request_custom_level(game: &mut GameState, maze: Maze, status: &str) {\n    let packet = encode_client(&ClientMessage::CustomLevel(maze));\n    match game.network.socket.send(packet.as_bytes()) {\n        Ok(_) => game.status = status.to_string(),\n        Err(error) => game.status = format!("Custom level send failed: {error}"),\n    }\n}\n\nfn load_custom_level_file() -> io::Result<Maze> {\n    let json = fs::read_to_string(CUSTOM_LEVEL_FILE)?;\n    let maze = serde_json::from_str::<Maze>(&json).map_err(io::Error::other)?;\n    if maze.cells.len() != maze.width.saturating_mul(maze.height)\n        || !(7..=31).contains(&maze.width)\n        || !(7..=31).contains(&maze.height)\n        || maze.width % 2 == 0\n        || maze.height % 2 == 0\n    {\n        return Err(io::Error::new(\n            io::ErrorKind::InvalidData,\n            "invalid maze dimensions",\n        ));\n    }\n    Ok(maze)\n}\n''',
    "simple firing and custom request",
)
text = replace_once(
    text,
    '''fn normalized_server(server: &str) -> String {\n    let server = server.trim();\n    if server.parse::<std::net::IpAddr>().is_ok() {\n        format!("{server}:34254")\n    } else {\n        server.to_string()\n    }\n}\n''',
    '''fn normalized_server(server: &str) -> String {\n    let server = server.trim();\n    match server.parse::<std::net::IpAddr>() {\n        Ok(std::net::IpAddr::V4(address)) => format!("{address}:34254"),\n        Ok(std::net::IpAddr::V6(address)) => format!("[{address}]:34254"),\n        Err(_) => server.to_string(),\n    }\n}\n''',
    "IPv6 normalization",
)
text = replace_once(
    text,
    '''fn server_host(server: &str) -> String {\n    server.split(':').next().unwrap_or(server).to_string()\n}\n''',
    '''fn server_host(server: &str) -> String {\n    server\n        .parse::<std::net::SocketAddr>()\n        .map(|address| address.ip().to_string())\n        .unwrap_or_else(|_| server.to_string())\n}\n''',
    "IPv6 host label",
)
path.write_text(text)


# PROTOCOL.md
Path("PROTOCOL.md").write_text('''# UDP protocol\n\nDefault port: **34254/UDP**.\n\nPackets are UTF-8 text datagrams terminated by `\\n`. The server is authoritative: clients send movement intent and accumulated mouse-look deltas, not trusted world coordinates.\n\n## Client -> server\n\n```text\nJOIN|<username>\nINPUT|<sequence>|<forward>|<strafe>|<keyboard_turn>|<mouse_look_delta>\nSHOOT|<sequence>\nPING\nNEXT\nSETLEVEL|<level_index>\nCUSTOM|<width>|<height>|<seed>|<wall_bits>\nLEAVE\n```\n\nMovement axes are clamped to `[-1, 1]`. `mouse_look_delta` is an accumulated angular impulse in radians since the previous INPUT packet, so mouse sensitivity is independent of render FPS. Usernames are reduced to 16 protocol-safe ASCII characters.\n\nCustom mazes are bounded to odd dimensions from 7 through 31 and use one `0`/`1` bit per cell. The server validates them, installs them as level index 3, and broadcasts the maze to every client.\n\n## Server -> client\n\n```text\nWELCOME|<player_id>|<level_index>|<spawn_x>|<spawn_y>\nSTATE|<tick>|<level_index>|<player>;<player>;...\nSHOT|<shooter_id>|<target_id_or_0>\nLEVEL|<level_index>\nCUSTOMLEVEL|<width>|<height>|<seed>|<wall_bits>\nPONG\nREJECT|<reason>\n```\n\nA player inside `STATE` is:\n\n```text\nid,name,x,y,angle,health,score,is_bot\n```\n\nThe server simulates at **60 Hz** and broadcasts world snapshots every two ticks (**30 Hz**). Client rendering is independent of the network tick rate and interpolates remote players.\n\n## UDP behaviour\n\n- `PING` / input packets refresh the player's timeout.\n- Clients are dropped after 5 seconds without activity.\n- A repeated `JOIN` from the same socket returns the existing `WELCOME` packet, which makes the initial handshake tolerant of UDP packet loss.\n- If the active level is custom, the server also sends its maze payload during the handshake.\n- Old snapshots are ignored by the client using the server tick number.\n- The server accepts up to 64 network clients, exceeding the subject minimum of 10.\n''')

# README.md
path = Path("README.md")
text = path.read_text()
text = text.replace("- Mouse look, left-click/Space fire, reload and weapon HUD.", "- Frame-rate-independent mouse look, left-click/Space fire and weapon HUD.")
text = text.replace("- Native level editor and procedural maze generation.", "- Native level editor, procedural maze generation and playable custom mazes.")
text = text.replace('| `R` | Reload |\n', '')
text = text.replace(
    "Use the top bar for built-in levels, procedural generation, gateway, editor, scoreboard and settings.",
    "Use the top bar for built-in levels, procedural generation, the saved custom maze, scoreboard and settings. Gateway/custom actions do not disconnect a live match; use `Esc` to leave the server and edit maps from the connection screen.",
)
text = text.replace(
    "- `Esc`: return.\n",
    "- `Esc`: return.\n\nAfter saving, connect to the server and press **CUSTOM** in the game header. The maze is sent to the authoritative server and broadcast to all connected clients. **PROCEDURAL** generates and activates a fresh networked custom maze without disconnecting.\n",
)
path.write_text(text)
