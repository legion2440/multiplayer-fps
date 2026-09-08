import { Player } from '../types';

export interface BotState {
  playerId: string;
  state: 'PATROL' | 'ENGAGE' | 'RETREAT';
  targetX: number;
  targetY: number;
  shootCooldown: number;
  turnSpeed: number;
  moveSpeed: number;
  lastKnownPlayerPos: { x: number; y: number } | null;
  patrolTimer: number;
}

export function hasLineOfSight(x1: number, y1: number, x2: number, y2: number, grid: number[][]): boolean {
  const dx = x2 - x1;
  const dy = y2 - y1;
  const dist = Math.hypot(dx, dy);
  const steps = Math.ceil(dist * 5);
  if (steps === 0) return true;
  for (let i = 1; i < steps; i++) {
    const t = i / steps;
    const cx = Math.floor(x1 + dx * t);
    const cy = Math.floor(y1 + dy * t);
    if (cy >= 0 && cy < grid.length && cx >= 0 && cx < grid[0].length && grid[cy][cx] !== 0) return false;
  }
  return true;
}

export function updateBots(
  bots: Player[], botStates: Map<string, BotState>, humanPlayer: Player, grid: number[][], dt: number,
  difficulty: 'Easy' | 'Medium' | 'Hard', onBotShoot: (bot: Player) => void
) {
  const speedMultiplier = difficulty === 'Easy' ? 0.7 : difficulty === 'Medium' ? 1.0 : 1.35;
  const shootDelay = difficulty === 'Easy' ? 1.8 : difficulty === 'Medium' ? 1.2 : 0.75;
  const fovRange = difficulty === 'Easy' ? 7 : difficulty === 'Medium' ? 10 : 14;

  bots.forEach((bot) => {
    if (!bot.alive) return;
    let state = botStates.get(bot.id);
    if (!state) {
      state = { playerId: bot.id, state: 'PATROL', targetX: bot.x, targetY: bot.y, shootCooldown: Math.random() * shootDelay, turnSpeed: 2.5, moveSpeed: 1.8 * speedMultiplier, lastKnownPlayerPos: null, patrolTimer: 0 };
      botStates.set(bot.id, state);
    }
    state.shootCooldown -= dt;
    state.patrolTimer -= dt;
    const distToPlayer = Math.hypot(humanPlayer.x - bot.x, humanPlayer.y - bot.y);
    const canSeePlayer = humanPlayer.alive && distToPlayer <= fovRange && hasLineOfSight(bot.x, bot.y, humanPlayer.x, humanPlayer.y, grid);

    if (canSeePlayer) {
      state.state = 'ENGAGE';
      state.lastKnownPlayerPos = { x: humanPlayer.x, y: humanPlayer.y };
    } else if (state.state === 'ENGAGE' && state.lastKnownPlayerPos && Math.hypot(state.lastKnownPlayerPos.x - bot.x, state.lastKnownPlayerPos.y - bot.y) < 1.0) {
      state.state = 'PATROL';
      state.lastKnownPlayerPos = null;
    }

    let targetAngle = bot.angle;
    let desiredMoveSpeed = 0;
    if (state.state === 'ENGAGE' && humanPlayer.alive) {
      targetAngle = Math.atan2(humanPlayer.y - bot.y, humanPlayer.x - bot.x);
      if (distToPlayer > 3.5) desiredMoveSpeed = state.moveSpeed;
      else if (distToPlayer < 2.0) desiredMoveSpeed = -state.moveSpeed * 0.5;
      else desiredMoveSpeed = state.moveSpeed * 0.3;

      let angleDiff = targetAngle - bot.angle;
      while (angleDiff > Math.PI) angleDiff -= Math.PI * 2;
      while (angleDiff < -Math.PI) angleDiff += Math.PI * 2;
      if (Math.abs(angleDiff) < 0.25 && state.shootCooldown <= 0 && bot.ammo > 0) {
        state.shootCooldown = shootDelay + (Math.random() * 0.3 - 0.15);
        onBotShoot(bot);
      }
    } else {
      if (state.patrolTimer <= 0) {
        state.patrolTimer = 2.0 + Math.random() * 3.0;
        targetAngle = Math.random() * Math.PI * 2;
      }
      desiredMoveSpeed = state.moveSpeed * 0.6;
    }

    let angleDiff = targetAngle - bot.angle;
    while (angleDiff > Math.PI) angleDiff -= Math.PI * 2;
    while (angleDiff < -Math.PI) angleDiff += Math.PI * 2;
    bot.angle += Math.sign(angleDiff) * Math.min(Math.abs(angleDiff), state.turnSpeed * dt);
    bot.angle = (bot.angle + Math.PI * 2) % (Math.PI * 2);

    if (desiredMoveSpeed !== 0) {
      const moveStep = desiredMoveSpeed * dt;
      const nextX = bot.x + Math.cos(bot.angle) * moveStep;
      const nextY = bot.y + Math.sin(bot.angle) * moveStep;
      const radius = 0.25;
      const testX = Math.floor(nextX + Math.sign(Math.cos(bot.angle)) * radius);
      const testY = Math.floor(nextY + Math.sign(Math.sin(bot.angle)) * radius);
      const rowY = Math.floor(bot.y);
      const colX = Math.floor(bot.x);
      const canMoveX = rowY >= 0 && rowY < grid.length && testX >= 0 && testX < grid[0].length && grid[rowY][testX] === 0;
      const canMoveY = testY >= 0 && testY < grid.length && colX >= 0 && colX < grid[0].length && grid[testY][colX] === 0;
      if (canMoveX) bot.x = nextX;
      if (canMoveY) bot.y = nextY;
      if (!canMoveX && !canMoveY && state.state === 'PATROL') {
        bot.angle += Math.PI * 0.5 + Math.random() * Math.PI * 0.5;
        state.patrolTimer = 1.0;
      }
    }
  });
}
