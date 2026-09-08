import { GameLevel, PickupItem, Player, Projectile, VisualTheme } from '../types';

export interface RenderConfig {
  theme: VisualTheme;
  fov: number;
  renderDistance: number;
  crtEffect: boolean;
}

export function renderRaycasterScene(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  player: Player,
  level: GameLevel,
  otherPlayers: Player[],
  projectiles: Projectile[],
  pickups: PickupItem[],
  config: RenderConfig
) {
  drawBackground(ctx, width, height, config.theme);
  const zBuffer = new Array<number>(width).fill(Infinity);
  const dirX = Math.cos(player.angle);
  const dirY = Math.sin(player.angle);
  const plane = Math.tan(config.fov / 2);
  const planeX = -dirY * plane;
  const planeY = dirX * plane;

  for (let x = 0; x < width; x++) {
    const cameraX = (2 * x) / width - 1;
    const rayX = dirX + planeX * cameraX;
    const rayY = dirY + planeY * cameraX;
    let mapX = Math.floor(player.x);
    let mapY = Math.floor(player.y);
    const deltaX = Math.abs(1 / (rayX || 0.00001));
    const deltaY = Math.abs(1 / (rayY || 0.00001));
    const stepX = rayX < 0 ? -1 : 1;
    const stepY = rayY < 0 ? -1 : 1;
    let sideX = rayX < 0 ? (player.x - mapX) * deltaX : (mapX + 1 - player.x) * deltaX;
    let sideY = rayY < 0 ? (player.y - mapY) * deltaY : (mapY + 1 - player.y) * deltaY;
    let side = 0;

    for (let guard = 0; guard < level.width + level.height + 64; guard++) {
      if (sideX < sideY) { sideX += deltaX; mapX += stepX; side = 0; }
      else { sideY += deltaY; mapY += stepY; side = 1; }
      if (mapY < 0 || mapY >= level.grid.length || mapX < 0 || mapX >= level.grid[0].length || level.grid[mapY][mapX] > 0) break;
    }

    const dist = Math.abs(side === 0
      ? (mapX - player.x + (1 - stepX) / 2) / rayX
      : (mapY - player.y + (1 - stepY) / 2) / rayY);
    if (!Number.isFinite(dist) || dist <= 0) continue;
    zBuffer[x] = dist;
    const lineHeight = Math.floor(height / Math.max(0.0001, dist));
    const top = Math.max(0, Math.floor(height / 2 - lineHeight / 2));
    const bottom = Math.min(height - 1, Math.floor(height / 2 + lineHeight / 2));
    drawWall(ctx, x, top, bottom, dist, side, config.theme);
  }

  drawSprites(ctx, width, height, player, otherPlayers, projectiles, pickups, zBuffer, config);
}

function drawBackground(ctx: CanvasRenderingContext2D, width: number, height: number, theme: VisualTheme) {
  const half = height / 2;
  if (theme === 'green') {
    ctx.fillStyle = '#030d06'; ctx.fillRect(0, 0, width, half);
    ctx.fillStyle = '#010502'; ctx.fillRect(0, half, width, half);
  } else if (theme === 'amber') {
    ctx.fillStyle = '#0f0a02'; ctx.fillRect(0, 0, width, half);
    ctx.fillStyle = '#060401'; ctx.fillRect(0, half, width, half);
  } else if (theme === 'cyberpunk') {
    const sky = ctx.createLinearGradient(0, 0, 0, half); sky.addColorStop(0, '#050714'); sky.addColorStop(1, '#0e1428');
    const floor = ctx.createLinearGradient(0, half, 0, height); floor.addColorStop(0, '#0c101d'); floor.addColorStop(1, '#060810');
    ctx.fillStyle = sky; ctx.fillRect(0, 0, width, half); ctx.fillStyle = floor; ctx.fillRect(0, half, width, half);
    ctx.strokeStyle = 'rgba(56,189,248,.06)';
    for (let x = 0; x <= width; x += 60) { ctx.beginPath(); ctx.moveTo(width / 2, half); ctx.lineTo(x, height); ctx.stroke(); }
  } else {
    ctx.fillStyle = '#06070a'; ctx.fillRect(0, 0, width, height);
    ctx.strokeStyle = '#1e293b'; ctx.beginPath(); ctx.moveTo(0, half); ctx.lineTo(width, half); ctx.stroke();
  }
}

function drawWall(ctx: CanvasRenderingContext2D, x: number, y1: number, y2: number, dist: number, side: number, theme: VisualTheme) {
  const h = y2 - y1;
  if (h <= 0) return;
  const intensity = Math.max(0.1, 1 - Math.min(1, dist / 18));
  if (theme === 'green') {
    const g = Math.floor(255 * intensity * (side ? 0.75 : 1));
    ctx.fillStyle = `rgb(0,${g},${Math.floor(g * 0.3)})`;
  } else if (theme === 'amber') {
    const k = intensity * (side ? 0.75 : 1);
    ctx.fillStyle = `rgb(${Math.floor(255 * k)},${Math.floor(160 * k)},10)`;
  } else if (theme === 'cyberpunk') {
    const k = intensity * (side ? 0.75 : 1);
    ctx.fillStyle = `rgb(${Math.floor(30 * k)},${Math.floor(55 * k)},${Math.floor(110 * k)})`;
  } else {
    ctx.fillStyle = side ? '#090d15' : '#0e1524';
  }
  ctx.fillRect(x, y1, 1, h);
  if (theme === 'classic' || theme === 'cyberpunk') {
    ctx.fillStyle = theme === 'classic' ? `rgba(56,189,248,${intensity * 0.9})` : `rgba(168,85,247,${intensity * 0.8})`;
    ctx.fillRect(x, y1, 1, 2);
    ctx.fillRect(x, y2 - 2, 1, 2);
  }
}

type SpriteItem = { type: 'player' | 'pickup' | 'projectile'; x: number; y: number; dist: number; ref: Player | PickupItem | Projectile };

function drawSprites(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  player: Player,
  others: Player[],
  projectiles: Projectile[],
  pickups: PickupItem[],
  zBuffer: number[],
  config: RenderConfig
) {
  const sprites: SpriteItem[] = [];
  for (const p of others) if (p.id !== player.id && p.alive) sprites.push({ type: 'player', x: p.x, y: p.y, dist: Math.hypot(player.x - p.x, player.y - p.y), ref: p });
  for (const p of pickups) if (p.active) sprites.push({ type: 'pickup', x: p.x, y: p.y, dist: Math.hypot(player.x - p.x, player.y - p.y), ref: p });
  for (const p of projectiles) sprites.push({ type: 'projectile', x: p.x, y: p.y, dist: Math.hypot(player.x - p.x, player.y - p.y), ref: p });
  sprites.sort((a, b) => b.dist - a.dist);

  const dirX = Math.cos(player.angle), dirY = Math.sin(player.angle);
  const plane = Math.tan(config.fov / 2), planeX = -dirY * plane, planeY = dirX * plane;
  const invDet = 1 / (planeX * dirY - dirX * planeY);

  for (const sprite of sprites) {
    const sx = sprite.x - player.x, sy = sprite.y - player.y;
    const tx = invDet * (dirY * sx - dirX * sy);
    const ty = invDet * (-planeY * sx + planeX * sy);
    if (ty <= 0.2 || ty > config.renderDistance) continue;
    const screenX = Math.floor((width / 2) * (1 + tx / ty));
    const size = Math.abs(Math.floor(height / ty));
    if (screenX < -size || screenX >= width + size) continue;
    if (screenX >= 0 && screenX < width && ty > zBuffer[screenX]) continue;

    if (sprite.type === 'player') drawEye(ctx, screenX, height / 2, size, sprite.ref as Player, player, config.theme);
    else if (sprite.type === 'pickup') drawPickup(ctx, screenX, height / 2 + size * 0.2, size * 0.6, sprite.ref as PickupItem);
    else drawProjectile(ctx, screenX, height / 2, size * 0.2, sprite.ref as Projectile);
  }
}

function drawEye(ctx: CanvasRenderingContext2D, cx: number, cy: number, size: number, target: Player, viewer: Player, theme: VisualTheme) {
  const radius = size * 0.35;
  if (radius < 3) return;
  const grad = ctx.createRadialGradient(cx - radius * 0.3, cy - radius * 0.3, radius * 0.1, cx, cy, radius);
  if (theme === 'green') { grad.addColorStop(0, '#86efac'); grad.addColorStop(1, '#052e16'); }
  else if (theme === 'amber') { grad.addColorStop(0, '#fde68a'); grad.addColorStop(1, '#451a03'); }
  else { grad.addColorStop(0, '#fff'); grad.addColorStop(0.75, '#e2e8f0'); grad.addColorStop(1, '#64748b'); }
  ctx.beginPath(); ctx.arc(cx, cy, radius, 0, Math.PI * 2); ctx.fillStyle = grad; ctx.fill();
  ctx.strokeStyle = target.isBot ? '#ef4444' : '#38bdf8'; ctx.lineWidth = Math.max(1, radius * 0.08); ctx.stroke();

  const angleToViewer = Math.atan2(viewer.y - target.y, viewer.x - target.x);
  let relative = target.angle - angleToViewer;
  while (relative > Math.PI) relative -= Math.PI * 2;
  while (relative < -Math.PI) relative += Math.PI * 2;
  const pupilX = cx + Math.sin(relative) * radius * 0.5;
  ctx.beginPath(); ctx.arc(pupilX, cy, radius * 0.45, 0, Math.PI * 2); ctx.fillStyle = target.isBot ? '#dc2626' : '#0284c7'; ctx.fill();
  ctx.beginPath(); ctx.arc(pupilX, cy, radius * 0.22, 0, Math.PI * 2); ctx.fillStyle = '#0f172a'; ctx.fill();
  ctx.beginPath(); ctx.arc(pupilX - radius * 0.08, cy - radius * 0.08, radius * 0.07, 0, Math.PI * 2); ctx.fillStyle = '#fff'; ctx.fill();

  const tagY = cy - radius - 12;
  ctx.font = `${Math.max(10, Math.floor(radius * 0.35))}px 'JetBrains Mono', monospace`; ctx.textAlign = 'center'; ctx.fillStyle = target.isBot ? '#f87171' : '#38bdf8'; ctx.fillText(target.name, cx, tagY);
  const barW = Math.max(24, radius * 1.6), hp = Math.max(0, target.health / target.maxHealth);
  ctx.fillStyle = 'rgba(0,0,0,.6)'; ctx.fillRect(cx - barW / 2, tagY + 3, barW, 4);
  ctx.fillStyle = hp > 0.5 ? '#10b981' : hp > 0.25 ? '#f59e0b' : '#ef4444'; ctx.fillRect(cx - barW / 2, tagY + 3, barW * hp, 4);
}

function drawPickup(ctx: CanvasRenderingContext2D, cx: number, cy: number, size: number, pickup: PickupItem) {
  const r = size * 0.4;
  if (r < 2) return;
  const bob = Math.sin(Date.now() * 0.003 + pickup.x * 2) * r * 0.2;
  const grad = ctx.createRadialGradient(cx, cy + bob, r * 0.2, cx, cy + bob, r * 1.2);
  const health = pickup.type === 'health';
  grad.addColorStop(0, health ? 'rgba(16,185,129,.9)' : 'rgba(56,189,248,.9)');
  grad.addColorStop(1, health ? 'rgba(16,185,129,0)' : 'rgba(56,189,248,0)');
  ctx.beginPath(); ctx.arc(cx, cy + bob, r * 1.2, 0, Math.PI * 2); ctx.fillStyle = grad; ctx.fill();
  ctx.fillStyle = health ? '#10b981' : '#38bdf8'; ctx.fillRect(cx - r * 0.5, cy + bob - r * 0.5, r, r);
  ctx.fillStyle = '#fff'; ctx.font = `bold ${Math.max(8, Math.floor(r * 0.8))}px monospace`; ctx.textAlign = 'center'; ctx.textBaseline = 'middle'; ctx.fillText(health ? '+' : '⚡', cx, cy + bob);
}

function drawProjectile(ctx: CanvasRenderingContext2D, cx: number, cy: number, size: number, projectile: Projectile) {
  const r = Math.max(3, size);
  const grad = ctx.createRadialGradient(cx, cy, r * 0.1, cx, cy, r * 2.5);
  grad.addColorStop(0, '#fff'); grad.addColorStop(0.4, projectile.color || '#38bdf8'); grad.addColorStop(1, 'rgba(56,189,248,0)');
  ctx.beginPath(); ctx.arc(cx, cy, r * 2.5, 0, Math.PI * 2); ctx.fillStyle = grad; ctx.fill();
  ctx.beginPath(); ctx.arc(cx, cy, r, 0, Math.PI * 2); ctx.fillStyle = '#fff'; ctx.fill();
}
