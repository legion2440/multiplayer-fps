use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::f32::consts::PI;

pub const MOVE_SPEED: f32 = 3.2;
pub const TURN_SPEED: f32 = 2.6;
pub const PLAYER_RADIUS: f32 = 0.18;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Maze {
    pub name: String,
    pub difficulty: String,
    pub width: usize,
    pub height: usize,
    pub seed: u64,
    pub cells: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub struct RayHit {
    pub distance: f32,
    pub side: u8,
}

#[derive(Clone, Copy)]
struct XorShift64 {
    state: u64,
}

impl XorShift64 {
    fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn index(&mut self, len: usize) -> usize {
        if len <= 1 {
            0
        } else {
            (self.next_u64() as usize) % len
        }
    }
}

impl Maze {
    pub fn generated(
        name: impl Into<String>,
        difficulty: impl Into<String>,
        width: usize,
        height: usize,
        seed: u64,
    ) -> Self {
        let width = normalize_size(width);
        let height = normalize_size(height);
        let mut cells = vec![1u8; width * height];
        let mut rng = XorShift64::new(seed);
        let mut stack = vec![(1usize, 1usize)];
        cells[width + 1] = 0;

        while let Some(&(cx, cy)) = stack.last() {
            let mut candidates = [(0usize, 0usize, 0usize, 0usize); 4];
            let mut count = 0usize;
            for (dx, dy) in [(2isize, 0isize), (-2, 0), (0, 2), (0, -2)] {
                let nx = cx as isize + dx;
                let ny = cy as isize + dy;
                if nx <= 0
                    || ny <= 0
                    || nx >= width as isize - 1
                    || ny >= height as isize - 1
                {
                    continue;
                }
                let nx = nx as usize;
                let ny = ny as usize;
                if cells[ny * width + nx] == 1 {
                    let wx = ((cx as isize + nx as isize) / 2) as usize;
                    let wy = ((cy as isize + ny as isize) / 2) as usize;
                    candidates[count] = (nx, ny, wx, wy);
                    count += 1;
                }
            }

            if count == 0 {
                stack.pop();
                continue;
            }

            let (nx, ny, wx, wy) = candidates[rng.index(count)];
            cells[wy * width + wx] = 0;
            cells[ny * width + nx] = 0;
            stack.push((nx, ny));
        }

        Self {
            name: name.into(),
            difficulty: difficulty.into(),
            width,
            height,
            seed,
            cells,
        }
    }

    pub fn editor_blank(width: usize, height: usize) -> Self {
        let width = normalize_size(width);
        let height = normalize_size(height);
        let mut cells = vec![0u8; width * height];
        for y in 0..height {
            for x in 0..width {
                if x == 0 || y == 0 || x == width - 1 || y == height - 1 {
                    cells[y * width + x] = 1;
                }
            }
        }
        Self {
            name: "Custom Maze".to_string(),
            difficulty: "Custom".to_string(),
            width,
            height,
            seed: 0,
            cells,
        }
    }

    pub fn is_wall_cell(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return true;
        }
        self.cells[y as usize * self.width + x as usize] != 0
    }

    pub fn is_wall_at(&self, x: f32, y: f32) -> bool {
        self.is_wall_cell(x.floor() as i32, y.floor() as i32)
    }

    pub fn set_wall(&mut self, x: usize, y: usize, wall: bool) {
        if x >= self.width || y >= self.height {
            return;
        }
        if x == 0 || y == 0 || x == self.width - 1 || y == self.height - 1 {
            self.cells[y * self.width + x] = 1;
        } else {
            self.cells[y * self.width + x] = u8::from(wall);
        }
    }

    pub fn toggle_wall(&mut self, x: usize, y: usize) {
        if x == 0 || y == 0 || x >= self.width - 1 || y >= self.height - 1 {
            return;
        }
        let idx = y * self.width + x;
        self.cells[idx] = if self.cells[idx] == 0 { 1 } else { 0 };
    }

    pub fn dead_ends(&self) -> usize {
        let mut count = 0usize;
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                if self.cells[y * self.width + x] != 0 {
                    continue;
                }
                let open = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                    .iter()
                    .filter(|&&(dx, dy)| !self.is_wall_cell(x as i32 + dx, y as i32 + dy))
                    .count();
                if open == 1 {
                    count += 1;
                }
            }
        }
        count
    }

    pub fn spawn_points(&self, count: usize) -> Vec<(f32, f32)> {
        let mut candidates = Vec::new();
        for y in 1..self.height - 1 {
            for x in 1..self.width - 1 {
                if self.cells[y * self.width + x] == 0 {
                    candidates.push((x as f32 + 0.5, y as f32 + 0.5));
                }
            }
        }
        if candidates.is_empty() {
            return vec![(1.5, 1.5)];
        }

        let mut selected = vec![candidates[0]];
        while selected.len() < count.min(candidates.len()) {
            let mut best = candidates[0];
            let mut best_score = -1.0f32;
            for &candidate in &candidates {
                let min_dist = selected
                    .iter()
                    .map(|&(sx, sy)| {
                        let dx = candidate.0 - sx;
                        let dy = candidate.1 - sy;
                        dx * dx + dy * dy
                    })
                    .fold(f32::INFINITY, f32::min);
                if min_dist > best_score {
                    best_score = min_dist;
                    best = candidate;
                }
            }
            selected.push(best);
        }
        selected
    }

    pub fn cast_ray(&self, x: f32, y: f32, angle: f32) -> RayHit {
        let ray_dir_x = angle.cos();
        let ray_dir_y = angle.sin();
        let mut map_x = x.floor() as i32;
        let mut map_y = y.floor() as i32;

        let delta_x = if ray_dir_x.abs() < 1e-6 {
            f32::INFINITY
        } else {
            (1.0 / ray_dir_x).abs()
        };
        let delta_y = if ray_dir_y.abs() < 1e-6 {
            f32::INFINITY
        } else {
            (1.0 / ray_dir_y).abs()
        };

        let (step_x, mut side_x) = if ray_dir_x < 0.0 {
            (-1, (x - map_x as f32) * delta_x)
        } else {
            (1, (map_x as f32 + 1.0 - x) * delta_x)
        };
        let (step_y, mut side_y) = if ray_dir_y < 0.0 {
            (-1, (y - map_y as f32) * delta_y)
        } else {
            (1, (map_y as f32 + 1.0 - y) * delta_y)
        };

        let mut side = 0u8;
        for _ in 0..(self.width + self.height) * 4 {
            if side_x < side_y {
                side_x += delta_x;
                map_x += step_x;
                side = 0;
            } else {
                side_y += delta_y;
                map_y += step_y;
                side = 1;
            }
            if self.is_wall_cell(map_x, map_y) {
                break;
            }
        }

        let distance = if side == 0 {
            (map_x as f32 - x + (1 - step_x) as f32 * 0.5) / ray_dir_x
        } else {
            (map_y as f32 - y + (1 - step_y) as f32 * 0.5) / ray_dir_y
        };

        RayHit {
            distance: distance.abs().max(0.001),
            side,
        }
    }

    pub fn line_of_sight(&self, from: (f32, f32), to: (f32, f32)) -> bool {
        let dx = to.0 - from.0;
        let dy = to.1 - from.1;
        let distance = (dx * dx + dy * dy).sqrt();
        if distance < 0.05 {
            return true;
        }
        let angle = dy.atan2(dx);
        self.cast_ray(from.0, from.1, angle).distance + 0.08 >= distance
    }

    pub fn next_step_toward(
        &self,
        start: (usize, usize),
        goal: (usize, usize),
    ) -> Option<(usize, usize)> {
        if start == goal {
            return Some(goal);
        }
        if start.0 >= self.width
            || start.1 >= self.height
            || goal.0 >= self.width
            || goal.1 >= self.height
            || self.cells[start.1 * self.width + start.0] != 0
            || self.cells[goal.1 * self.width + goal.0] != 0
        {
            return None;
        }

        let total = self.width * self.height;
        let mut prev = vec![usize::MAX; total];
        let start_idx = start.1 * self.width + start.0;
        let goal_idx = goal.1 * self.width + goal.0;
        let mut queue = VecDeque::new();
        queue.push_back(start_idx);
        prev[start_idx] = start_idx;

        while let Some(idx) = queue.pop_front() {
            if idx == goal_idx {
                break;
            }
            let x = idx % self.width;
            let y = idx / self.width;
            for (dx, dy) in [(1i32, 0i32), (-1, 0), (0, 1), (0, -1)] {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= self.width as i32 || ny >= self.height as i32 {
                    continue;
                }
                let nidx = ny as usize * self.width + nx as usize;
                if self.cells[nidx] == 0 && prev[nidx] == usize::MAX {
                    prev[nidx] = idx;
                    queue.push_back(nidx);
                }
            }
        }

        if prev[goal_idx] == usize::MAX {
            return None;
        }

        let mut cursor = goal_idx;
        while prev[cursor] != start_idx && cursor != start_idx {
            cursor = prev[cursor];
        }
        Some((cursor % self.width, cursor / self.width))
    }
}

pub fn builtin_levels() -> Vec<Maze> {
    let specs = [
        ("Sector 01: Training Quarters", "Novice", 15usize, 15usize, 0xA11CEu64),
        ("Sector 02: Quarantine Vault", "Intermediate", 21, 21, 0xBADC0DE),
        ("Sector 03: The Void Labyrinth", "Master", 27, 27, 0xC0FFEE),
    ];
    let mut levels = Vec::with_capacity(specs.len());
    let mut previous_dead_ends = 0usize;

    for (name, difficulty, width, height, base_seed) in specs {
        let mut chosen = None;
        for attempt in 0..1024u64 {
            let maze = Maze::generated(name, difficulty, width, height, base_seed + attempt);
            if maze.dead_ends() > previous_dead_ends {
                chosen = Some(maze);
                break;
            }
        }
        let maze = chosen.unwrap_or_else(|| Maze::generated(name, difficulty, width, height, base_seed));
        previous_dead_ends = maze.dead_ends();
        levels.push(maze);
    }
    levels
}

pub fn move_entity(
    maze: &Maze,
    x: &mut f32,
    y: &mut f32,
    angle: &mut f32,
    forward: f32,
    strafe: f32,
    turn: f32,
    dt: f32,
) {
    *angle = normalize_angle(*angle + turn.clamp(-1.0, 1.0) * TURN_SPEED * dt);
    let mut mx = angle.cos() * forward.clamp(-1.0, 1.0)
        - angle.sin() * strafe.clamp(-1.0, 1.0);
    let mut my = angle.sin() * forward.clamp(-1.0, 1.0)
        + angle.cos() * strafe.clamp(-1.0, 1.0);
    let len = (mx * mx + my * my).sqrt();
    if len > 1.0 {
        mx /= len;
        my /= len;
    }
    mx *= MOVE_SPEED * dt;
    my *= MOVE_SPEED * dt;

    let next_x = *x + mx;
    if can_stand(maze, next_x, *y) {
        *x = next_x;
    }
    let next_y = *y + my;
    if can_stand(maze, *x, next_y) {
        *y = next_y;
    }
}

pub fn can_stand(maze: &Maze, x: f32, y: f32) -> bool {
    for (ox, oy) in [
        (-PLAYER_RADIUS, -PLAYER_RADIUS),
        (PLAYER_RADIUS, -PLAYER_RADIUS),
        (-PLAYER_RADIUS, PLAYER_RADIUS),
        (PLAYER_RADIUS, PLAYER_RADIUS),
    ] {
        if maze.is_wall_at(x + ox, y + oy) {
            return false;
        }
    }
    true
}

pub fn normalize_angle(mut angle: f32) -> f32 {
    while angle > PI {
        angle -= 2.0 * PI;
    }
    while angle < -PI {
        angle += 2.0 * PI;
    }
    angle
}

fn normalize_size(size: usize) -> usize {
    let size = size.max(7);
    if size % 2 == 0 { size + 1 } else { size }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_levels_increase_in_dead_ends() {
        let levels = builtin_levels();
        assert_eq!(levels.len(), 3);
        assert!(levels[1].dead_ends() > levels[0].dead_ends());
        assert!(levels[2].dead_ends() > levels[1].dead_ends());
    }

    #[test]
    fn generated_maze_has_open_spawn_and_walls_around_edge() {
        let maze = Maze::generated("test", "test", 15, 15, 42);
        assert!(!maze.is_wall_at(1.5, 1.5));
        for x in 0..maze.width {
            assert!(maze.is_wall_cell(x as i32, 0));
            assert!(maze.is_wall_cell(x as i32, maze.height as i32 - 1));
        }
    }

    #[test]
    fn pathfinder_returns_adjacent_step() {
        let maze = Maze::generated("test", "test", 15, 15, 99);
        let floors: Vec<_> = maze
            .spawn_points(2)
            .into_iter()
            .map(|(x, y)| (x.floor() as usize, y.floor() as usize))
            .collect();
        let next = maze.next_step_toward(floors[0], floors[1]).unwrap();
        let manhattan = floors[0].0.abs_diff(next.0) + floors[0].1.abs_diff(next.1);
        assert_eq!(manhattan, 1);
    }
}
