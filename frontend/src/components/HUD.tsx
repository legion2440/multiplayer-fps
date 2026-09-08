import React from 'react';
import { KillEvent, Player } from '../types';

interface HUDProps {
  player: Player;
  fps: number;
  frameTime: number;
  recentKills: KillEvent[];
  ping: number;
  serverAddress: string;
}

export const HUD: React.FC<HUDProps> = ({
  player,
  fps,
  frameTime,
  recentKills,
  ping,
  serverAddress,
}) => {
  const hp = Math.max(0, Math.min(100, (player.health / player.maxHealth) * 100));
  const ammo = Math.max(0, Math.min(100, (player.ammo / player.maxAmmo) * 100));
  const angleDeg = ((player.angle * 180) / Math.PI + 360) % 360;
  const compass = angleDeg >= 315 || angleDeg < 45 ? 'E' : angleDeg < 135 ? 'S' : angleDeg < 225 ? 'W' : 'N';

  return (
    <div className="absolute inset-0 pointer-events-none z-10 font-mono text-xs">
      <div className="absolute top-4 left-4 flex flex-col gap-2">
        <div className="rounded-lg border border-slate-700/80 bg-slate-950/75 px-3 py-2 backdrop-blur-sm">
          <div className="text-slate-400">SERVER</div>
          <div className="text-cyan-300 font-bold">{serverAddress}</div>
          <div className="text-slate-500 mt-1">PING <span className="text-emerald-400">{ping} ms</span></div>
        </div>
        <div className="rounded-lg border border-slate-700/80 bg-slate-950/75 px-3 py-2 backdrop-blur-sm">
          <span className={fps >= 50 ? 'text-emerald-400 font-bold' : 'text-red-400 font-bold'}>{fps} FPS</span>
          <span className="text-slate-500 ml-2">{frameTime.toFixed(1)} ms</span>
          <div className={fps >= 50 ? 'text-emerald-500 text-[10px] mt-0.5' : 'text-red-400 text-[10px] mt-0.5'}>
            {fps >= 50 ? '01-EDU REQ >50 FPS' : 'FPS BELOW REQUIREMENT'}
          </div>
        </div>
      </div>

      <div className="absolute top-4 left-1/2 -translate-x-1/2 text-center">
        <div className="rounded border border-slate-700/70 bg-slate-950/60 px-4 py-1.5 text-slate-300">
          {compass} {Math.round(angleDeg)}°
        </div>
      </div>

      <div className="absolute left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2">
        <div className="relative h-5 w-5">
          <span className="absolute left-1/2 top-0 h-1.5 w-px -translate-x-1/2 bg-cyan-200" />
          <span className="absolute bottom-0 left-1/2 h-1.5 w-px -translate-x-1/2 bg-cyan-200" />
          <span className="absolute left-0 top-1/2 h-px w-1.5 -translate-y-1/2 bg-cyan-200" />
          <span className="absolute right-0 top-1/2 h-px w-1.5 -translate-y-1/2 bg-cyan-200" />
        </div>
      </div>

      <div className="absolute bottom-4 left-4 w-64 rounded-xl border border-slate-700/80 bg-slate-950/80 p-3 backdrop-blur-sm">
        <div className="mb-1 flex justify-between"><span className="text-slate-400">HULL</span><span className="text-white font-bold">{player.health}/{player.maxHealth}</span></div>
        <div className="h-2 overflow-hidden rounded bg-slate-800"><div className="h-full bg-emerald-500" style={{ width: `${hp}%` }} /></div>
        <div className="mb-1 mt-3 flex justify-between"><span className="text-slate-400">CHARGE</span><span className="text-cyan-300 font-bold">{player.ammo}/{player.maxAmmo}</span></div>
        <div className="h-2 overflow-hidden rounded bg-slate-800"><div className="h-full bg-cyan-500" style={{ width: `${ammo}%` }} /></div>
      </div>

      <div className="absolute bottom-4 right-4 rounded-xl border border-slate-700/80 bg-slate-950/80 px-4 py-3 text-right backdrop-blur-sm">
        <div className="text-[10px] text-slate-500">{player.name}</div>
        <div className="text-lg font-bold text-white">{player.score}</div>
        <div className="text-[10px] text-slate-400">K {player.kills} / D {player.deaths}</div>
      </div>

      {recentKills.length > 0 && (
        <div className="absolute right-4 top-52 flex flex-col items-end gap-1">
          {recentKills.slice(-4).map((event) => (
            <div key={event.id} className="rounded border border-slate-800 bg-slate-950/70 px-2 py-1 text-[10px] text-slate-300">
              <span className="text-cyan-300">{event.killer}</span> &gt; <span className="text-red-300">{event.victim}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};
