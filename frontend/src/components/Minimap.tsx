import React, { useEffect, useRef } from 'react';
import { GameLevel, PickupItem, Player } from '../types';

interface MinimapProps {
  level: GameLevel;
  player: Player;
  otherPlayers: Player[];
  pickups: PickupItem[];
  showRays: boolean;
  fov: number;
}

export const Minimap: React.FC<MinimapProps> = ({ level, player, otherPlayers, pickups, showRays, fov }) => {
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    const w = canvas.width;
    const h = canvas.height;
    const sx = w / level.width;
    const sy = h / level.height;

    ctx.clearRect(0, 0, w, h);
    ctx.fillStyle = 'rgba(2, 6, 23, 0.94)';
    ctx.fillRect(0, 0, w, h);

    for (let y = 0; y < level.grid.length; y++) {
      for (let x = 0; x < level.grid[y].length; x++) {
        if (level.grid[y][x] !== 0) {
          ctx.fillStyle = '#334155';
          ctx.fillRect(x * sx, y * sy, Math.ceil(sx), Math.ceil(sy));
        }
      }
    }

    pickups.filter((p) => p.active).forEach((p) => {
      ctx.fillStyle = p.type === 'health' ? '#10b981' : '#38bdf8';
      ctx.beginPath();
      ctx.arc(p.x * sx, p.y * sy, 2.5, 0, Math.PI * 2);
      ctx.fill();
    });

    otherPlayers.filter((p) => p.alive).forEach((p) => {
      ctx.fillStyle = p.isBot ? '#ef4444' : '#a855f7';
      ctx.beginPath();
      ctx.arc(p.x * sx, p.y * sy, 3, 0, Math.PI * 2);
      ctx.fill();
      ctx.strokeStyle = ctx.fillStyle;
      ctx.beginPath();
      ctx.moveTo(p.x * sx, p.y * sy);
      ctx.lineTo((p.x + Math.cos(p.angle) * 0.7) * sx, (p.y + Math.sin(p.angle) * 0.7) * sy);
      ctx.stroke();
    });

    if (showRays) {
      ctx.strokeStyle = 'rgba(34, 211, 238, 0.25)';
      ctx.beginPath();
      ctx.moveTo(player.x * sx, player.y * sy);
      ctx.lineTo((player.x + Math.cos(player.angle - fov / 2) * 3) * sx, (player.y + Math.sin(player.angle - fov / 2) * 3) * sy);
      ctx.moveTo(player.x * sx, player.y * sy);
      ctx.lineTo((player.x + Math.cos(player.angle + fov / 2) * 3) * sx, (player.y + Math.sin(player.angle + fov / 2) * 3) * sy);
      ctx.stroke();
    }

    ctx.fillStyle = '#22d3ee';
    ctx.beginPath();
    ctx.arc(player.x * sx, player.y * sy, 4, 0, Math.PI * 2);
    ctx.fill();
    ctx.strokeStyle = '#e0f2fe';
    ctx.beginPath();
    ctx.moveTo(player.x * sx, player.y * sy);
    ctx.lineTo((player.x + Math.cos(player.angle) * 0.9) * sx, (player.y + Math.sin(player.angle) * 0.9) * sy);
    ctx.stroke();

    ctx.strokeStyle = '#0891b2';
    ctx.lineWidth = 1.5;
    ctx.strokeRect(0.75, 0.75, w - 1.5, h - 1.5);
  }, [level, player, otherPlayers, pickups, showRays, fov]);

  return <canvas ref={canvasRef} width={180} height={180} className="rounded-lg border border-cyan-500/50 bg-slate-950/90 shadow-xl shadow-cyan-950/30" />;
};
