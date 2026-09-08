use multiplayer_fps::maze::{builtin_levels, move_entity, normalize_angle, Maze};
use multiplayer_fps::protocol::{
    custom_level as custom_level_packet, level as level_packet, parse_client, pong, reject, shot,
    state, welcome, ClientMessage, NetPlayer,
};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::thread;
use std::time::{Duration, Instant};

const TICK_HZ: u64 = 60;
const SNAPSHOT_EVERY_TICKS: u64 = 2;
const CLIENT_TIMEOUT: Duration = Duration::from_secs(5);
const RESPAWN_DELAY: Duration = Duration::from_secs(2);
const MAX_NETWORK_CLIENTS: usize = 64;
const DEFAULT_BOTS: usize = 3;

#[derive(Debug, Clone, Copy, Default)]
struct InputState {
    forward: f32,
    strafe: f32,
    turn: f32,
}

#[derive(Debug)]
struct Player {
    id: u32,
    name: String,
    addr: Option<SocketAddr>,
    x: f32,
    y: f32,
    angle: f32,
    health: i32,
    score: i32,
    last_seen: Instant,
    input: InputState,
    shoot_requested: bool,
    respawn_at: Option<Instant>,
    bot: bool,
    ai_waypoint: Option<(f32, f32)>,
    ai_repath_at: Instant,
    ai_shot_at: Instant,
}

impl Player {
    fn net_state(&self) -> NetPlayer {
        NetPlayer {
            id: self.id,
            name: self.name.clone(),
            x: self.x,
            y: self.y,
            angle: self.angle,
            health: self.health,
            score: self.score,
            bot: self.bot,
        }
    }
}

#[derive(Debug)]
struct Config {
    bind: String,
    bots: usize,
    level: usize,
}

fn main() -> io::Result<()> {
    let config = parse_args();
    let socket = UdpSocket::bind(&config.bind)?;
    socket.set_nonblocking(true)?;

    let mut levels = builtin_levels();
    let mut level_index = config.level.min(levels.len() - 1);
    let mut players: HashMap<u32, Player> = HashMap::new();
    let mut address_to_id: HashMap<SocketAddr, u32> = HashMap::new();
    let mut next_id = 1u32;

    add_bots(
        &mut players,
        config.bots,
        &levels[level_index],
        &mut next_id,
    );

    println!("Maze Wars UDP server");
    println!("  bind: {}", config.bind);
    println!("  bots: {}", config.bots);
    println!(
        "  level: {} - {} ({} dead ends)",
        level_index + 1,
        levels[level_index].name,
        levels[level_index].dead_ends()
    );
    println!("  capacity: {} network clients", MAX_NETWORK_CLIENTS);

    let tick_duration = Duration::from_nanos(1_000_000_000 / TICK_HZ);
    let mut tick = 0u64;
    let mut recv_buf = [0u8; 4096];
    let mut next_tick = Instant::now();
    let mut level_change_allowed_at = Instant::now();

    loop {
        receive_packets(
            &socket,
            &mut recv_buf,
            &mut players,
            &mut address_to_id,
            &mut levels,
            &mut level_index,
            &mut next_id,
            &mut level_change_allowed_at,
        )?;

        let now = Instant::now();
        if now < next_tick {
            thread::sleep(next_tick - now);
            continue;
        }
        next_tick += tick_duration;
        if now.duration_since(next_tick) > Duration::from_millis(250) {
            next_tick = now + tick_duration;
        }

        tick = tick.wrapping_add(1);
        remove_timed_out(&mut players, &mut address_to_id);
        update_respawns(&mut players, &levels[level_index], tick);
        update_bots(&mut players, &levels[level_index], now, tick);
        update_humans(
            &mut players,
            &levels[level_index],
            tick_duration.as_secs_f32(),
        );
        process_shots(&socket, &mut players, &levels[level_index]);

        if tick % SNAPSHOT_EVERY_TICKS == 0 {
            broadcast_state(&socket, &players, tick, level_index);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn receive_packets(
    socket: &UdpSocket,
    buf: &mut [u8],
    players: &mut HashMap<u32, Player>,
    address_to_id: &mut HashMap<SocketAddr, u32>,
    levels: &mut Vec<Maze>,
    level_index: &mut usize,
    next_id: &mut u32,
    level_change_allowed_at: &mut Instant,
) -> io::Result<()> {
    loop {
        let (amount, source) = match socket.recv_from(buf) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(()),
            Err(error) => return Err(error),
        };
        let packet = String::from_utf8_lossy(&buf[..amount]);
        let Some(message) = parse_client(&packet) else {
            continue;
        };

        match message {
            ClientMessage::Join(name) => {
                if let Some(id) = address_to_id.get(&source).copied() {
                    if let Some(player) = players.get_mut(&id) {
                        player.last_seen = Instant::now();
                        let _ = socket.send_to(
                            welcome(player.id, *level_index, player.x, player.y).as_bytes(),
                            source,
                        );
                        send_active_custom(socket, source, levels, *level_index);
                    }
                    continue;
                }
                let network_count = players.values().filter(|player| !player.bot).count();
                if network_count >= MAX_NETWORK_CLIENTS {
                    let _ = socket.send_to(reject("Server is full").as_bytes(), source);
                    continue;
                }
                let id = *next_id;
                *next_id = next_id.wrapping_add(1).max(1);
                let spawn = spawn_for_slot(&levels[*level_index], id as usize);
                let player = Player {
                    id,
                    name: name.clone(),
                    addr: Some(source),
                    x: spawn.0,
                    y: spawn.1,
                    angle: 0.0,
                    health: 100,
                    score: 0,
                    last_seen: Instant::now(),
                    input: InputState::default(),
                    shoot_requested: false,
                    respawn_at: None,
                    bot: false,
                    ai_waypoint: None,
                    ai_repath_at: Instant::now(),
                    ai_shot_at: Instant::now(),
                };
                players.insert(id, player);
                address_to_id.insert(source, id);
                let _ = socket.send_to(
                    welcome(id, *level_index, spawn.0, spawn.1).as_bytes(),
                    source,
                );
                send_active_custom(socket, source, levels, *level_index);
                println!("[join] {name} from {source} as #{id}");
            }
            ClientMessage::Input {
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
            }
            ClientMessage::CustomLevel(maze) => {
                if address_to_id.contains_key(&source) && Instant::now() >= *level_change_allowed_at
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
                    broadcast_raw(socket, players, &custom_level_packet(&levels[CUSTOM_INDEX]));
                    broadcast_raw(socket, players, &level_packet(CUSTOM_INDEX));
                    println!(
                        "[level] custom - {}x{} ({} dead ends)",
                        levels[CUSTOM_INDEX].width,
                        levels[CUSTOM_INDEX].height,
                        levels[CUSTOM_INDEX].dead_ends()
                    );
                }
            }
            ClientMessage::Shoot(_) => {
                if let Some(player) = player_for_source_mut(players, address_to_id, source) {
                    player.last_seen = Instant::now();
                    if player.health > 0 {
                        player.shoot_requested = true;
                    }
                }
            }
            ClientMessage::Ping => {
                if let Some(player) = player_for_source_mut(players, address_to_id, source) {
                    player.last_seen = Instant::now();
                }
                let _ = socket.send_to(pong().as_bytes(), source);
            }
            ClientMessage::NextLevel => {
                if address_to_id.contains_key(&source) && Instant::now() >= *level_change_allowed_at
                {
                    *level_index = (*level_index + 1) % levels.len();
                    reset_for_level(players, &levels[*level_index]);
                    *level_change_allowed_at = Instant::now() + Duration::from_secs(2);
                    broadcast_raw(socket, players, &level_packet(*level_index));
                    println!(
                        "[level] {} - {} ({} dead ends)",
                        *level_index + 1,
                        levels[*level_index].name,
                        levels[*level_index].dead_ends()
                    );
                }
            }
            ClientMessage::SetLevel(requested) => {
                if address_to_id.contains_key(&source)
                    && requested < levels.len()
                    && Instant::now() >= *level_change_allowed_at
                {
                    *level_index = requested;
                    reset_for_level(players, &levels[*level_index]);
                    *level_change_allowed_at = Instant::now() + Duration::from_secs(2);
                    broadcast_raw(socket, players, &level_packet(*level_index));
                    println!(
                        "[level] {} - {} ({} dead ends)",
                        *level_index + 1,
                        levels[*level_index].name,
                        levels[*level_index].dead_ends()
                    );
                }
            }
            ClientMessage::Leave => {
                if let Some(id) = address_to_id.remove(&source) {
                    if let Some(player) = players.remove(&id) {
                        println!("[leave] {} ({source})", player.name);
                    }
                }
            }
        }
    }
}

fn send_active_custom(socket: &UdpSocket, source: SocketAddr, levels: &[Maze], level_index: usize) {
    if level_index >= 3 {
        if let Some(maze) = levels.get(level_index) {
            let _ = socket.send_to(custom_level_packet(maze).as_bytes(), source);
        }
    }
}

fn player_for_source_mut<'a>(
    players: &'a mut HashMap<u32, Player>,
    address_to_id: &HashMap<SocketAddr, u32>,
    source: SocketAddr,
) -> Option<&'a mut Player> {
    let id = address_to_id.get(&source)?;
    players.get_mut(id)
}

fn update_humans(players: &mut HashMap<u32, Player>, maze: &Maze, dt: f32) {
    for player in players
        .values_mut()
        .filter(|player| !player.bot && player.health > 0)
    {
        move_entity(
            maze,
            &mut player.x,
            &mut player.y,
            &mut player.angle,
            player.input.forward,
            player.input.strafe,
            player.input.turn,
            dt,
        );
    }
}

fn update_bots(players: &mut HashMap<u32, Player>, maze: &Maze, now: Instant, tick: u64) {
    let humans: Vec<(u32, f32, f32)> = players
        .values()
        .filter(|player| !player.bot && player.health > 0)
        .map(|player| (player.id, player.x, player.y))
        .collect();
    if humans.is_empty() {
        return;
    }

    let bot_ids: Vec<u32> = players
        .values()
        .filter(|player| player.bot && player.health > 0)
        .map(|player| player.id)
        .collect();

    for bot_id in bot_ids {
        let Some((_, tx, ty)) = humans
            .iter()
            .min_by(|left, right| {
                let bot = &players[&bot_id];
                let dl = squared_distance((bot.x, bot.y), (left.1, left.2));
                let dr = squared_distance((bot.x, bot.y), (right.1, right.2));
                dl.total_cmp(&dr)
            })
            .copied()
        else {
            continue;
        };

        let (bx, by) = {
            let bot = &players[&bot_id];
            (bot.x, bot.y)
        };
        let sees_target = maze.line_of_sight((bx, by), (tx, ty));
        let desired_angle = (ty - by).atan2(tx - bx);

        let bot = players
            .get_mut(&bot_id)
            .expect("bot id came from player map");
        let angle_error = normalize_angle(desired_angle - bot.angle);
        bot.angle = normalize_angle(bot.angle + angle_error.clamp(-0.055, 0.055));

        if sees_target {
            let distance = squared_distance((bot.x, bot.y), (tx, ty)).sqrt();
            if distance > 2.0 {
                move_entity(
                    maze,
                    &mut bot.x,
                    &mut bot.y,
                    &mut bot.angle,
                    0.72,
                    0.0,
                    0.0,
                    1.0 / TICK_HZ as f32,
                );
            }
            if angle_error.abs() < 0.12 && now >= bot.ai_shot_at {
                bot.shoot_requested = true;
                let jitter = (bot.id as u64 * 17 + tick) % 7;
                bot.ai_shot_at = now + Duration::from_millis(650 + jitter * 45);
            }
        } else {
            if now >= bot.ai_repath_at {
                let start = (bot.x.floor() as usize, bot.y.floor() as usize);
                let goal = (tx.floor() as usize, ty.floor() as usize);
                bot.ai_waypoint = maze
                    .next_step_toward(start, goal)
                    .map(|(x, y)| (x as f32 + 0.5, y as f32 + 0.5));
                bot.ai_repath_at = now + Duration::from_millis(180);
            }
            if let Some((wx, wy)) = bot.ai_waypoint {
                let desired = (wy - bot.y).atan2(wx - bot.x);
                let error = normalize_angle(desired - bot.angle);
                bot.angle = normalize_angle(bot.angle + error.clamp(-0.075, 0.075));
                move_entity(
                    maze,
                    &mut bot.x,
                    &mut bot.y,
                    &mut bot.angle,
                    0.75,
                    0.0,
                    0.0,
                    1.0 / TICK_HZ as f32,
                );
            }
        }
    }
}

fn process_shots(socket: &UdpSocket, players: &mut HashMap<u32, Player>, maze: &Maze) {
    let shooters: Vec<u32> = players
        .values_mut()
        .filter_map(|player| {
            (player.health > 0 && std::mem::take(&mut player.shoot_requested)).then_some(player.id)
        })
        .collect();

    for shooter_id in shooters {
        let Some(shooter) = players.get(&shooter_id) else {
            continue;
        };
        let wall_distance = maze.cast_ray(shooter.x, shooter.y, shooter.angle).distance;
        let direction = (shooter.angle.cos(), shooter.angle.sin());
        let origin = (shooter.x, shooter.y);

        let target_id = players
            .values()
            .filter(|target| target.id != shooter_id && target.health > 0)
            .filter_map(|target| {
                let dx = target.x - origin.0;
                let dy = target.y - origin.1;
                let along = dx * direction.0 + dy * direction.1;
                let perpendicular = (dx * direction.1 - dy * direction.0).abs();
                (along > 0.0 && along < wall_distance && perpendicular < 0.30)
                    .then_some((target.id, along))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(id, _)| id);

        if let Some(target_id) = target_id {
            let mut killed = false;
            if let Some(target) = players.get_mut(&target_id) {
                target.health = (target.health - 34).max(0);
                if target.health == 0 {
                    killed = true;
                    target.input = InputState::default();
                    target.respawn_at = Some(Instant::now() + RESPAWN_DELAY);
                }
            }
            if killed {
                if let Some(shooter) = players.get_mut(&shooter_id) {
                    shooter.score += 1;
                }
            }
        }
        broadcast_raw(socket, players, &shot(shooter_id, target_id));
    }
}

fn update_respawns(players: &mut HashMap<u32, Player>, maze: &Maze, tick: u64) {
    let now = Instant::now();
    let due: Vec<u32> = players
        .values()
        .filter(|player| player.health == 0 && player.respawn_at.is_some_and(|at| now >= at))
        .map(|player| player.id)
        .collect();
    for id in due {
        let spawn = spawn_for_slot(maze, id as usize + tick as usize);
        if let Some(player) = players.get_mut(&id) {
            player.x = spawn.0;
            player.y = spawn.1;
            player.angle = 0.0;
            player.health = 100;
            player.respawn_at = None;
        }
    }
}

fn remove_timed_out(
    players: &mut HashMap<u32, Player>,
    address_to_id: &mut HashMap<SocketAddr, u32>,
) {
    let now = Instant::now();
    let stale: Vec<(u32, SocketAddr, String)> = players
        .values()
        .filter(|player| !player.bot && now.duration_since(player.last_seen) > CLIENT_TIMEOUT)
        .filter_map(|player| {
            player
                .addr
                .map(|addr| (player.id, addr, player.name.clone()))
        })
        .collect();
    for (id, addr, name) in stale {
        players.remove(&id);
        address_to_id.remove(&addr);
        println!("[timeout] {name} ({addr})");
    }
}

fn add_bots(players: &mut HashMap<u32, Player>, count: usize, maze: &Maze, next_id: &mut u32) {
    for index in 0..count {
        let id = *next_id;
        *next_id = next_id.wrapping_add(1).max(1);
        let spawn = spawn_for_slot(maze, index + 1);
        players.insert(
            id,
            Player {
                id,
                name: format!("EYE-BOT-{:02}", index + 1),
                addr: None,
                x: spawn.0,
                y: spawn.1,
                angle: PI,
                health: 100,
                score: 0,
                last_seen: Instant::now(),
                input: InputState::default(),
                shoot_requested: false,
                respawn_at: None,
                bot: true,
                ai_waypoint: None,
                ai_repath_at: Instant::now(),
                ai_shot_at: Instant::now() + Duration::from_millis(700 + 120 * index as u64),
            },
        );
    }
}

fn reset_for_level(players: &mut HashMap<u32, Player>, maze: &Maze) {
    let ids: Vec<u32> = players.keys().copied().collect();
    for (slot, id) in ids.into_iter().enumerate() {
        let spawn = spawn_for_slot(maze, slot);
        if let Some(player) = players.get_mut(&id) {
            player.x = spawn.0;
            player.y = spawn.1;
            player.angle = 0.0;
            player.health = 100;
            player.respawn_at = None;
            player.input = InputState::default();
            player.ai_waypoint = None;
        }
    }
}

fn spawn_for_slot(maze: &Maze, slot: usize) -> (f32, f32) {
    let spawns = maze.spawn_points(24);
    spawns[slot % spawns.len()]
}

fn broadcast_state(socket: &UdpSocket, players: &HashMap<u32, Player>, tick: u64, level: usize) {
    let packet = state(tick, level, players.values().map(Player::net_state));
    broadcast_raw(socket, players, &packet);
}

fn broadcast_raw(socket: &UdpSocket, players: &HashMap<u32, Player>, packet: &str) {
    for addr in players.values().filter_map(|player| player.addr) {
        let _ = socket.send_to(packet.as_bytes(), addr);
    }
}

fn squared_distance(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    dx * dx + dy * dy
}

fn parse_args() -> Config {
    let mut config = Config {
        bind: "0.0.0.0:34254".to_string(),
        bots: DEFAULT_BOTS,
        level: 0,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bind" => {
                if let Some(value) = args.next() {
                    config.bind = value;
                }
            }
            "--bots" => {
                if let Some(value) = args.next() {
                    config.bots = value.parse().unwrap_or(DEFAULT_BOTS).min(32);
                }
            }
            "--level" => {
                if let Some(value) = args.next() {
                    config.level = value.parse::<usize>().unwrap_or(1).saturating_sub(1).min(2);
                }
            }
            "--help" | "-h" => {
                println!(
                    "Usage: cargo run --release --bin server -- [--bind 0.0.0.0:34254] [--bots 3] [--level 1]"
                );
                std::process::exit(0);
            }
            _ => {}
        }
    }
    config
}
