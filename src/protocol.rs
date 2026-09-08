use crate::maze::Maze;

#[derive(Debug, Clone)]
pub enum ClientMessage {
    Join(String),
    Input {
        seq: u32,
        forward: f32,
        strafe: f32,
        turn: f32,
        angle: f32,
    },
    Shoot(u32),
    Ping,
    NextLevel,
    SetLevel(usize),
    CustomLevel(Maze),
    Leave,
}

#[derive(Debug, Clone)]
pub struct NetPlayer {
    pub id: u32,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub angle: f32,
    pub health: i32,
    pub score: i32,
    pub bot: bool,
}

#[derive(Debug, Clone)]
pub enum ServerMessage {
    Welcome {
        id: u32,
        level: usize,
        x: f32,
        y: f32,
    },
    State {
        tick: u64,
        level: usize,
        players: Vec<NetPlayer>,
    },
    Shot {
        shooter: u32,
        target: Option<u32>,
    },
    Level(usize),
    CustomLevel(Maze),
    Pong,
    Reject(String),
}

pub fn sanitize_username(input: &str) -> String {
    let cleaned: String = input
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
        .take(16)
        .collect();
    if cleaned.is_empty() {
        "Agent".to_string()
    } else {
        cleaned
    }
}

pub fn encode_client(message: &ClientMessage) -> String {
    match message {
        ClientMessage::Join(name) => format!("JOIN|{}\n", sanitize_username(name)),
        ClientMessage::Input {
            seq,
            forward,
            strafe,
            turn,
            angle,
        } => {
            let angle = if angle.is_finite() { *angle } else { 0.0 };
            format!(
                "INPUT|{}|{:.3}|{:.3}|{:.3}|{:.6}\n",
                seq,
                forward.clamp(-1.0, 1.0),
                strafe.clamp(-1.0, 1.0),
                turn.clamp(-1.0, 1.0),
                angle
            )
        }
        ClientMessage::Shoot(seq) => format!("SHOOT|{}\n", seq),
        ClientMessage::Ping => "PING\n".to_string(),
        ClientMessage::NextLevel => "NEXT\n".to_string(),
        ClientMessage::SetLevel(level) => format!("SETLEVEL|{}\n", level),
        ClientMessage::CustomLevel(maze) => encode_maze("CUSTOM", maze),
        ClientMessage::Leave => "LEAVE\n".to_string(),
    }
}

pub fn parse_client(input: &str) -> Option<ClientMessage> {
    let parts: Vec<&str> = input.trim().split('|').collect();
    match parts.first().copied()? {
        "JOIN" => Some(ClientMessage::Join(sanitize_username(
            parts.get(1).copied().unwrap_or("Agent"),
        ))),
        "INPUT" if parts.len() >= 5 => {
            let angle = parts
                .get(5)
                .and_then(|value| value.parse::<f32>().ok())
                .unwrap_or(0.0);
            if !angle.is_finite() {
                return None;
            }
            Some(ClientMessage::Input {
                seq: parts[1].parse().ok()?,
                forward: parts[2].parse::<f32>().ok()?.clamp(-1.0, 1.0),
                strafe: parts[3].parse::<f32>().ok()?.clamp(-1.0, 1.0),
                turn: parts[4].parse::<f32>().ok()?.clamp(-1.0, 1.0),
                angle,
            })
        }
        "SHOOT" => Some(ClientMessage::Shoot(parts.get(1)?.parse().ok()?)),
        "PING" => Some(ClientMessage::Ping),
        "NEXT" => Some(ClientMessage::NextLevel),
        "SETLEVEL" => Some(ClientMessage::SetLevel(parts.get(1)?.parse().ok()?)),
        "CUSTOM" => Some(ClientMessage::CustomLevel(parse_maze(&parts)?)),
        "LEAVE" => Some(ClientMessage::Leave),
        _ => None,
    }
}

pub fn welcome(id: u32, level: usize, x: f32, y: f32) -> String {
    format!("WELCOME|{}|{}|{:.3}|{:.3}\n", id, level, x, y)
}

pub fn reject(reason: &str) -> String {
    let safe = reason.replace(['|', '\n', '\r'], " ");
    format!("REJECT|{}\n", safe)
}

pub fn pong() -> &'static str {
    "PONG\n"
}

pub fn level(index: usize) -> String {
    format!("LEVEL|{}\n", index)
}

pub fn custom_level(maze: &Maze) -> String {
    encode_maze("CUSTOMLEVEL", maze)
}

pub fn shot(shooter: u32, target: Option<u32>) -> String {
    format!("SHOT|{}|{}\n", shooter, target.unwrap_or(0))
}

pub fn state(tick: u64, level: usize, players: impl Iterator<Item = NetPlayer>) -> String {
    let mut out = format!("STATE|{}|{}|", tick, level);
    for player in players {
        out.push_str(&format!(
            "{},{},{:.3},{:.3},{:.3},{},{},{};",
            player.id,
            sanitize_username(&player.name),
            player.x,
            player.y,
            player.angle,
            player.health,
            player.score,
            u8::from(player.bot)
        ));
    }
    out.push('\n');
    out
}

pub fn parse_server(input: &str) -> Option<ServerMessage> {
    let parts: Vec<&str> = input.trim().split('|').collect();
    match parts.first().copied()? {
        "WELCOME" if parts.len() >= 5 => Some(ServerMessage::Welcome {
            id: parts[1].parse().ok()?,
            level: parts[2].parse().ok()?,
            x: parts[3].parse().ok()?,
            y: parts[4].parse().ok()?,
        }),
        "STATE" if parts.len() >= 4 => {
            let tick = parts[1].parse().ok()?;
            let level = parts[2].parse().ok()?;
            let mut players = Vec::new();
            for raw in parts[3].split(';').filter(|part| !part.is_empty()) {
                let fields: Vec<&str> = raw.split(',').collect();
                if fields.len() < 8 {
                    continue;
                }
                players.push(NetPlayer {
                    id: fields[0].parse().ok()?,
                    name: fields[1].to_string(),
                    x: fields[2].parse().ok()?,
                    y: fields[3].parse().ok()?,
                    angle: fields[4].parse().ok()?,
                    health: fields[5].parse().ok()?,
                    score: fields[6].parse().ok()?,
                    bot: fields[7] == "1",
                });
            }
            Some(ServerMessage::State {
                tick,
                level,
                players,
            })
        }
        "SHOT" if parts.len() >= 3 => {
            let shooter = parts[1].parse().ok()?;
            let target = parts[2].parse::<u32>().ok()?;
            Some(ServerMessage::Shot {
                shooter,
                target: (target != 0).then_some(target),
            })
        }
        "LEVEL" if parts.len() >= 2 => Some(ServerMessage::Level(parts[1].parse().ok()?)),
        "CUSTOMLEVEL" => Some(ServerMessage::CustomLevel(parse_maze(&parts)?)),
        "PONG" => Some(ServerMessage::Pong),
        "REJECT" => Some(ServerMessage::Reject(
            parts.get(1).copied().unwrap_or("Rejected").to_string(),
        )),
        _ => None,
    }
}

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
    let maze = Maze {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_round_trips_input() {
        let encoded = encode_client(&ClientMessage::Input {
            seq: 7,
            forward: 1.0,
            strafe: -0.5,
            turn: 0.25,
            angle: -0.123456,
        });
        match parse_client(&encoded).unwrap() {
            ClientMessage::Input {
                seq,
                forward,
                strafe,
                turn,
                angle,
            } => {
                assert_eq!(seq, 7);
                assert_eq!(forward, 1.0);
                assert_eq!(strafe, -0.5);
                assert_eq!(turn, 0.25);
                assert!((angle + 0.123456).abs() < 0.000001);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn protocol_round_trips_level_selection() {
        let encoded = encode_client(&ClientMessage::SetLevel(2));
        match parse_client(&encoded).unwrap() {
            ClientMessage::SetLevel(level) => assert_eq!(level, 2),
            other => panic!("unexpected message: {other:?}"),
        }
    }

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

    #[test]
    fn protocol_rejects_disconnected_custom_level() {
        let mut cells = vec![b'1'; 7 * 7];
        cells[8] = b'0';
        cells[5 * 7 + 5] = b'0';
        let walls = String::from_utf8(cells).unwrap();
        let packet = format!("CUSTOM|7|7|1|{walls}\n");
        assert!(parse_client(&packet).is_none());
    }

    #[test]
    fn username_is_protocol_safe() {
        assert_eq!(sanitize_username("a|b,c;d"), "abcd");
        assert_eq!(sanitize_username(""), "Agent");
    }
}
