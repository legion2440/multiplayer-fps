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

fn draw_text<T: AsRef<str>>(
    text: T,
    x: f32,
    y: f32,
    font_size: f32,
    color: Color,
) -> TextDimensions {
    macroquad::prelude::draw_text(text.as_ref(), x, y, font_size, color)
}

const DEFAULT_SERVER: &str = "127.0.0.1:34254";
const HOSTS_FILE: &str = "hosts.json";
const CUSTOM_LEVEL_FILE: &str = "custom_level.json";
const INPUT_SEND_INTERVAL: Duration = Duration::from_millis(33);
const PING_INTERVAL: Duration = Duration::from_secs(1);
const MOUSE_RADIANS_PER_PIXEL: f32 = 0.0025;

fn window_conf() -> Conf {
    Conf {
        window_title: "Maze Wars 3D - Multiplayer FPS".to_string(),
        window_width: 1280,
        window_height: 800,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisualTheme {
    Classic,
    Cyberpunk,
    Green,
    Amber,
}

impl VisualTheme {
    fn label(self) -> &'static str {
        match self {
            Self::Classic => "Classic 1974",
            Self::Cyberpunk => "Cyberpunk Neon",
            Self::Green => "Phosphor Green",
            Self::Amber => "Amber Mono",
        }
    }
}

#[derive(Debug, Clone)]
struct GameSettings {
    theme: VisualTheme,
    show_minimap: bool,
    show_rays: bool,
    crt_effect: bool,
    mouse_sensitivity: f32,
    fov: f32,
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            theme: VisualTheme::Classic,
            show_minimap: true,
            show_rays: true,
            crt_effect: false,
            mouse_sensitivity: 1.0,
            fov: 1.15,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Overlay {
    None,
    Scoreboard,
    Settings,
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
    custom_level: Option<Maze>,
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

#[derive(Debug, Clone)]
struct KillFeedEntry {
    killer: String,
    victim: String,
    expires_at: Instant,
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
    damage_flash: f32,
    status: String,
    display_fps: f32,
    display_frame_ms: f32,
    ping_ms: u32,
    ping_sent: Option<Instant>,
    cursor_grabbed: bool,
    settings: GameSettings,
    overlay: Overlay,
    level_resync_sent: Option<Instant>,
    last_health: i32,
    last_hit_by: HashMap<u32, u32>,
    kill_feed: Vec<KillFeedEntry>,
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

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct Palette {
    bg: Color,
    panel: Color,
    panel_alt: Color,
    border: Color,
    accent: Color,
    accent_alt: Color,
    text: Color,
    muted: Color,
    sky: Color,
    floor: Color,
    wall: Color,
    wall_side: Color,
    health: Color,
    danger: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopAction {
    Level(usize),
    Procedural,
    Gateway,
    Editor,
    Scoreboard,
    Settings,
}

#[macroquad::main(window_conf)]
async fn main() {
    let hosts = load_hosts();
    let (server, username) = hosts
        .first()
        .map(|host| (host.address.clone(), host.username.clone()))
        .unwrap_or_else(|| (DEFAULT_SERVER.to_string(), "Pilot_01".to_string()));

    let mut screen = AppScreen::Connect(ConnectScreen {
        server,
        username,
        alias: String::new(),
        active: ActiveField::Server,
        hosts,
        status: "Enter a UDP server endpoint and pilot call sign.".to_string(),
        pending: None,
    });

    set_cursor_grab(false);
    show_mouse(true);

    loop {
        clear_background(Color::new(0.008, 0.011, 0.02, 1.0));
        screen = match screen {
            AppScreen::Connect(connect) => update_connect(connect),
            AppScreen::Game(game) => update_game(game),
            AppScreen::Editor(editor) => update_editor(editor),
        };
        next_frame().await;
    }
}

fn update_connect(mut screen: ConnectScreen) -> AppScreen {
    set_cursor_grab(false);
    show_mouse(true);
    draw_connect_background();

    if is_key_pressed(KeyCode::Tab) {
        screen.active = match screen.active {
            ActiveField::Server => ActiveField::Username,
            ActiveField::Username => ActiveField::Alias,
            ActiveField::Alias => ActiveField::Server,
        };
    }
    edit_active_field(&mut screen);

    let panel_w = 650.0_f32.min(screen_width() - 40.0);
    let panel_h = 680.0_f32.min(screen_height() - 36.0);
    let panel_x = (screen_width() - panel_w) * 0.5;
    let panel_y = (screen_height() - panel_h) * 0.5;
    let panel = Rect::new(panel_x, panel_y, panel_w, panel_h);

    draw_shadowed_panel(panel, Color::new(0.025, 0.035, 0.065, 0.98), SKYBLUE);

    let icon = Rect::new(panel.x + 26.0, panel.y + 24.0, 48.0, 48.0);
    draw_rectangle(
        icon.x,
        icon.y,
        icon.w,
        icon.h,
        Color::new(0.03, 0.28, 0.34, 1.0),
    );
    draw_rectangle_lines(
        icon.x,
        icon.y,
        icon.w,
        icon.h,
        1.0,
        Color::new(0.2, 0.8, 0.9, 0.8),
    );
    draw_centered("MW", icon, 21.0, Color::new(0.65, 0.96, 1.0, 1.0));

    draw_text(
        "01-EDU HOST MANAGER & GATEWAY",
        panel.x + 92.0,
        panel.y + 47.0,
        24.0,
        WHITE,
    );
    draw_text(
        "Native UDP client / saved host aliases / server AI",
        panel.x + 92.0,
        panel.y + 69.0,
        15.0,
        GRAY,
    );
    draw_line(
        panel.x,
        panel.y + 92.0,
        panel.x + panel.w,
        panel.y + 92.0,
        1.0,
        Color::new(0.16, 0.22, 0.32, 1.0),
    );

    let server_rect = Rect::new(panel.x + 28.0, panel.y + 130.0, panel.w - 56.0, 46.0);
    let user_rect = Rect::new(panel.x + 28.0, panel.y + 212.0, panel.w - 56.0, 46.0);
    let alias_rect = Rect::new(panel.x + 28.0, panel.y + 294.0, panel.w - 56.0, 42.0);
    draw_field(
        "IP ADDRESS & UDP PORT",
        &screen.server,
        server_rect,
        screen.active == ActiveField::Server,
    );
    draw_field(
        "PILOT CALL SIGN",
        &screen.username,
        user_rect,
        screen.active == ActiveField::Username,
    );
    draw_field(
        "OPTIONAL HOST ALIAS",
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

    let network_card = Rect::new(
        panel.x + 28.0,
        panel.y + 362.0,
        (panel.w - 68.0) * 0.5,
        74.0,
    );
    let ai_card = Rect::new(
        network_card.x + network_card.w + 12.0,
        network_card.y,
        network_card.w,
        network_card.h,
    );
    draw_status_card(
        network_card,
        "UDP NETWORK",
        "Real client-server transport",
        SKYBLUE,
    );
    draw_status_card(
        ai_card,
        "SERVER AI READY",
        "Bots stay authoritative",
        Color::new(0.35, 0.95, 0.65, 1.0),
    );

    draw_text(
        "SAVED HOSTS & QUICK RECONNECT",
        panel.x + 28.0,
        panel.y + 468.0,
        14.0,
        GRAY,
    );

    let mut chosen_host = None;
    for (index, host) in screen.hosts.iter().take(4).enumerate() {
        let y = panel.y + 482.0 + index as f32 * 34.0;
        let rect = Rect::new(panel.x + 28.0, y, panel.w - 56.0, 29.0);
        let hover = point_in_rect(mouse_position(), rect);
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if hover {
                Color::new(0.06, 0.11, 0.17, 1.0)
            } else {
                Color::new(0.035, 0.055, 0.09, 1.0)
            },
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::new(0.12, 0.18, 0.25, 1.0),
        );
        let label = format!("{}   {}   [{}]", host.alias, host.address, host.username);
        draw_text(
            shorten(&label, 58),
            rect.x + 9.0,
            rect.y + 20.0,
            15.0,
            if hover { WHITE } else { LIGHTGRAY },
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

    let connect_rect = Rect::new(
        panel.x + 28.0,
        panel.y + panel.h - 74.0,
        panel.w - 220.0,
        46.0,
    );
    let editor_rect = Rect::new(
        panel.x + panel.w - 180.0,
        panel.y + panel.h - 74.0,
        152.0,
        46.0,
    );
    let connect_clicked = draw_gradient_like_button(
        connect_rect,
        if screen.pending.is_some() {
            "CONNECTING..."
        } else {
            "ENTER THE MAZE"
        },
        SKYBLUE,
    );
    let editor_clicked = draw_dark_button(editor_rect, "LEVEL EDITOR", false);

    draw_text(
        shorten(&screen.status, 72),
        panel.x + 28.0,
        panel.y + panel.h - 88.0,
        14.0,
        Color::new(0.65, 0.72, 0.82, 1.0),
    );

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
                    custom_level: None,
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
                ServerMessage::CustomLevel(maze) => pending.custom_level = Some(maze),
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
            let mut levels = builtin_levels();
            if let Some(custom) = pending.custom_level.take() {
                levels.push(custom);
            }
            let initial_level = if level < levels.len() {
                level
            } else {
                level.min(2)
            };
            return AppScreen::Game(GameState {
                server: normalized_server(&screen.server),
                username: screen.username.clone(),
                network: pending.network,
                levels,
                level_index: initial_level,
                x,
                y,
                angle: 0.0,
                health: 100,
                score: 0,
                remotes: HashMap::new(),
                last_server_tick: 0,
                shot_flash: 0.0,
                damage_flash: 0.0,
                status: "Connected".to_string(),
                display_fps: 60.0,
                display_frame_ms: 16.7,
                ping_ms: 0,
                ping_sent: None,
                cursor_grabbed: false,
                settings: GameSettings::default(),
                overlay: Overlay::None,
                level_resync_sent: None,
                last_health: 100,
                last_hit_by: HashMap::new(),
                kill_feed: Vec::new(),
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
    let alpha = 1.0 - (-dt / 0.5).exp();
    game.display_fps += (instant_fps - game.display_fps) * alpha;
    game.display_frame_ms += (dt * 1000.0 - game.display_frame_ms) * alpha;
    game.shot_flash = (game.shot_flash - dt).max(0.0);
    game.damage_flash = (game.damage_flash - dt).max(0.0);
    game.kill_feed
        .retain(|entry| Instant::now() < entry.expires_at);

    if is_key_pressed(KeyCode::Tab) {
        game.overlay = if game.overlay == Overlay::Scoreboard {
            Overlay::None
        } else {
            release_cursor(&mut game);
            Overlay::Scoreboard
        };
    }

    if is_key_pressed(KeyCode::Escape) {
        if game.overlay != Overlay::None {
            game.overlay = Overlay::None;
        } else if game.cursor_grabbed {
            release_cursor(&mut game);
        } else {
            disconnect(&game);
            return connect_after_game(&game, "Disconnected.");
        }
    }

    if game.overlay == Overlay::Settings {
        handle_settings_input(&mut game);
    } else if game.overlay == Overlay::Scoreboard {
        if is_mouse_button_pressed(MouseButton::Left)
            && point_in_rect(mouse_position(), scoreboard_close_rect())
        {
            game.overlay = Overlay::None;
        }
    } else if !game.cursor_grabbed {
        if let Some(action) = top_action_at_mouse(game.level_index) {
            match action {
                TopAction::Level(level) => {
                    let _ = game
                        .network
                        .socket
                        .send(encode_client(&ClientMessage::SetLevel(level)).as_bytes());
                }
                TopAction::Procedural => {
                    let maze = Maze::generated("Procedural Maze", "Generated", 21, 21, time_seed());
                    request_custom_level(&mut game, maze, "Procedural level requested.");
                }
                TopAction::Gateway => {
                    game.status =
                        "Gateway disabled during a live match. Press Esc twice to disconnect."
                            .to_string();
                }
                TopAction::Editor => match load_custom_level_file() {
                    Ok(maze) => {
                        request_custom_level(&mut game, maze, "Saved custom level requested.");
                    }
                    Err(error) => {
                        game.status = format!(
                            "Custom level unavailable: {error}. Edit and save it from the gateway."
                        );
                    }
                },
                TopAction::Scoreboard => {
                    game.overlay = Overlay::Scoreboard;
                }
                TopAction::Settings => {
                    game.overlay = Overlay::Settings;
                }
            }
        } else if is_mouse_button_pressed(MouseButton::Left)
            && point_in_rect(mouse_position(), game_view_rect())
        {
            grab_cursor(&mut game);
        }
    }

    let controls_enabled = game.overlay == Overlay::None;
    let mut forward = 0.0;
    let mut strafe = 0.0;
    let mut turn = 0.0;
    let mut wants_fire = false;
    if controls_enabled {
        forward = axis(KeyCode::W, KeyCode::S);
        strafe = axis(KeyCode::D, KeyCode::A);
        turn =
            (axis(KeyCode::Right, KeyCode::Left) + axis(KeyCode::E, KeyCode::Q)).clamp(-1.0, 1.0);
        if game.cursor_grabbed {
            let mouse = mouse_delta_position();
            let mouse_pixels_x = mouse.x * screen_width() * 0.5;
            let look_delta =
                -mouse_pixels_x * MOUSE_RADIANS_PER_PIXEL * game.settings.mouse_sensitivity;
            if look_delta.is_finite() {
                game.angle = normalize_angle(game.angle + look_delta);
            }
        }

        let maze = &game.levels[game.level_index];
        move_entity(
            maze,
            &mut game.x,
            &mut game.y,
            &mut game.angle,
            forward,
            strafe,
            turn,
            dt,
        );

        let mouse_fire = game.cursor_grabbed && is_mouse_button_pressed(MouseButton::Left);
        wants_fire = is_key_pressed(KeyCode::Space) || mouse_fire;
    }

    if game.network.last_input_sent.elapsed() >= INPUT_SEND_INTERVAL || wants_fire {
        game.network.seq = game.network.seq.wrapping_add(1);
        let packet = encode_client(&ClientMessage::Input {
            seq: game.network.seq,
            forward,
            strafe,
            turn,
            angle: game.angle,
        });
        if game.network.socket.send(packet.as_bytes()).is_ok() {
            game.network.last_input_sent = Instant::now();
        }
    }

    if wants_fire {
        fire_weapon(&mut game);
    }

    if is_key_pressed(KeyCode::N) && controls_enabled {
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
        game.ping_sent = Some(Instant::now());
    }

    let messages = poll_network(&game.network.socket);
    for message in messages {
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
                if level < game.levels.len() {
                    game.level_resync_sent = None;
                    if level != game.level_index {
                        game.level_index = level;
                        game.remotes.clear();
                    }
                } else {
                    request_level_resync(&mut game);
                }

                let mut seen = Vec::with_capacity(players.len());
                let mut deaths = Vec::new();
                for player in players {
                    if player.id == game.network.player_id {
                        if game.last_health > 0 && player.health == 0 {
                            deaths.push((player.id, game.username.clone()));
                        }
                        if player.health < game.health {
                            game.damage_flash = 0.22;
                        }
                        game.last_health = player.health;
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
                    } else {
                        seen.push(player.id);
                        let old_health = game
                            .remotes
                            .get(&player.id)
                            .map(|visual| visual.state.health)
                            .unwrap_or(player.health);
                        if old_health > 0 && player.health == 0 {
                            deaths.push((player.id, player.name.clone()));
                        }
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
                for (victim_id, victim_name) in deaths {
                    record_death(&mut game, victim_id, victim_name);
                }
            }
            ServerMessage::Shot { shooter, target } => {
                if let Some(target_id) = target {
                    game.last_hit_by.insert(target_id, shooter);
                    if target_id == game.network.player_id {
                        game.damage_flash = 0.16;
                    }
                }
                if shooter == game.network.player_id {
                    game.shot_flash = 0.12;
                }
            }
            ServerMessage::Level(level) => {
                if level < game.levels.len() {
                    game.level_resync_sent = None;
                    game.level_index = level;
                    game.remotes.clear();
                    game.status = format!("Level changed to {}", level + 1);
                } else {
                    request_level_resync(&mut game);
                }
            }
            ServerMessage::CustomLevel(maze) => {
                const CUSTOM_INDEX: usize = 3;
                if game.levels.len() == CUSTOM_INDEX {
                    game.levels.push(maze);
                } else if game.levels.len() > CUSTOM_INDEX {
                    game.levels[CUSTOM_INDEX] = maze;
                }
                if game.levels.len() > CUSTOM_INDEX {
                    game.level_index = CUSTOM_INDEX;
                    game.level_resync_sent = None;
                    game.remotes.clear();
                    game.status = "Custom level activated.".to_string();
                }
            }
            ServerMessage::Pong => {
                if let Some(sent) = game.ping_sent.take() {
                    game.ping_ms = sent.elapsed().as_millis().min(u128::from(u32::MAX)) as u32;
                }
            }
            ServerMessage::Reject(reason) => game.status = reason,
            ServerMessage::Welcome { .. } => {}
        }
    }

    let smoothing = 1.0 - (-12.0 * dt).exp();
    for visual in game.remotes.values_mut() {
        visual.display_x += (visual.state.x - visual.display_x) * smoothing;
        visual.display_y += (visual.state.y - visual.display_y) * smoothing;
    }

    draw_game(&game);
    AppScreen::Game(game)
}

fn update_editor(mut editor: EditorState) -> AppScreen {
    set_cursor_grab(false);
    show_mouse(true);
    let palette = palette(VisualTheme::Classic);
    clear_background(palette.bg);

    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        78.0,
        Color::new(0.015, 0.022, 0.04, 1.0),
    );
    draw_line(0.0, 78.0, screen_width(), 78.0, 1.0, palette.border);
    draw_text("MW", 24.0, 49.0, 28.0, palette.accent);
    draw_text("MAZE WARS 3D", 78.0, 36.0, 25.0, WHITE);
    draw_text("VISUAL MAZE EDITOR", 78.0, 57.0, 14.0, palette.muted);

    let top = 104.0;
    let left = 34.0;
    let available_w = screen_width() - 68.0;
    let available_h = screen_height() - 220.0;
    let cell = (available_w / editor.maze.width as f32)
        .min(available_h / editor.maze.height as f32)
        .floor()
        .max(4.0);
    let grid_w = cell * editor.maze.width as f32;
    let grid_h = cell * editor.maze.height as f32;

    draw_rectangle(
        left - 8.0,
        top - 8.0,
        grid_w + 16.0,
        grid_h + 16.0,
        palette.panel,
    );
    draw_rectangle_lines(
        left - 8.0,
        top - 8.0,
        grid_w + 16.0,
        grid_h + 16.0,
        1.0,
        palette.border,
    );

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
                    palette.wall
                } else {
                    Color::new(0.02, 0.04, 0.06, 1.0)
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

    let button_y = screen_height() - 80.0;
    let generate = draw_dark_button(
        Rect::new(34.0, button_y, 156.0, 44.0),
        "GENERATE [G]",
        false,
    );
    let size = draw_dark_button(
        Rect::new(202.0, button_y, 138.0, 44.0),
        &format!("SIZE {}", editor.generation_size),
        false,
    );
    let save = draw_dark_button(Rect::new(352.0, button_y, 118.0, 44.0), "SAVE [S]", false);
    let load = draw_dark_button(Rect::new(482.0, button_y, 118.0, 44.0), "LOAD [L]", false);
    let back = draw_dark_button(Rect::new(612.0, button_y, 138.0, 44.0), "BACK [ESC]", false);

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
        if !editor.maze.is_connected() {
            editor.status = "Save refused: maze must have one connected open region.".to_string();
        } else {
            editor.status = match serde_json::to_string_pretty(&editor.maze)
                .map_err(io::Error::other)
                .and_then(|json| fs::write(CUSTOM_LEVEL_FILE, json))
            {
                Ok(()) => format!("Saved {CUSTOM_LEVEL_FILE}."),
                Err(error) => format!("Save failed: {error}"),
            };
        }
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
        shorten(&editor.status, 68),
        775.0_f32.min(screen_width() - 430.0),
        button_y + 29.0,
        16.0,
        LIGHTGRAY,
    );

    if back || is_key_pressed(KeyCode::Escape) {
        return default_connect_screen("Returned from level editor.");
    }

    AppScreen::Editor(editor)
}

fn draw_game(game: &GameState) {
    let palette = palette(game.settings.theme);
    clear_background(palette.bg);
    draw_header(game, palette);
    draw_info_bar(game, palette);

    let view = game_view_rect();
    draw_rectangle(
        view.x - 1.0,
        view.y - 1.0,
        view.w + 2.0,
        view.h + 2.0,
        palette.border,
    );
    draw_rectangle(view.x, view.y, view.w, view.h, BLACK);
    draw_world(game, view, palette);
    draw_feature_cards(palette);

    match game.overlay {
        Overlay::None => {}
        Overlay::Scoreboard => draw_scoreboard(game, palette),
        Overlay::Settings => draw_settings(game, palette),
    }
}

fn draw_world(game: &GameState, view: Rect, palette: Palette) {
    let maze = &game.levels[game.level_index];
    draw_rectangle(view.x, view.y, view.w, view.h * 0.5, palette.sky);
    draw_rectangle(
        view.x,
        view.y + view.h * 0.5,
        view.w,
        view.h * 0.5,
        palette.floor,
    );

    let ray_count = ((view.w / 2.0) as usize).clamp(240, 800);
    let stripe_width = view.w / ray_count as f32 + 0.7;
    let mut z_buffer = vec![0.0f32; ray_count];

    for (index, z) in z_buffer.iter_mut().enumerate() {
        let camera = index as f32 / ray_count as f32 - 0.5;
        let ray_angle = game.angle + camera * game.settings.fov;
        let hit = maze.cast_ray(game.x, game.y, ray_angle);
        let corrected = hit.distance * (ray_angle - game.angle).cos().abs().max(0.05);
        *z = corrected;
        let wall_height = (view.h / corrected).min(view.h * 1.6);
        let fade = (1.0 / (1.0 + corrected * 0.11)).clamp(0.18, 0.95);
        let base = if hit.side == 1 {
            palette.wall_side
        } else {
            palette.wall
        };
        let wall_color = Color::new(base.r * fade, base.g * fade, base.b * fade, 1.0);
        let x = view.x + index as f32 * view.w / ray_count as f32;
        draw_rectangle(
            x,
            view.y + (view.h - wall_height) * 0.5,
            stripe_width,
            wall_height,
            wall_color,
        );
    }

    draw_remote_players(game, view, &z_buffer, ray_count, palette);

    if game.shot_flash > 0.0 {
        draw_rectangle(
            view.x,
            view.y,
            view.w,
            view.h,
            Color::new(0.65, 0.9, 1.0, (game.shot_flash * 1.2).min(0.14)),
        );
    }
    if game.damage_flash > 0.0 {
        draw_rectangle(
            view.x,
            view.y,
            view.w,
            view.h,
            Color::new(0.9, 0.06, 0.08, (game.damage_flash * 1.1).min(0.22)),
        );
        draw_rectangle_lines(
            view.x + 2.0,
            view.y + 2.0,
            view.w - 4.0,
            view.h - 4.0,
            8.0,
            Color::new(0.8, 0.0, 0.05, 0.3),
        );
    }

    draw_weapon(game, view, palette);
    draw_crosshair(view.x + view.w * 0.5, view.y + view.h * 0.5, palette.accent);
    draw_hud(game, maze, view, palette);
    if game.settings.show_minimap {
        draw_minimap(game, maze, view, palette);
    }
    if game.settings.crt_effect {
        draw_crt(view);
    }

    if game.health == 0 {
        draw_rectangle(
            view.x,
            view.y,
            view.w,
            view.h,
            Color::new(0.18, 0.0, 0.02, 0.78),
        );
        let title = "HULL INTEGRITY COMPROMISED";
        let m = measure_text(title, None, 34, 1.0);
        draw_text(
            title,
            view.x + view.w * 0.5 - m.width * 0.5,
            view.y + view.h * 0.48,
            34.0,
            palette.danger,
        );
        let sub = "Reconstructing optic telemetry...";
        let sm = measure_text(sub, None, 18, 1.0);
        draw_text(
            sub,
            view.x + view.w * 0.5 - sm.width * 0.5,
            view.y + view.h * 0.55,
            18.0,
            LIGHTGRAY,
        );
    }

    if !game.cursor_grabbed && game.overlay == Overlay::None {
        let hint = "CLICK VIEWPORT TO LOCK MOUSE";
        let m = measure_text(hint, None, 16, 1.0);
        let rect = Rect::new(
            view.x + view.w * 0.5 - m.width * 0.5 - 14.0,
            view.y + 14.0,
            m.width + 28.0,
            30.0,
        );
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.0, 0.0, 0.0, 0.65),
        );
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 1.0, palette.border);
        draw_text(hint, rect.x + 14.0, rect.y + 21.0, 16.0, palette.accent);
    }
}

fn draw_remote_players(
    game: &GameState,
    view: Rect,
    z_buffer: &[f32],
    ray_count: usize,
    palette: Palette,
) {
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
        if relative.abs() > game.settings.fov * 0.62 {
            continue;
        }
        let screen_x =
            view.x + view.w * 0.5 + (relative / (game.settings.fov * 0.5)) * view.w * 0.5;
        if screen_x < view.x || screen_x > view.x + view.w {
            continue;
        }
        let ray_index = (((screen_x - view.x) / view.w) * ray_count as f32).floor() as isize;
        if ray_index < 0
            || ray_index >= ray_count as isize
            || distance > z_buffer[ray_index as usize] + 0.25
        {
            continue;
        }

        let size = (view.h * 0.64 / distance).clamp(18.0, view.h * 0.55);
        let cy = view.y + view.h * 0.5;
        let eye = if visual.state.bot {
            Color::new(0.96, 0.83, 0.67, 1.0)
        } else {
            WHITE
        };
        draw_circle(screen_x, cy, size * 0.38, eye);
        draw_circle_lines(screen_x, cy, size * 0.38, (size * 0.035).max(2.0), BLACK);
        draw_circle(screen_x, cy, size * 0.15, palette.accent);
        let look_dx = (visual.state.angle - game.angle).sin() * size * 0.035;
        draw_circle(screen_x + look_dx, cy, size * 0.07, BLACK);
        draw_text(
            shorten(&visual.state.name, 16),
            screen_x - size * 0.38,
            cy - size * 0.48,
            (size * 0.11).clamp(12.0, 22.0),
            WHITE,
        );
        if visual.state.bot {
            draw_text(
                "BOT",
                screen_x + size * 0.18,
                cy - size * 0.47,
                (size * 0.08).clamp(10.0, 16.0),
                palette.danger,
            );
        }
        let health_w = size * 0.7;
        let health_x = screen_x - health_w * 0.5;
        let health_y = cy + size * 0.48;
        draw_rectangle(health_x, health_y, health_w, 5.0, DARKGRAY);
        draw_rectangle(
            health_x,
            health_y,
            health_w * (visual.state.health.max(0) as f32 / 100.0),
            5.0,
            palette.health,
        );
    }
}

fn draw_hud(game: &GameState, maze: &Maze, view: Rect, palette: Palette) {
    let panel_x = view.x + 16.0;
    let panel_y = view.y + 16.0;
    let server_rect = Rect::new(panel_x, panel_y, 190.0, 64.0);
    draw_hud_panel(server_rect, palette);
    draw_text(
        "SERVER",
        server_rect.x + 11.0,
        server_rect.y + 18.0,
        12.0,
        palette.muted,
    );
    draw_text(
        shorten(&game.server, 23),
        server_rect.x + 11.0,
        server_rect.y + 38.0,
        15.0,
        palette.accent,
    );
    draw_text(
        format!("PING  {} ms", game.ping_ms),
        server_rect.x + 11.0,
        server_rect.y + 56.0,
        12.0,
        palette.health,
    );

    let fps_rect = Rect::new(panel_x, panel_y + 72.0, 190.0, 62.0);
    draw_hud_panel(fps_rect, palette);
    let fps = game.display_fps.round() as i32;
    let fps_color = if fps >= 50 {
        palette.health
    } else {
        palette.danger
    };
    draw_text(
        format!("{} FPS", fps),
        fps_rect.x + 11.0,
        fps_rect.y + 24.0,
        20.0,
        fps_color,
    );
    draw_text(
        format!("{:.1} ms", game.display_frame_ms),
        fps_rect.x + 92.0,
        fps_rect.y + 24.0,
        13.0,
        palette.muted,
    );
    draw_text(
        if fps >= 50 {
            "01-EDU REQ >50 FPS"
        } else {
            "FPS BELOW REQUIREMENT"
        },
        fps_rect.x + 11.0,
        fps_rect.y + 48.0,
        11.0,
        fps_color,
    );

    let angle_deg = (game.angle * 180.0 / PI + 360.0) % 360.0;
    let compass = if !(45.0..315.0).contains(&angle_deg) {
        "E"
    } else if angle_deg < 135.0 {
        "S"
    } else if angle_deg < 225.0 {
        "W"
    } else {
        "N"
    };
    let compass_text = format!("{}  {:03.0} deg", compass, angle_deg);
    let cm = measure_text(&compass_text, None, 15, 1.0);
    let compass_rect = Rect::new(
        view.x + view.w * 0.5 - cm.width * 0.5 - 16.0,
        view.y + 14.0,
        cm.width + 32.0,
        30.0,
    );
    draw_hud_panel(compass_rect, palette);
    draw_text(
        compass_text,
        compass_rect.x + 16.0,
        compass_rect.y + 21.0,
        15.0,
        LIGHTGRAY,
    );

    let hull = Rect::new(view.x + 16.0, view.y + view.h - 76.0, 245.0, 52.0);
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
    );

    let score = Rect::new(view.x + view.w - 172.0, view.y + view.h - 94.0, 156.0, 70.0);
    draw_hud_panel(score, palette);
    draw_text(
        shorten(&game.username, 16),
        score.x + 12.0,
        score.y + 18.0,
        11.0,
        palette.muted,
    );
    draw_text(
        format!("{}", game.score),
        score.x + 12.0,
        score.y + 46.0,
        25.0,
        WHITE,
    );
    draw_text("FRAGS", score.x + 72.0, score.y + 45.0, 11.0, palette.muted);

    let helper = "WASD Move   Mouse / arrows Turn   Space / Click Fire   TAB Leaderboard";
    let hm = measure_text(helper, None, 11, 1.0);
    let helper_rect = Rect::new(
        view.x + view.w * 0.5 - hm.width * 0.5 - 14.0,
        view.y + view.h - 28.0,
        hm.width + 28.0,
        20.0,
    );
    draw_rectangle(
        helper_rect.x,
        helper_rect.y,
        helper_rect.w,
        helper_rect.h,
        Color::new(0.0, 0.0, 0.0, 0.72),
    );
    draw_rectangle_lines(
        helper_rect.x,
        helper_rect.y,
        helper_rect.w,
        helper_rect.h,
        1.0,
        Color::new(0.1, 0.16, 0.23, 1.0),
    );
    draw_text(
        helper,
        helper_rect.x + 14.0,
        helper_rect.y + 14.0,
        11.0,
        palette.muted,
    );

    let mut y = view.y + 160.0;
    for entry in game.kill_feed.iter().rev().take(4) {
        let text = format!("{} > {}", entry.killer, entry.victim);
        let m = measure_text(&text, None, 12, 1.0);
        let rect = Rect::new(view.x + view.w - m.width - 28.0, y, m.width + 18.0, 23.0);
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.0, 0.0, 0.0, 0.68),
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::new(0.1, 0.15, 0.22, 1.0),
        );
        draw_text(text, rect.x + 9.0, rect.y + 16.0, 12.0, LIGHTGRAY);
        y += 27.0;
    }

    let level_note = format!(
        "L{} / {}   {} dead ends",
        game.level_index + 1,
        game.levels.len(),
        maze.dead_ends()
    );
    draw_text(
        level_note,
        view.x + 16.0,
        view.y + view.h - 114.0,
        11.0,
        palette.muted,
    );
}

fn draw_minimap(game: &GameState, maze: &Maze, view: Rect, palette: Palette) {
    let size = 180.0;
    let left = view.x + view.w - size - 16.0;
    let top = view.y + 16.0;
    let sx = size / maze.width as f32;
    let sy = size / maze.height as f32;

    draw_rectangle(left, top, size, size, Color::new(0.01, 0.025, 0.055, 0.94));
    for y in 0..maze.height {
        for x in 0..maze.width {
            if maze.cells[y * maze.width + x] != 0 {
                draw_rectangle(
                    left + x as f32 * sx,
                    top + y as f32 * sy,
                    sx.ceil(),
                    sy.ceil(),
                    Color::new(0.18, 0.25, 0.34, 1.0),
                );
            }
        }
    }

    for visual in game
        .remotes
        .values()
        .filter(|visual| visual.state.health > 0)
    {
        let c = if visual.state.bot {
            palette.danger
        } else {
            palette.accent_alt
        };
        let px = left + visual.display_x * sx;
        let py = top + visual.display_y * sy;
        draw_circle(px, py, 3.0, c);
        draw_line(
            px,
            py,
            px + visual.state.angle.cos() * 7.0,
            py + visual.state.angle.sin() * 7.0,
            1.0,
            c,
        );
    }

    let px = left + game.x * sx;
    let py = top + game.y * sy;
    if game.settings.show_rays {
        draw_line(
            px,
            py,
            px + (game.angle - game.settings.fov * 0.5).cos() * 26.0,
            py + (game.angle - game.settings.fov * 0.5).sin() * 26.0,
            1.0,
            Color::new(palette.accent.r, palette.accent.g, palette.accent.b, 0.35),
        );
        draw_line(
            px,
            py,
            px + (game.angle + game.settings.fov * 0.5).cos() * 26.0,
            py + (game.angle + game.settings.fov * 0.5).sin() * 26.0,
            1.0,
            Color::new(palette.accent.r, palette.accent.g, palette.accent.b, 0.35),
        );
    }
    draw_circle(px, py, 4.0, palette.accent);
    draw_line(
        px,
        py,
        px + game.angle.cos() * 9.0,
        py + game.angle.sin() * 9.0,
        1.4,
        WHITE,
    );
    draw_rectangle_lines(left, top, size, size, 1.5, palette.border);
}

fn draw_weapon(game: &GameState, view: Rect, palette: Palette) {
    let bob = if is_key_down(KeyCode::W)
        || is_key_down(KeyCode::S)
        || is_key_down(KeyCode::A)
        || is_key_down(KeyCode::D)
    {
        (get_time() as f32 * 7.0).sin() * 4.0
    } else {
        0.0
    };
    let cx = view.x + view.w * 0.5;
    let bottom = view.y + view.h - 30.0 + bob;
    if game.shot_flash > 0.0 {
        draw_circle(cx, bottom - 124.0, 28.0, Color::new(0.45, 0.9, 1.0, 0.38));
        draw_circle(cx, bottom - 124.0, 12.0, Color::new(0.8, 1.0, 1.0, 0.85));
    }
    draw_rectangle(
        cx - 28.0,
        bottom - 112.0,
        56.0,
        112.0,
        Color::new(0.035, 0.05, 0.075, 0.98),
    );
    draw_rectangle(
        cx - 24.0,
        bottom - 108.0,
        48.0,
        16.0,
        Color::new(0.18, 0.22, 0.29, 1.0),
    );
    draw_rectangle(cx - 12.0, bottom - 104.0, 24.0, 10.0, palette.accent);
    draw_rectangle(
        cx - 20.0,
        bottom - 58.0,
        40.0,
        7.0,
        Color::new(0.01, 0.015, 0.025, 1.0),
    );
    draw_rectangle(
        cx - 22.0,
        bottom - 29.0,
        44.0,
        20.0,
        Color::new(0.02, 0.03, 0.05, 1.0),
    );
    draw_rectangle_lines(cx - 22.0, bottom - 29.0, 44.0, 20.0, 1.0, palette.border);
    draw_text("MK-IV", cx - 17.0, bottom - 15.0, 11.0, palette.accent);
    draw_rectangle_lines(
        cx - 28.0,
        bottom - 112.0,
        56.0,
        112.0,
        2.0,
        Color::new(palette.accent.r, palette.accent.g, palette.accent.b, 0.55),
    );
}

fn draw_header(game: &GameState, palette: Palette) {
    let sw = screen_width();
    draw_rectangle(0.0, 0.0, sw, 72.0, Color::new(0.012, 0.018, 0.034, 0.98));
    draw_line(0.0, 72.0, sw, 72.0, 1.0, Color::new(0.12, 0.17, 0.24, 1.0));

    let logo = Rect::new(16.0, 16.0, 34.0, 34.0);
    draw_rectangle(logo.x, logo.y, logo.w, logo.h, palette.accent);
    draw_centered("MW", logo, 15.0, Color::new(0.015, 0.025, 0.04, 1.0));
    draw_text("MAZE WARS 3D", 62.0, 31.0, 21.0, WHITE);
    let badge = Rect::new(218.0, 13.0, 174.0, 23.0);
    draw_rectangle(
        badge.x,
        badge.y,
        badge.w,
        badge.h,
        Color::new(0.02, 0.16, 0.19, 0.72),
    );
    draw_rectangle_lines(
        badge.x,
        badge.y,
        badge.w,
        badge.h,
        1.0,
        Color::new(0.1, 0.55, 0.62, 0.65),
    );
    draw_centered("01-EDU MULTIPLAYER FPS", badge, 10.5, palette.accent);
    draw_text("DDA Raycasting", 62.0, 53.0, 11.0, palette.muted);
    draw_text("|", 153.0, 53.0, 11.0, DARKGRAY);
    draw_text("UDP Native Client", 164.0, 53.0, 11.0, palette.muted);
    draw_text("|", 270.0, 53.0, 11.0, DARKGRAY);
    draw_text(">50 FPS Target", 281.0, 53.0, 11.0, palette.health);

    let level_rects = level_button_rects();
    for (index, rect) in level_rects.iter().enumerate() {
        let active = game.level_index == index;
        draw_dark_button(*rect, &format!("L{}", index + 1), active);
    }
    let proc = procedural_button_rect();
    draw_dark_button(proc, "PROCEDURAL", false);

    let gateway = gateway_button_rect();
    let gateway_text = format!("GATEWAY {}", server_host(&game.server));
    draw_dark_button(gateway, &shorten(&gateway_text, 20), false);
    draw_dark_button(editor_button_rect(), "CUSTOM", game.level_index == 3);
    draw_dark_button(
        score_button_rect(),
        "SCORE",
        game.overlay == Overlay::Scoreboard,
    );
    draw_dark_button(
        settings_button_rect(),
        "SET",
        game.overlay == Overlay::Settings,
    );
}

fn draw_info_bar(game: &GameState, palette: Palette) {
    let maze = &game.levels[game.level_index];
    draw_text(
        format!(
            "{}   {}x{} sector   {} dead ends",
            maze.name,
            maze.width,
            maze.height,
            maze.dead_ends()
        ),
        18.0,
        96.0,
        13.0,
        LIGHTGRAY,
    );
    let helper = if game.cursor_grabbed {
        "ESC releases mouse"
    } else {
        "Click viewport to lock mouse cursor"
    };
    let m = measure_text(helper, None, 12, 1.0);
    draw_text(
        helper,
        screen_width() - m.width - 18.0,
        96.0,
        12.0,
        palette.muted,
    );
}

fn draw_feature_cards(palette: Palette) {
    let y = screen_height() - 80.0;
    let gap = 9.0;
    let margin = 16.0;
    let w = (screen_width() - margin * 2.0 - gap * 3.0) / 4.0;
    let cards = [
        ("AUTHENTIC MAZE WARS 3D", "DDA + eyeball avatars"),
        ("UDP CLIENT-SERVER", "Authoritative Rust server"),
        ("3 LEVELS + GENERATOR", "Progressive dead ends"),
        ("LEVEL EDITOR", "Editable custom mazes"),
    ];
    for (index, (title, sub)) in cards.iter().enumerate() {
        let rect = Rect::new(margin + index as f32 * (w + gap), y, w, 62.0);
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            Color::new(0.012, 0.02, 0.035, 0.96),
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            1.0,
            Color::new(0.1, 0.15, 0.22, 1.0),
        );
        draw_circle(rect.x + 14.0, rect.y + 18.0, 4.0, palette.health);
        draw_text(*title, rect.x + 25.0, rect.y + 22.0, 11.0, palette.accent);
        draw_text(*sub, rect.x + 14.0, rect.y + 45.0, 10.0, palette.muted);
    }
}

fn draw_scoreboard(game: &GameState, palette: Palette) {
    draw_modal_backdrop();
    let panel = Rect::new(
        (screen_width() - 760.0) * 0.5,
        (screen_height() - 500.0) * 0.5,
        760.0,
        500.0,
    );
    draw_shadowed_panel(panel, Color::new(0.018, 0.026, 0.05, 0.99), palette.accent);
    draw_text(
        "COMBAT LEADERBOARD",
        panel.x + 28.0,
        panel.y + 42.0,
        24.0,
        WHITE,
    );
    draw_text(
        &game.levels[game.level_index].name,
        panel.x + 28.0,
        panel.y + 64.0,
        13.0,
        palette.muted,
    );
    draw_dark_button(scoreboard_close_rect(), "X", false);
    draw_line(
        panel.x + 24.0,
        panel.y + 82.0,
        panel.x + panel.w - 24.0,
        panel.y + 82.0,
        1.0,
        Color::new(0.12, 0.17, 0.24, 1.0),
    );

    let headers = [
        ("#", 34.0),
        ("AGENT", 80.0),
        ("STATUS", 330.0),
        ("HP", 470.0),
        ("FRAGS", 555.0),
        ("PING", 650.0),
    ];
    for (label, x) in headers {
        draw_text(label, panel.x + x, panel.y + 112.0, 11.0, palette.muted);
    }

    let mut rows: Vec<(u32, String, i32, i32, bool, bool)> = Vec::new();
    rows.push((
        game.network.player_id,
        game.username.clone(),
        game.health,
        game.score,
        false,
        true,
    ));
    for visual in game.remotes.values() {
        rows.push((
            visual.state.id,
            visual.state.name.clone(),
            visual.state.health,
            visual.state.score,
            visual.state.bot,
            false,
        ));
    }
    rows.sort_by(|left, right| right.3.cmp(&left.3).then_with(|| left.1.cmp(&right.1)));

    for (index, (_, name, health, score, bot, local)) in rows.iter().take(10).enumerate() {
        let y = panel.y + 132.0 + index as f32 * 31.0;
        if index % 2 == 0 {
            draw_rectangle(
                panel.x + 24.0,
                y - 18.0,
                panel.w - 48.0,
                28.0,
                Color::new(0.025, 0.04, 0.07, 0.7),
            );
        }
        draw_text(
            format!("{}", index + 1),
            panel.x + 36.0,
            y,
            13.0,
            palette.muted,
        );
        draw_text(
            shorten(name, 22),
            panel.x + 80.0,
            y,
            13.0,
            if *local {
                palette.accent
            } else if *bot {
                palette.danger
            } else {
                WHITE
            },
        );
        let status = if *local {
            "YOU"
        } else if *bot {
            "BOT"
        } else {
            "REMOTE"
        };
        draw_text(
            status,
            panel.x + 330.0,
            y,
            11.0,
            if *bot { palette.danger } else { palette.muted },
        );
        draw_text(
            format!("{}", health.max(&0)),
            panel.x + 470.0,
            y,
            13.0,
            LIGHTGRAY,
        );
        draw_text(format!("{}", score), panel.x + 555.0, y, 13.0, WHITE);
        draw_text(
            if *local {
                format!("{} ms", game.ping_ms)
            } else {
                "-".to_string()
            },
            panel.x + 650.0,
            y,
            12.0,
            palette.health,
        );
    }

    draw_text(
        "TAB toggles scoreboard during combat",
        panel.x + 246.0,
        panel.y + panel.h - 24.0,
        11.0,
        palette.muted,
    );
}

fn draw_settings(game: &GameState, palette: Palette) {
    draw_modal_backdrop();
    let panel = settings_panel_rect();
    draw_shadowed_panel(panel, Color::new(0.018, 0.026, 0.05, 0.99), palette.accent);
    draw_text(
        "SIMULATION & GRAPHICS CONFIGURATION",
        panel.x + 26.0,
        panel.y + 38.0,
        21.0,
        WHITE,
    );
    draw_text(
        "Visual rendering pipeline and input tuning",
        panel.x + 26.0,
        panel.y + 60.0,
        12.0,
        palette.muted,
    );
    draw_dark_button(settings_close_rect(), "X", false);
    draw_line(
        panel.x + 22.0,
        panel.y + 78.0,
        panel.x + panel.w - 22.0,
        panel.y + 78.0,
        1.0,
        Color::new(0.12, 0.17, 0.24, 1.0),
    );

    draw_text(
        "RENDER ARCHITECTURE & AESTHETIC",
        panel.x + 26.0,
        panel.y + 104.0,
        12.0,
        LIGHTGRAY,
    );
    let themes = [
        VisualTheme::Classic,
        VisualTheme::Cyberpunk,
        VisualTheme::Green,
        VisualTheme::Amber,
    ];
    for (index, theme) in themes.iter().enumerate() {
        let rect = theme_rect(index);
        let active = game.settings.theme == *theme;
        draw_rectangle(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if active {
                Color::new(0.02, 0.17, 0.20, 0.82)
            } else {
                Color::new(0.025, 0.04, 0.07, 0.8)
            },
        );
        draw_rectangle_lines(
            rect.x,
            rect.y,
            rect.w,
            rect.h,
            if active { 2.0 } else { 1.0 },
            if active {
                palette.accent
            } else {
                Color::new(0.12, 0.17, 0.24, 1.0)
            },
        );
        draw_text(
            theme.label(),
            rect.x + 12.0,
            rect.y + 25.0,
            14.0,
            if active { palette.accent } else { LIGHTGRAY },
        );
        let desc = match theme {
            VisualTheme::Classic => "High-contrast Maze Wars",
            VisualTheme::Cyberpunk => "Neon cyan / magenta",
            VisualTheme::Green => "Vintage phosphor CRT",
            VisualTheme::Amber => "Industrial monochrome",
        };
        draw_text(desc, rect.x + 12.0, rect.y + 46.0, 11.0, palette.muted);
    }

    draw_text(
        "DISPLAY OPTIONS",
        panel.x + 26.0,
        panel.y + 278.0,
        12.0,
        LIGHTGRAY,
    );
    draw_toggle(
        toggle_rect(0),
        "CRT SCANLINES",
        game.settings.crt_effect,
        palette,
    );
    draw_toggle(
        toggle_rect(1),
        "TACTICAL MINIMAP",
        game.settings.show_minimap,
        palette,
    );
    draw_toggle(
        toggle_rect(2),
        "MINIMAP FOV RAYS",
        game.settings.show_rays,
        palette,
    );

    draw_text(
        "MOUSE SENSITIVITY",
        panel.x + 26.0,
        panel.y + 370.0,
        12.0,
        LIGHTGRAY,
    );
    draw_slider(
        sensitivity_rect(),
        (game.settings.mouse_sensitivity - 0.25) / 1.75,
        palette,
    );
    draw_text(
        format!("{:.2}x", game.settings.mouse_sensitivity),
        panel.x + panel.w - 82.0,
        panel.y + 370.0,
        12.0,
        palette.accent,
    );

    draw_text(
        "FIELD OF VIEW",
        panel.x + 26.0,
        panel.y + 426.0,
        12.0,
        LIGHTGRAY,
    );
    draw_slider(fov_rect(), (game.settings.fov - 0.75) / 0.75, palette);
    draw_text(
        format!("{:.0} deg", game.settings.fov * 180.0 / PI),
        panel.x + panel.w - 82.0,
        panel.y + 426.0,
        12.0,
        palette.accent,
    );

    draw_gradient_like_button(settings_apply_rect(), "APPLY & RETURN", palette.accent);
}

fn handle_settings_input(game: &mut GameState) {
    if is_mouse_button_pressed(MouseButton::Left) {
        if point_in_rect(mouse_position(), settings_close_rect())
            || point_in_rect(mouse_position(), settings_apply_rect())
        {
            game.overlay = Overlay::None;
            return;
        }
        let themes = [
            VisualTheme::Classic,
            VisualTheme::Cyberpunk,
            VisualTheme::Green,
            VisualTheme::Amber,
        ];
        for (index, theme) in themes.iter().enumerate() {
            if point_in_rect(mouse_position(), theme_rect(index)) {
                game.settings.theme = *theme;
                return;
            }
        }
        if point_in_rect(mouse_position(), toggle_rect(0)) {
            game.settings.crt_effect = !game.settings.crt_effect;
            return;
        }
        if point_in_rect(mouse_position(), toggle_rect(1)) {
            game.settings.show_minimap = !game.settings.show_minimap;
            return;
        }
        if point_in_rect(mouse_position(), toggle_rect(2)) {
            game.settings.show_rays = !game.settings.show_rays;
            return;
        }
    }

    if is_mouse_button_down(MouseButton::Left) {
        let mouse = mouse_position();
        let sensitivity = sensitivity_rect();
        if point_in_rect(
            mouse,
            Rect::new(
                sensitivity.x - 10.0,
                sensitivity.y - 12.0,
                sensitivity.w + 20.0,
                32.0,
            ),
        ) {
            let t = ((mouse.0 - sensitivity.x) / sensitivity.w).clamp(0.0, 1.0);
            game.settings.mouse_sensitivity = 0.25 + t * 1.75;
        }
        let fov = fov_rect();
        if point_in_rect(
            mouse,
            Rect::new(fov.x - 10.0, fov.y - 12.0, fov.w + 20.0, 32.0),
        ) {
            let t = ((mouse.0 - fov.x) / fov.w).clamp(0.0, 1.0);
            game.settings.fov = 0.75 + t * 0.75;
        }
    }
}

fn record_death(game: &mut GameState, victim_id: u32, victim_name: String) {
    let Some(shooter_id) = game.last_hit_by.remove(&victim_id) else {
        return;
    };
    let killer = name_for_id(game, shooter_id).unwrap_or_else(|| "Unknown".to_string());
    game.kill_feed.push(KillFeedEntry {
        killer,
        victim: victim_name,
        expires_at: Instant::now() + Duration::from_secs(5),
    });
    if game.kill_feed.len() > 8 {
        game.kill_feed.remove(0);
    }
}

fn name_for_id(game: &GameState, id: u32) -> Option<String> {
    if id == game.network.player_id {
        return Some(game.username.clone());
    }
    game.remotes
        .get(&id)
        .map(|visual| visual.state.name.clone())
}

fn fire_weapon(game: &mut GameState) {
    if game.health <= 0 {
        return;
    }
    game.network.seq = game.network.seq.wrapping_add(1);
    let packet = encode_client(&ClientMessage::Shoot(game.network.seq));
    let _ = game.network.socket.send(packet.as_bytes());
    game.shot_flash = 0.12;
}

fn request_custom_level(game: &mut GameState, maze: Maze, status: &str) {
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

fn load_custom_level_file() -> io::Result<Maze> {
    let json = fs::read_to_string(CUSTOM_LEVEL_FILE)?;
    let maze = serde_json::from_str::<Maze>(&json).map_err(io::Error::other)?;
    if maze.cells.len() != maze.width.saturating_mul(maze.height)
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
    Ok(maze)
}

fn grab_cursor(game: &mut GameState) {
    game.cursor_grabbed = true;
    set_cursor_grab(true);
    show_mouse(false);
}

fn release_cursor(game: &mut GameState) {
    game.cursor_grabbed = false;
    set_cursor_grab(false);
    show_mouse(true);
}

fn disconnect(game: &GameState) {
    let _ = game
        .network
        .socket
        .send(encode_client(&ClientMessage::Leave).as_bytes());
    set_cursor_grab(false);
    show_mouse(true);
}

fn connect_after_game(game: &GameState, status: &str) -> AppScreen {
    AppScreen::Connect(ConnectScreen {
        server: game.server.clone(),
        username: game.username.clone(),
        alias: String::new(),
        active: ActiveField::Server,
        hosts: load_hosts(),
        status: status.to_string(),
        pending: None,
    })
}

fn default_connect_screen(status: &str) -> AppScreen {
    let hosts = load_hosts();
    let (server, username) = hosts
        .first()
        .map(|host| (host.address.clone(), host.username.clone()))
        .unwrap_or_else(|| (DEFAULT_SERVER.to_string(), "Pilot_01".to_string()));
    AppScreen::Connect(ConnectScreen {
        server,
        username,
        alias: String::new(),
        active: ActiveField::Server,
        hosts,
        status: status.to_string(),
        pending: None,
    })
}

fn top_action_at_mouse(level_index: usize) -> Option<TopAction> {
    if !is_mouse_button_pressed(MouseButton::Left) {
        return None;
    }
    let mouse = mouse_position();
    for (index, rect) in level_button_rects().iter().enumerate() {
        if point_in_rect(mouse, *rect) && index != level_index {
            return Some(TopAction::Level(index));
        }
    }
    if point_in_rect(mouse, procedural_button_rect()) {
        return Some(TopAction::Procedural);
    }
    if point_in_rect(mouse, gateway_button_rect()) {
        return Some(TopAction::Gateway);
    }
    if point_in_rect(mouse, editor_button_rect()) {
        return Some(TopAction::Editor);
    }
    if point_in_rect(mouse, score_button_rect()) {
        return Some(TopAction::Scoreboard);
    }
    if point_in_rect(mouse, settings_button_rect()) {
        return Some(TopAction::Settings);
    }
    None
}

fn level_button_rects() -> [Rect; 3] {
    let start = 408.0;
    [
        Rect::new(start, 18.0, 54.0, 34.0),
        Rect::new(start + 58.0, 18.0, 54.0, 34.0),
        Rect::new(start + 116.0, 18.0, 54.0, 34.0),
    ]
}

fn procedural_button_rect() -> Rect {
    Rect::new(582.0, 18.0, 106.0, 34.0)
}

fn gateway_button_rect() -> Rect {
    Rect::new((screen_width() - 430.0).max(700.0), 18.0, 198.0, 34.0)
}

fn editor_button_rect() -> Rect {
    let gateway = gateway_button_rect();
    Rect::new(gateway.x + gateway.w + 7.0, 18.0, 72.0, 34.0)
}

fn score_button_rect() -> Rect {
    let editor = editor_button_rect();
    Rect::new(editor.x + editor.w + 7.0, 18.0, 66.0, 34.0)
}

fn settings_button_rect() -> Rect {
    let score = score_button_rect();
    Rect::new(score.x + score.w + 7.0, 18.0, 50.0, 34.0)
}

fn game_view_rect() -> Rect {
    Rect::new(
        16.0,
        108.0,
        screen_width() - 32.0,
        (screen_height() - 204.0).max(360.0),
    )
}

fn settings_panel_rect() -> Rect {
    Rect::new(
        (screen_width() - 610.0) * 0.5,
        (screen_height() - 550.0) * 0.5,
        610.0,
        550.0,
    )
}

fn settings_close_rect() -> Rect {
    let panel = settings_panel_rect();
    Rect::new(panel.x + panel.w - 52.0, panel.y + 20.0, 30.0, 30.0)
}

fn settings_apply_rect() -> Rect {
    let panel = settings_panel_rect();
    Rect::new(
        panel.x + panel.w - 188.0,
        panel.y + panel.h - 55.0,
        162.0,
        34.0,
    )
}

fn scoreboard_close_rect() -> Rect {
    Rect::new(
        (screen_width() - 760.0) * 0.5 + 704.0,
        (screen_height() - 500.0) * 0.5 + 20.0,
        30.0,
        30.0,
    )
}

fn theme_rect(index: usize) -> Rect {
    let panel = settings_panel_rect();
    let col = index % 2;
    let row = index / 2;
    Rect::new(
        panel.x + 26.0 + col as f32 * 280.0,
        panel.y + 118.0 + row as f32 * 74.0,
        266.0,
        62.0,
    )
}

fn toggle_rect(index: usize) -> Rect {
    let panel = settings_panel_rect();
    Rect::new(
        panel.x + 26.0 + index as f32 * 180.0,
        panel.y + 294.0,
        166.0,
        48.0,
    )
}

fn sensitivity_rect() -> Rect {
    let panel = settings_panel_rect();
    Rect::new(panel.x + 26.0, panel.y + 386.0, panel.w - 124.0, 8.0)
}

fn fov_rect() -> Rect {
    let panel = settings_panel_rect();
    Rect::new(panel.x + 26.0, panel.y + 442.0, panel.w - 124.0, 8.0)
}

fn draw_toggle(rect: Rect, label: &str, active: bool, palette: Palette) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.025, 0.04, 0.07, 0.85),
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if active {
            palette.accent
        } else {
            Color::new(0.12, 0.17, 0.24, 1.0)
        },
    );
    draw_text(label, rect.x + 10.0, rect.y + 20.0, 11.0, LIGHTGRAY);
    let switch = Rect::new(rect.x + rect.w - 46.0, rect.y + 13.0, 34.0, 18.0);
    draw_rectangle(
        switch.x,
        switch.y,
        switch.w,
        switch.h,
        if active { palette.accent } else { DARKGRAY },
    );
    draw_circle(
        if active {
            switch.x + switch.w - 9.0
        } else {
            switch.x + 9.0
        },
        switch.y + 9.0,
        6.0,
        WHITE,
    );
}

fn draw_slider(rect: Rect, value: f32, palette: Palette) {
    let value = value.clamp(0.0, 1.0);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.08, 0.11, 0.16, 1.0),
    );
    draw_rectangle(rect.x, rect.y, rect.w * value, rect.h, palette.accent);
    draw_circle(rect.x + rect.w * value, rect.y + rect.h * 0.5, 7.0, WHITE);
}

fn draw_crt(view: Rect) {
    let mut y = view.y + 2.0;
    while y < view.y + view.h {
        draw_line(
            view.x,
            y,
            view.x + view.w,
            y,
            1.0,
            Color::new(0.0, 0.0, 0.0, 0.16),
        );
        y += 6.0;
    }
}

fn draw_bar(rect: Rect, ratio: f32, color: Color) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.08, 0.1, 0.14, 1.0),
    );
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w * ratio.clamp(0.0, 1.0),
        rect.h,
        color,
    );
}

fn draw_hud_panel(rect: Rect, palette: Palette) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(palette.panel.r, palette.panel.g, palette.panel.b, 0.82),
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::new(palette.border.r, palette.border.g, palette.border.b, 0.8),
    );
}

fn draw_crosshair(x: f32, y: f32, color: Color) {
    draw_line(x - 10.0, y, x - 3.0, y, 1.5, color);
    draw_line(x + 3.0, y, x + 10.0, y, 1.5, color);
    draw_line(x, y - 10.0, x, y - 3.0, 1.5, color);
    draw_line(x, y + 3.0, x, y + 10.0, 1.5, color);
}

fn draw_modal_backdrop() {
    draw_rectangle(
        0.0,
        0.0,
        screen_width(),
        screen_height(),
        Color::new(0.0, 0.0, 0.0, 0.82),
    );
}

fn draw_shadowed_panel(rect: Rect, fill: Color, border: Color) {
    draw_rectangle(
        rect.x + 8.0,
        rect.y + 10.0,
        rect.w,
        rect.h,
        Color::new(0.0, 0.0, 0.0, 0.34),
    );
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::new(border.r, border.g, border.b, 0.48),
    );
}

fn draw_status_card(rect: Rect, title: &str, subtitle: &str, accent: Color) {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.03, 0.05, 0.085, 1.0),
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        Color::new(accent.r, accent.g, accent.b, 0.5),
    );
    draw_circle(rect.x + 16.0, rect.y + 19.0, 4.0, accent);
    draw_text(title, rect.x + 28.0, rect.y + 23.0, 13.0, accent);
    draw_text(subtitle, rect.x + 14.0, rect.y + 51.0, 11.0, GRAY);
}

fn draw_field(label: &str, value: &str, rect: Rect, active: bool) {
    draw_text(
        label,
        rect.x,
        rect.y - 8.0,
        13.0,
        if active { SKYBLUE } else { GRAY },
    );
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        Color::new(0.015, 0.025, 0.045, 1.0),
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if active { 2.0 } else { 1.0 },
        if active {
            SKYBLUE
        } else {
            Color::new(0.12, 0.17, 0.24, 1.0)
        },
    );
    let cursor = if active && ((get_time() * 2.0) as i32 % 2 == 0) {
        "_"
    } else {
        ""
    };
    draw_text(
        format!("{value}{cursor}"),
        rect.x + 12.0,
        rect.y + 30.0,
        20.0,
        WHITE,
    );
}

fn draw_gradient_like_button(rect: Rect, label: &str, accent: Color) -> bool {
    let hover = point_in_rect(mouse_position(), rect);
    let fill = if hover {
        Color::new(
            (accent.r + 0.12).min(1.0),
            (accent.g + 0.06).min(1.0),
            (accent.b + 0.02).min(1.0),
            1.0,
        )
    } else {
        accent
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, fill);
    draw_rectangle(
        rect.x + rect.w * 0.65,
        rect.y,
        rect.w * 0.35,
        rect.h,
        Color::new(0.18, 0.34, 0.78, 0.35),
    );
    draw_centered(label, rect, 14.0, Color::new(0.01, 0.02, 0.04, 1.0));
    hover && is_mouse_button_pressed(MouseButton::Left)
}

fn draw_dark_button(rect: Rect, label: &str, active: bool) -> bool {
    let hover = point_in_rect(mouse_position(), rect);
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if active {
            Color::new(0.025, 0.20, 0.24, 1.0)
        } else if hover {
            Color::new(0.055, 0.085, 0.13, 1.0)
        } else {
            Color::new(0.025, 0.04, 0.07, 1.0)
        },
    );
    draw_rectangle_lines(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        1.0,
        if active {
            SKYBLUE
        } else {
            Color::new(0.12, 0.17, 0.24, 1.0)
        },
    );
    draw_centered(label, rect, 11.0, if active { SKYBLUE } else { LIGHTGRAY });
    hover && is_mouse_button_pressed(MouseButton::Left)
}

fn draw_centered(text: &str, rect: Rect, size: f32, color: Color) {
    let m = measure_text(text, None, size as u16, 1.0);
    draw_text(
        text,
        rect.x + (rect.w - m.width) * 0.5,
        rect.y + (rect.h + m.height) * 0.5 - 2.0,
        size,
        color,
    );
}

fn draw_connect_background() {
    let sw = screen_width();
    let sh = screen_height();
    clear_background(Color::new(0.006, 0.01, 0.02, 1.0));
    let mut x = 0.0;
    while x < sw {
        draw_line(x, 0.0, x, sh, 1.0, Color::new(0.02, 0.10, 0.13, 0.22));
        x += 48.0;
    }
    let mut y = 0.0;
    while y < sh {
        draw_line(0.0, y, sw, y, 1.0, Color::new(0.02, 0.10, 0.13, 0.22));
        y += 48.0;
    }
    for i in 0..20 {
        let t = i as f32 / 20.0;
        draw_line(
            sw * (0.58 + t * 0.42),
            0.0,
            sw * (0.58 + t * 0.28),
            sh,
            1.0,
            Color::new(0.02, 0.18, 0.22, 0.23),
        );
    }
}

fn palette(theme: VisualTheme) -> Palette {
    match theme {
        VisualTheme::Classic => Palette {
            bg: Color::new(0.007, 0.01, 0.02, 1.0),
            panel: Color::new(0.012, 0.025, 0.05, 1.0),
            panel_alt: Color::new(0.022, 0.04, 0.07, 1.0),
            border: Color::new(0.08, 0.42, 0.50, 1.0),
            accent: Color::new(0.22, 0.83, 0.95, 1.0),
            accent_alt: Color::new(0.65, 0.35, 0.95, 1.0),
            text: WHITE,
            muted: Color::new(0.52, 0.59, 0.69, 1.0),
            sky: Color::new(0.018, 0.045, 0.08, 1.0),
            floor: Color::new(0.018, 0.02, 0.032, 1.0),
            wall: Color::new(0.70, 0.86, 0.91, 1.0),
            wall_side: Color::new(0.52, 0.68, 0.73, 1.0),
            health: Color::new(0.20, 0.86, 0.50, 1.0),
            danger: Color::new(0.94, 0.26, 0.30, 1.0),
        },
        VisualTheme::Cyberpunk => Palette {
            bg: Color::new(0.018, 0.005, 0.035, 1.0),
            panel: Color::new(0.03, 0.01, 0.07, 1.0),
            panel_alt: Color::new(0.05, 0.015, 0.09, 1.0),
            border: Color::new(0.70, 0.10, 0.72, 1.0),
            accent: Color::new(0.0, 0.92, 1.0, 1.0),
            accent_alt: Color::new(0.95, 0.10, 0.82, 1.0),
            text: WHITE,
            muted: Color::new(0.68, 0.52, 0.76, 1.0),
            sky: Color::new(0.055, 0.015, 0.10, 1.0),
            floor: Color::new(0.015, 0.01, 0.035, 1.0),
            wall: Color::new(0.08, 0.90, 0.98, 1.0),
            wall_side: Color::new(0.68, 0.10, 0.80, 1.0),
            health: Color::new(0.20, 0.94, 0.58, 1.0),
            danger: Color::new(1.0, 0.16, 0.52, 1.0),
        },
        VisualTheme::Green => Palette {
            bg: Color::new(0.0, 0.018, 0.005, 1.0),
            panel: Color::new(0.0, 0.035, 0.012, 1.0),
            panel_alt: Color::new(0.0, 0.055, 0.018, 1.0),
            border: Color::new(0.1, 0.55, 0.22, 1.0),
            accent: Color::new(0.25, 1.0, 0.45, 1.0),
            accent_alt: Color::new(0.55, 1.0, 0.65, 1.0),
            text: Color::new(0.78, 1.0, 0.82, 1.0),
            muted: Color::new(0.34, 0.68, 0.40, 1.0),
            sky: Color::new(0.0, 0.04, 0.015, 1.0),
            floor: Color::new(0.0, 0.018, 0.008, 1.0),
            wall: Color::new(0.25, 0.95, 0.38, 1.0),
            wall_side: Color::new(0.12, 0.60, 0.24, 1.0),
            health: Color::new(0.35, 1.0, 0.45, 1.0),
            danger: Color::new(0.95, 0.38, 0.24, 1.0),
        },
        VisualTheme::Amber => Palette {
            bg: Color::new(0.025, 0.012, 0.0, 1.0),
            panel: Color::new(0.05, 0.025, 0.0, 1.0),
            panel_alt: Color::new(0.075, 0.035, 0.0, 1.0),
            border: Color::new(0.62, 0.35, 0.08, 1.0),
            accent: Color::new(1.0, 0.67, 0.17, 1.0),
            accent_alt: Color::new(1.0, 0.82, 0.38, 1.0),
            text: Color::new(1.0, 0.90, 0.67, 1.0),
            muted: Color::new(0.69, 0.51, 0.28, 1.0),
            sky: Color::new(0.07, 0.03, 0.0, 1.0),
            floor: Color::new(0.025, 0.012, 0.0, 1.0),
            wall: Color::new(1.0, 0.64, 0.18, 1.0),
            wall_side: Color::new(0.62, 0.34, 0.08, 1.0),
            health: Color::new(0.73, 0.88, 0.28, 1.0),
            danger: Color::new(1.0, 0.25, 0.12, 1.0),
        },
    }
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
    let endpoint = normalized_server(server);
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

fn normalized_server(server: &str) -> String {
    let server = server.trim();
    match server.parse::<std::net::IpAddr>() {
        Ok(std::net::IpAddr::V4(address)) => format!("{address}:34254"),
        Ok(std::net::IpAddr::V6(address)) => format!("[{address}]:34254"),
        Err(_) => server.to_string(),
    }
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
    let normalized = normalized_server(address);
    let alias = if alias.trim().is_empty() {
        normalized.clone()
    } else {
        alias.trim().to_string()
    };
    let entry = HostEntry {
        alias,
        address: normalized,
        username: username.trim().to_string(),
    };
    hosts.retain(|host| host.address != entry.address);
    hosts.insert(0, entry);
    hosts.truncate(8);
}

fn server_host(server: &str) -> String {
    server
        .parse::<std::net::SocketAddr>()
        .map(|address| address.ip().to_string())
        .unwrap_or_else(|_| server.to_string())
}

fn shorten(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max_chars.saturating_sub(3)).collect();
    out.push_str("...");
    out
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
