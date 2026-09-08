use macroquad::prelude::*;
use multiplayer_fps::maze::{builtin_levels, move_entity, normalize_angle, Maze};
use multiplayer_fps::protocol::{
    encode_client, parse_server, ClientMessage, NetPlayer, ServerMessage,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::fs;
use std::io;
use std::net::UdpSocket;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DEFAULT_SERVER: &str = "127.0.0.1:34254";
const HOSTS_FILE: &str = "hosts.json";
const CUSTOM_LEVEL_FILE: &str = "custom_level.json";
const FOV: f32 = PI / 3.0;
const INPUT_SEND_INTERVAL: Duration = Duration::from_millis(33);
const PING_INTERVAL: Duration = Duration::from_secs(1);

fn window_conf() -> Conf {
    Conf {
        window_title: "Maze Wars Multiplayer FPS".to_string(),
        window_width: 1280,
        window_height: 720,
        high_dpi: true,
        window_resizable: true,
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HostEntry {
    alias: String,
    address: String,
    username: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveField {
    Server,
    Username,
    Alias,
}

#[derive(Debug)]
struct ConnectScreen {
    server: String,
    username: String,
    alias: String,
    active: ActiveField,
    hosts: Vec<HostEntry>,
    status: String,
    pending: Option<PendingConnection>,
}

#[derive(Debug)]
struct PendingConnection {
    network: NetworkClient,
    started: Instant,
}

#[derive(Debug)]
struct NetworkClient {
    socket: UdpSocket,
    player_id: u32,
    seq: u32,
    last_input_sent: Instant,
    last_ping: Instant,
}

#[derive(Debug, Clone)]
struct RemoteVisual {
    state: NetPlayer,
    display_x: f32,
    display_y: f32,
}

#[derive(Debug)]
struct GameState {
    server: String,
    username: String,
    network: NetworkClient,
    levels: Vec<Maze>,
    level_index: usize,
    x: f32,
    y: f32,
    angle: f32,
    health: i32,
    score: i32,
    remotes: HashMap<u32, RemoteVisual>,
    last_server_tick: u64,
    shot_flash: f32,
    status: String,
    display_fps: f32,
}

#[derive(Debug)]
struct EditorState {
    maze: Maze,
    status: String,
    generation_size: usize,
}

enum AppScreen {
    Connect(ConnectScreen),
    Game(GameState),
    Editor(EditorState),
}

#[macroquad::main(window_conf)]
async fn main() {
    let hosts = load_hosts();
    let (server, username) = hosts
        .first()
        .map(|host| (host.address.clone(), host.username.clone()))
        .unwrap_or_else(|| (DEFAULT_SERVER.to_string(), "Agent".to_string()));

    let mut screen = AppScreen::Connect(ConnectScreen {
        server,
        username,
        alias: String::new(),
        active: ActiveField::Server,
        hosts,
        status: "Enter a UDP server address and username.".to_string(),
        pending: None,
    });

    loop {
        clear_background(Color::new(0.015, 0.02, 0.03, 1.0));
        screen = match screen {
            AppScreen::Connect(connect) => update_connect(connect),
            AppScreen::Game(game) => update_game(game),
            AppScreen::Editor(editor) => update_editor(editor),
        };
        next_frame().await;
    }
}

fn update_connect(mut screen: ConnectScreen) -> AppScreen {
    draw_connect_background();

    if is_key_pressed(KeyCode::Tab) {
        screen.active = match screen.active {
            ActiveField::Server => ActiveField::Username,
            ActiveField::Username => ActiveField::Alias,
            ActiveField::Alias => ActiveField::Server,
        };
    }
    edit_active_field(&mut screen);

    let panel_x = 70.0;
    let panel_y = 70.0;
    let panel_w = (screen_width() * 0.56).clamp(620.0, 760.0);
    let panel_h = (screen_height() - 140.0).max(560.0);
    draw_rectangle(
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        Color::new(0.025, 0.035, 0.05, 0.96),
    );
    draw_rectangle_lines(
        panel_x,
        panel_y,
        panel_w,
        panel_h,
        2.0,
        Color::new(0.15, 0.75, 0.85, 0.7),
    );

    draw_text("MAZE WARS", panel_x + 36.0, panel_y + 62.0, 46.0, WHITE);
    draw_text(
        "MULTIPLAYER FPS / UDP",
        panel_x + 38.0,
        panel_y + 92.0,
        18.0,
        Color::new(0.3, 0.8, 0.9, 1.0),
    );

    let server_rect = Rect::new(panel_x + 38.0, panel_y + 138.0, panel_w - 76.0, 52.0);
    let user_rect = Rect::new(panel_x + 38.0, panel_y + 226.0, panel_w - 76.0, 52.0);
    let alias_rect = Rect::new(panel_x + 38.0, panel_y + 314.0, panel_w - 76.0, 52.0);
    draw_field(
        "SERVER IP:PORT",
        &screen.server,
        server_rect,
        screen.active == ActiveField::Server,
    );
    draw_field(
        "USERNAME",
        &screen.username,
        user_rect,
        screen.active == ActiveField::Username,
    );
    draw_field(
        "HOST ALIAS (OPTIONAL)",
        &screen.alias,
        alias_rect,
        screen.active == ActiveField::Alias,
    );

    if clicked(server_rect) {
        screen.active = ActiveField::Server;
    }
    if clicked(user_rect) {
        screen.active = ActiveField::Username;
    }
    if clicked(alias_rect) {
        screen.active = ActiveField::Alias;
    }

    let connect_rect = Rect::new(panel_x + 38.0, panel_y + 398.0, 230.0, 54.0);
    let editor_rect = Rect::new(panel_x + 286.0, panel_y + 398.0, 230.0, 54.0);
    let connect_clicked = draw_button(
        connect_rect,
        if screen.pending.is_some() {
            "CONNECTING..."
        } else {
            "CONNECT"
        },
    );
    let editor_clicked = draw_button(editor_rect, "LEVEL EDITOR");

    draw_text(
        &screen.status,
        panel_x + 38.0,
        panel_y + 486.0,
        20.0,
        LIGHTGRAY,
    );
    draw_text("Saved hosts", panel_x + 38.0, panel_y + 532.0, 22.0, WHITE);

    let mut chosen_host = None;
    for (index, host) in screen.hosts.iter().take(5).enumerate() {
        let y = panel_y + 552.0 + index as f32 * 34.0;
        let rect = Rect::new(panel_x + 38.0, y, panel_w - 76.0, 28.0);
        let hover = point_in_rect(mouse_position(), rect);
        if hover {
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                Color::new(0.08, 0.15, 0.19, 1.0),
            );
        }
        let label = format!("{}  -  {}  [{}]", host.alias, host.address, host.username);
        draw_text(
            &label,
            rect.x + 8.0,
            rect.y + 20.0,
            18.0,
            if hover { WHITE } else { GRAY },
        );
        if hover && is_mouse_button_pressed(MouseButton::Left) {
            chosen_host = Some(index);
        }
    }
    if let Some(index) = chosen_host {
        let host = &screen.hosts[index];
        screen.server = host.address.clone();
        screen.username = host.username.clone();
        screen.alias = host.alias.clone();
        screen.status = format!("Loaded host '{}'.", host.alias);
    }

    if editor_clicked {
        return AppScreen::Editor(EditorState {
            maze: Maze::generated("Editor Maze", "Custom", 21, 21, time_seed()),
            status: "Left click toggles walls. G generates, S saves, L loads.".to_string(),
            generation_size: 21,
        });
    }

    if (connect_clicked || is_key_pressed(KeyCode::Enter)) && screen.pending.is_none() {
        match start_connection(&screen.server, &screen.username) {
            Ok(network) => {
                screen.pending = Some(PendingConnection {
                    network,
                    started: Instant::now(),
                });
                screen.status = "Waiting for UDP handshake...".to_string();
            }
            Err(error) => screen.status = format!("Connection error: {error}"),
        }
    }

    if let Some(mut pending) = screen.pending.take() {
        let mut joined = None;
        let mut rejected = None;
        for message in poll_network(&pending.network.socket) {
            match message {
                ServerMessage::Welcome { id, level, x, y } => joined = Some((id, level, x, y)),
                ServerMessage::Reject(reason) => rejected = Some(reason),
                _ => {}
            }
        }

        if let Some(reason) = rejected {
            screen.status = format!("Server rejected connection: {reason}");
        } else if let Some((id, level, x, y)) = joined {
            pending.network.player_id = id;
            remember_host(
                &mut screen.hosts,
                &screen.alias,
                &screen.server,
                &screen.username,
            );
            save_hosts(&screen.hosts);
            return AppScreen::Game(GameState {
                server: screen.server.clone(),
                username: screen.username.clone(),
                network: pending.network,
                levels: builtin_levels(),
                level_index: level.min(2),
                x,
                y,
                angle: 0.0,
                health: 100,
                score: 0,
                remotes: HashMap::new(),
                last_server_tick: 0,
                shot_flash: 0.0,
                status: "Connected".to_string(),
                display_fps: 60.0,
            });
        } else if pending.started.elapsed() > Duration::from_secs(4) {
            screen.status = "No response from server (UDP timeout).".to_string();
        } else {
            if pending.network.last_ping.elapsed() >= Duration::from_millis(500) {
                let packet = encode_client(&ClientMessage::Join(screen.username.clone()));
                let _ = pending.network.socket.send(packet.as_bytes());
                pending.network.last_ping = Instant::now();
            }
            screen.pending = Some(pending);
        }
    }

    AppScreen::Connect(screen)
}

fn update_game(mut game: GameState) -> AppScreen {
    let dt = get_frame_time().min(0.05);
    let instant_fps = 1.0 / dt.max(0.0001);
    let fps_alpha = 1.0 - (-dt / 0.5).exp();
    game.display_fps += (instant_fps - game.display_fps) * fps_alpha;
    let maze = &game.levels[game.level_index];

    let forward = axis(KeyCode::W, KeyCode::S);
    let strafe = axis(KeyCode::D, KeyCode::A);
    let turn = axis(KeyCode::Right, KeyCode::Left) + axis(KeyCode::E, KeyCode::Q);
    move_entity(
        maze,
        &mut game.x,
        &mut game.y,
        &mut game.angle,
        forward,
        strafe,
        turn.clamp(-1.0, 1.0),
        dt,
    );

    if game.network.last_input_sent.elapsed() >= INPUT_SEND_INTERVAL {
        game.network.seq = game.network.seq.wrapping_add(1);
        let packet = encode_client(&ClientMessage::Input {
            seq: game.network.seq,
            forward,
            strafe,
            turn: turn.clamp(-1.0, 1.0),
        });
        let _ = game.network.socket.send(packet.as_bytes());
        game.network.last_input_sent = Instant::now();
    }

    if is_key_pressed(KeyCode::Space) {
        game.network.seq = game.network.seq.wrapping_add(1);
        let packet = encode_client(&ClientMessage::Shoot(game.network.seq));
        let _ = game.network.socket.send(packet.as_bytes());
        game.shot_flash = 0.12;
    }
    if is_key_pressed(KeyCode::N) {
        let _ = game
            .network
            .socket
            .send(encode_client(&ClientMessage::NextLevel).as_bytes());
    }
    if game.network.last_ping.elapsed() >= PING_INTERVAL {
        let _ = game
            .network
            .socket
            .send(encode_client(&ClientMessage::Ping).as_bytes());
        game.network.last_ping = Instant::now();
    }
    if is_key_pressed(KeyCode::Escape) {
        let _ = game
            .network
            .socket
            .send(encode_client(&ClientMessage::Leave).as_bytes());
        return AppScreen::Connect(ConnectScreen {
            server: game.server.clone(),
            username: game.username.clone(),
            alias: String::new(),
            active: ActiveField::Server,
            hosts: load_hosts(),
            status: "Disconnected.".to_string(),
            pending: None,
        });
    }

    for message in poll_network(&game.network.socket) {
        match message {
            ServerMessage::State {
                tick,
                level,
                players,
            } => {
                if tick < game.last_server_tick {
                    continue;
                }
                game.last_server_tick = tick;
                if level < game.levels.len() && level != game.level_index {
                    game.level_index = level;
                    game.remotes.clear();
                }
                let mut seen = Vec::with_capacity(players.len());
                for player in players {
                    if player.id == game.network.player_id {
                        game.health = player.health;
                        game.score = player.score;
                        let error_x = player.x - game.x;
                        let error_y = player.y - game.y;
                        let error_sq = error_x * error_x + error_y * error_y;
                        if error_sq > 0.45 * 0.45 {
                            game.x = player.x;
                            game.y = player.y;
                        } else {
                            game.x += error_x * 0.12;
                            game.y += error_y * 0.12;
                        }
                        game.angle = normalize_angle(
                            game.angle + normalize_angle(player.angle - game.angle) * 0.08,
                        );
                    } else {
                        seen.push(player.id);
                        game.remotes
                            .entry(player.id)
                            .and_modify(|visual| {
                                visual.state = player.clone();
                            })
                            .or_insert(RemoteVisual {
                                display_x: player.x,
                                display_y: player.y,
                                state: player,
                            });
                    }
                }
                game.remotes.retain(|id, _| seen.contains(id));
            }
            ServerMessage::Shot { shooter, target } => {
                if shooter == game.network.player_id || target == Some(game.network.player_id) {
                    game.shot_flash = 0.14;
                }
            }
            ServerMessage::Level(level) => {
                if level < game.levels.len() {
                    game.level_index = level;
                    game.remotes.clear();
                    game.status = format!("Level changed to {}", level + 1);
                }
            }
            ServerMessage::Reject(reason) => game.status = reason,
            ServerMessage::Welcome { .. } | ServerMessage::Pong => {}
        }
    }

    let smoothing = 1.0 - (-12.0 * dt).exp();
    for visual in game.remotes.values_mut() {
        visual.display_x += (visual.state.x - visual.display_x) * smoothing;
        visual.display_y += (visual.state.y - visual.display_y) * smoothing;
    }

    game.shot_flash = (game.shot_flash - dt).max(0.0);
    draw_game(&game);
    AppScreen::Game(game)
}

fn update_editor(mut editor: EditorState) -> AppScreen {
    clear_background(Color::new(0.012, 0.018, 0.025, 1.0));
    draw_text("LEVEL EDITOR", 42.0, 54.0, 38.0, WHITE);
    draw_text(
        "Bonus: editable mazes + procedural DFS generator",
        44.0,
        82.0,
        18.0,
        SKYBLUE,
    );

    let top = 110.0;
    let left = 42.0;
    let available_w = screen_width() - 84.0;
    let available_h = screen_height() - 220.0;
    let cell = (available_w / editor.maze.width as f32)
        .min(available_h / editor.maze.height as f32)
        .floor()
        .max(4.0);
    let grid_w = cell * editor.maze.width as f32;
    let grid_h = cell * editor.maze.height as f32;

    for y in 0..editor.maze.height {
        for x in 0..editor.maze.width {
            let rect = Rect::new(left + x as f32 * cell, top + y as f32 * cell, cell, cell);
            let wall = editor.maze.cells[y * editor.maze.width + x] != 0;
            draw_rectangle(
                rect.x,
                rect.y,
                rect.w - 1.0,
                rect.h - 1.0,
                if wall {
                    Color::new(0.65, 0.75, 0.78, 1.0)
                } else {
                    Color::new(0.035, 0.06, 0.07, 1.0)
                },
            );
        }
    }

    if is_mouse_button_pressed(MouseButton::Left) {
        let (mx, my) = mouse_position();
        if mx >= left && my >= top && mx < left + grid_w && my < top + grid_h {
            let x = ((mx - left) / cell) as usize;
            let y = ((my - top) / cell) as usize;
            editor.maze.toggle_wall(x, y);
        }
    }

    let button_y = screen_height() - 86.0;
    let generate = draw_button(Rect::new(42.0, button_y, 150.0, 46.0), "GENERATE [G]");
    let size = draw_button(
        Rect::new(204.0, button_y, 150.0, 46.0),
        &format!("SIZE {}", editor.generation_size),
    );
    let save = draw_button(Rect::new(366.0, button_y, 130.0, 46.0), "SAVE [S]");
    let load = draw_button(Rect::new(508.0, button_y, 130.0, 46.0), "LOAD [L]");
    let back = draw_button(Rect::new(650.0, button_y, 130.0, 46.0), "BACK [ESC]");

    if generate || is_key_pressed(KeyCode::G) {
        editor.maze = Maze::generated(
            "Procedural Custom Maze",
            "Generated",
            editor.generation_size,
            editor.generation_size,
            time_seed(),
        );
        editor.status = format!("Generated maze with {} dead ends.", editor.maze.dead_ends());
    }
    if size {
        editor.generation_size = match editor.generation_size {
            15 => 21,
            21 => 27,
            _ => 15,
        };
        editor.status = format!(
            "Generator size set to {}x{}.",
            editor.generation_size, editor.generation_size
        );
    }
    if save || is_key_pressed(KeyCode::S) {
        editor.status = match serde_json::to_string_pretty(&editor.maze)
            .map_err(io::Error::other)
            .and_then(|json| fs::write(CUSTOM_LEVEL_FILE, json))
        {
            Ok(()) => format!("Saved {CUSTOM_LEVEL_FILE}."),
            Err(error) => format!("Save failed: {error}"),
        };
    }
    if load || is_key_pressed(KeyCode::L) {
        editor.status = match fs::read_to_string(CUSTOM_LEVEL_FILE)
            .and_then(|json| serde_json::from_str::<Maze>(&json).map_err(io::Error::other))
        {
            Ok(maze) if maze.cells.len() == maze.width * maze.height => {
                editor.generation_size = maze.width;
                editor.maze = maze;
                format!("Loaded {CUSTOM_LEVEL_FILE}.")
            }
            Ok(_) => "Load failed: malformed maze dimensions.".to_string(),
            Err(error) => format!("Load failed: {error}"),
        };
    }

    draw_text(
        &editor.status,
        810.0_f32.min(screen_width() - 400.0),
        button_y + 30.0,
        18.0,
        LIGHTGRAY,
    );

    if back || is_key_pressed(KeyCode::Escape) {
        let hosts = load_hosts();
        let (server, username) = hosts
            .first()
            .map(|host| (host.address.clone(), host.username.clone()))
            .unwrap_or_else(|| (DEFAULT_SERVER.to_string(), "Agent".to_string()));
        return AppScreen::Connect(ConnectScreen {
            server,
            username,
            alias: String::new(),
            active: ActiveField::Server,
            hosts,
            status: "Returned from level editor.".to_string(),
            pending: None,
        });
    }

    AppScreen::Editor(editor)
}

fn draw_game(game: &GameState) {
    let maze = &game.levels[game.level_index];
    let sw = screen_width();
    let sh = screen_height();
    draw_rectangle(0.0, 0.0, sw, sh * 0.5, Color::new(0.02, 0.04, 0.07, 1.0));
    draw_rectangle(
        0.0,
        sh * 0.5,
        sw,
        sh * 0.5,
        Color::new(0.025, 0.025, 0.03, 1.0),
    );

    let ray_count = ((sw / 2.0) as usize).clamp(240, 800);
    let stripe_width = sw / ray_count as f32 + 0.6;
    let mut z_buffer = vec![0.0f32; ray_count];

    for (index, z) in z_buffer.iter_mut().enumerate() {
        let camera = index as f32 / ray_count as f32 - 0.5;
        let ray_angle = game.angle + camera * FOV;
        let hit = maze.cast_ray(game.x, game.y, ray_angle);
        let corrected = hit.distance * (ray_angle - game.angle).cos().abs().max(0.05);
        *z = corrected;
        let wall_height = (sh / corrected).min(sh * 1.5);
        let brightness = (0.92 / (1.0 + corrected * 0.14)).clamp(0.16, 0.86)
            * if hit.side == 1 { 0.78 } else { 1.0 };
        let x = index as f32 * sw / ray_count as f32;
        draw_rectangle(
            x,
            (sh - wall_height) * 0.5,
            stripe_width,
            wall_height,
            Color::new(brightness * 0.75, brightness * 0.94, brightness, 1.0),
        );
    }

    draw_remote_players(game, &z_buffer, ray_count);

    if game.shot_flash > 0.0 {
        draw_rectangle(
            0.0,
            0.0,
            sw,
            sh,
            Color::new(0.9, 0.95, 1.0, (game.shot_flash * 1.4).min(0.14)),
        );
    }

    draw_crosshair(sw * 0.5, sh * 0.5);
    draw_minimap(game, maze);
    draw_hud(game, maze);
}

fn draw_remote_players(game: &GameState, z_buffer: &[f32], ray_count: usize) {
    let sw = screen_width();
    let sh = screen_height();
    let mut visuals: Vec<&RemoteVisual> = game
        .remotes
        .values()
        .filter(|visual| visual.state.health > 0)
        .collect();
    visuals.sort_by(|left, right| {
        let dl = squared_distance((game.x, game.y), (left.display_x, left.display_y));
        let dr = squared_distance((game.x, game.y), (right.display_x, right.display_y));
        dr.total_cmp(&dl)
    });

    for visual in visuals {
        let dx = visual.display_x - game.x;
        let dy = visual.display_y - game.y;
        let distance = (dx * dx + dy * dy).sqrt().max(0.01);
        let relative = normalize_angle(dy.atan2(dx) - game.angle);
        if relative.abs() > FOV * 0.62 {
            continue;
        }
        let screen_x = sw * 0.5 + (relative / (FOV * 0.5)) * sw * 0.5;
        let ray_index = ((screen_x / sw) * ray_count as f32).floor() as isize;
        if ray_index < 0
            || ray_index >= ray_count as isize
            || distance > z_buffer[ray_index as usize] + 0.25
        {
            continue;
        }

        let size = (sh * 0.64 / distance).clamp(18.0, sh * 0.55);
        let cy = sh * 0.5;
        let eye = if visual.state.bot {
            Color::new(0.94, 0.84, 0.7, 1.0)
        } else {
            WHITE
        };
        draw_circle(screen_x, cy, size * 0.38, eye);
        draw_circle_lines(screen_x, cy, size * 0.38, (size * 0.035).max(2.0), BLACK);
        draw_circle(screen_x, cy, size * 0.15, Color::new(0.2, 0.75, 0.85, 1.0));
        draw_circle(screen_x, cy, size * 0.07, BLACK);
        draw_text(
            &visual.state.name,
            screen_x - size * 0.38,
            cy - size * 0.48,
            (size * 0.11).clamp(12.0, 22.0),
            WHITE,
        );
        let health_w = size * 0.7;
        let health_x = screen_x - health_w * 0.5;
        let health_y = cy + size * 0.48;
        draw_rectangle(health_x, health_y, health_w, 5.0, DARKGRAY);
        draw_rectangle(
            health_x,
            health_y,
            health_w * (visual.state.health.max(0) as f32 / 100.0),
            5.0,
            Color::new(0.3, 0.9, 0.55, 1.0),
        );
    }
}

fn draw_minimap(game: &GameState, maze: &Maze) {
    let max_size = 230.0;
    let cell = (max_size / maze.width.max(maze.height) as f32).max(3.0);
    let width = maze.width as f32 * cell;
    let height = maze.height as f32 * cell;
    let left = 18.0;
    let top = 18.0;

    draw_rectangle(
        left - 8.0,
        top - 8.0,
        width + 16.0,
        height + 16.0,
        Color::new(0.0, 0.0, 0.0, 0.72),
    );
    for y in 0..maze.height {
        for x in 0..maze.width {
            if maze.cells[y * maze.width + x] != 0 {
                draw_rectangle(
                    left + x as f32 * cell,
                    top + y as f32 * cell,
                    cell,
                    cell,
                    Color::new(0.46, 0.58, 0.62, 0.92),
                );
            }
        }
    }

    for visual in game
        .remotes
        .values()
        .filter(|visual| visual.state.health > 0)
    {
        draw_circle(
            left + visual.display_x * cell,
            top + visual.display_y * cell,
            (cell * 0.32).max(2.0),
            if visual.state.bot { ORANGE } else { RED },
        );
    }

    let px = left + game.x * cell;
    let py = top + game.y * cell;
    let dir = vec2(game.angle.cos(), game.angle.sin());
    let right = vec2(-dir.y, dir.x);
    let tip = vec2(px, py) + dir * (cell * 0.75);
    let base = vec2(px, py) - dir * (cell * 0.42);
    draw_triangle(
        tip,
        base + right * cell * 0.42,
        base - right * cell * 0.42,
        YELLOW,
    );
}

fn draw_hud(game: &GameState, maze: &Maze) {
    let sw = screen_width();
    let fps = game.display_fps.round() as i32;
    let fps_color = if fps >= 50 { GREEN } else { RED };
    let x = sw - 280.0;
    draw_rectangle(
        x - 16.0,
        16.0,
        264.0,
        154.0,
        Color::new(0.0, 0.0, 0.0, 0.62),
    );
    draw_text(format!("FPS {:>3}", fps), x, 48.0, 28.0, fps_color);
    draw_text(format!("HP  {:>3}", game.health), x, 78.0, 24.0, WHITE);
    draw_text(format!("SCORE {}", game.score), x, 106.0, 24.0, WHITE);
    draw_text(
        format!("LEVEL {} / 3", game.level_index + 1),
        x,
        134.0,
        21.0,
        SKYBLUE,
    );
    draw_text(
        format!("DEAD ENDS {}", maze.dead_ends()),
        x,
        158.0,
        18.0,
        GRAY,
    );

    let bottom = screen_height() - 24.0;
    draw_text(
        "WASD move  Q/E or arrows turn  SPACE shoot  N next level  ESC disconnect",
        18.0,
        bottom,
        18.0,
        LIGHTGRAY,
    );
    if game.health == 0 {
        let text = "ELIMINATED - respawning...";
        let m = measure_text(text, None, 34, 1.0);
        draw_text(
            text,
            screen_width() * 0.5 - m.width * 0.5,
            screen_height() * 0.66,
            34.0,
            RED,
        );
    }
}

fn draw_crosshair(x: f32, y: f32) {
    draw_line(x - 10.0, y, x - 3.0, y, 2.0, WHITE);
    draw_line(x + 3.0, y, x + 10.0, y, 2.0, WHITE);
    draw_line(x, y - 10.0, x, y - 3.0, 2.0, WHITE);
    draw_line(x, y + 3.0, x, y + 10.0, 2.0, WHITE);
}

fn draw_connect_background() {
    let sw = screen_width();
    let sh = screen_height();
    for i in 0..24 {
        let t = i as f32 / 24.0;
        let x = sw * (0.58 + t * 0.42);
        let perspective = 1.0 - t * 0.85;
        draw_line(
            x,
            0.0,
            x - 180.0 * perspective,
            sh,
            1.0,
            Color::new(0.04, 0.23, 0.28, 0.45),
        );
    }
    for i in 0..16 {
        let y = i as f32 / 16.0 * sh;
        draw_line(sw * 0.58, y, sw, y, 1.0, Color::new(0.04, 0.18, 0.22, 0.35));
    }
}

fn draw_field(label: &str, value: &str, rect: Rect, active: bool) {
    draw_text(
        label,
        rect.x,
        rect.y - 8.0,
        17.0,
        if active { SKYBLUE } else { GRAY },
    );
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.01, 0.018, 0.025, 1.0),
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if active { 2.0 } else { 1.0 },
        if active { SKYBLUE } else { DARKGRAY },
    );
    let cursor = if active && ((get_time() * 2.0) as i32 % 2 == 0) {
        "_"
    } else {
        ""
    };
    draw_text(
        format!("{value}{cursor}"),
        rect.x + 14.0,
        rect.y + 34.0,
        24.0,
        WHITE,
    );
}

fn draw_button(rect: Rect, label: &str) -> bool {
    let hover = point_in_rect(mouse_position(), rect);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if hover {
            Color::new(0.08, 0.34, 0.38, 1.0)
        } else {
            Color::new(0.04, 0.20, 0.24, 1.0)
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.5,
        Color::new(0.25, 0.78, 0.84, 0.9),
    );
    let m = measure_text(label, None, 20, 1.0);
    draw_text(
        label,
        rect.x + (rect.w - m.width) * 0.5,
        rect.y + rect.h * 0.63,
        20.0,
        WHITE,
    );
    hover && is_mouse_button_pressed(MouseButton::Left)
}

fn edit_active_field(screen: &mut ConnectScreen) {
    let target = match screen.active {
        ActiveField::Server => &mut screen.server,
        ActiveField::Username => &mut screen.username,
        ActiveField::Alias => &mut screen.alias,
    };
    while let Some(ch) = get_char_pressed() {
        if !ch.is_control() && target.len() < 64 {
            target.push(ch);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        target.pop();
    }
}

fn start_connection(server: &str, username: &str) -> io::Result<NetworkClient> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let server = server.trim();
    let endpoint = if server.parse::<std::net::IpAddr>().is_ok() {
        format!("{server}:34254")
    } else {
        server.to_string()
    };
    socket.connect(&endpoint)?;
    socket.set_nonblocking(true)?;
    let join = encode_client(&ClientMessage::Join(username.to_string()));
    socket.send(join.as_bytes())?;
    Ok(NetworkClient {
        socket,
        player_id: 0,
        seq: 0,
        last_input_sent: Instant::now(),
        last_ping: Instant::now(),
    })
}

fn poll_network(socket: &UdpSocket) -> Vec<ServerMessage> {
    let mut messages = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        match socket.recv(&mut buf) {
            Ok(amount) => {
                let text = String::from_utf8_lossy(&buf[..amount]);
                if let Some(message) = parse_server(&text) {
                    messages.push(message);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }
    messages
}

fn axis(positive: KeyCode, negative: KeyCode) -> f32 {
    let positive = if is_key_down(positive) { 1.0 } else { 0.0 };
    let negative = if is_key_down(negative) { 1.0 } else { 0.0 };
    positive - negative
}

fn clicked(rect: Rect) -> bool {
    point_in_rect(mouse_position(), rect) && is_mouse_button_pressed(MouseButton::Left)
}

fn point_in_rect(point: (f32, f32), rect: Rect) -> bool {
    point.0 >= rect.x
        && point.0 <= rect.x + rect.w
        && point.1 >= rect.y
        && point.1 <= rect.y + rect.h
}

fn load_hosts() -> Vec<HostEntry> {
    fs::read_to_string(HOSTS_FILE)
        .ok()
        .and_then(|data| serde_json::from_str(&data).ok())
        .unwrap_or_default()
}

fn save_hosts(hosts: &[HostEntry]) {
    if let Ok(json) = serde_json::to_string_pretty(hosts) {
        let _ = fs::write(HOSTS_FILE, json);
    }
}

fn remember_host(hosts: &mut Vec<HostEntry>, alias: &str, address: &str, username: &str) {
    let alias = if alias.trim().is_empty() {
        address.trim().to_string()
    } else {
        alias.trim().to_string()
    };
    let entry = HostEntry {
        alias,
        address: address.trim().to_string(),
        username: username.trim().to_string(),
    };
    hosts.retain(|host| host.address != entry.address);
    hosts.insert(0, entry);
    hosts.truncate(8);
}

fn time_seed() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(1)
}

fn squared_distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    dx * dx + dy * dy
}
