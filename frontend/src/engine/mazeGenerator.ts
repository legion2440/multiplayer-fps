import { GameLevel, PickupItem, Position } from '../types';

export function generateProceduralMaze(
  width: number = 18,
  height: number = 18,
  difficulty: 'Novice' | 'Intermediate' | 'Master' = 'Intermediate'
): GameLevel {
  const w = width % 2 === 0 ? width + 1 : width;
  const h = height % 2 === 0 ? height + 1 : height;
  const grid: number[][] = Array(h).fill(0).map(() => Array(w).fill(1));
  const visited: boolean[][] = Array(h).fill(false).map(() => Array(w).fill(false));
  const stack: [number, number][] = [];

  grid[1][1] = 0;
  visited[1][1] = true;
  stack.push([1, 1]);

  const directions = [[0, -2], [2, 0], [0, 2], [-2, 0]];

  while (stack.length > 0) {
    const [cx, cy] = stack[stack.length - 1];
    const shuffledDirs = [...directions].sort(() => Math.random() - 0.5);
    let found = false;

    for (const [dx, dy] of shuffledDirs) {
      const nx = cx + dx;
      const ny = cy + dy;
      if (nx > 0 && nx < w - 1 && ny > 0 && ny < h - 1 && !visited[ny][nx]) {
        grid[cy + dy / 2][cx + dx / 2] = 0;
        grid[ny][nx] = 0;
        visited[ny][nx] = true;
        stack.push([nx, ny]);
        found = true;
        break;
      }
    }
    if (!found) stack.pop();
  }

  const loopChance = difficulty === 'Novice' ? 0.18 : difficulty === 'Intermediate' ? 0.08 : 0.03;
  for (let y = 1; y < h - 1; y++) {
    for (let x = 1; x < w - 1; x++) {
      if (grid[y][x] === 1) {
        const horizontal = grid[y][x - 1] === 0 && grid[y][x + 1] === 0;
        const vertical = grid[y - 1][x] === 0 && grid[y + 1][x] === 0;
        if ((horizontal || vertical) && Math.random() < loopChance) {
          grid[y][x] = 0;
        }
      }
    }
  }

  const openCells: Position[] = [];
  for (let y = 1; y < h - 1; y++) {
    for (let x = 1; x < w - 1; x++) {
      if (grid[y][x] === 0) openCells.push({ x: x + 0.5, y: y + 0.5 });
    }
  }

  const shuffledCells = [...openCells].sort(() => Math.random() - 0.5);
  const playerSpawn = shuffledCells.pop() || { x: 1.5, y: 1.5 };
  const botCount = difficulty === 'Novice' ? 3 : difficulty === 'Intermediate' ? 5 : 7;
  const enemySpawns: Position[] = [];
  for (let i = 0; i < botCount && shuffledCells.length > 0; i++) {
    enemySpawns.push(shuffledCells.pop()!);
  }

  const pickups: PickupItem[] = [];
  const pickupCount = Math.min(8, Math.floor(shuffledCells.length * 0.1));
  for (let i = 0; i < pickupCount; i++) {
    const cell = shuffledCells.pop();
    if (!cell) break;
    pickups.push({
      id: `pickup-proc-${i}`,
      type: i % 2 === 0 ? 'health' : 'ammo',
      x: cell.x,
      y: cell.y,
      active: true,
      respawnTime: 0,
    });
  }

  return {
    id: `procedural-${Date.now()}`,
    name: `Procedural Sector ${w}x${h}`,
    difficulty,
    width: w,
    height: h,
    grid,
    playerSpawn,
    enemySpawns,
    pickups,
    description: `Procedural labyrinth (${difficulty}, ${loopChance * 100}% loop factor).`,
  };
}
