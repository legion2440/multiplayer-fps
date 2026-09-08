import React from 'react';
import { Player } from '../types';
import { Trophy, X } from 'lucide-react';

interface ScoreboardProps {
  isOpen: boolean;
  onClose: () => void;
  players: Player[];
  levelName: string;
}

export const Scoreboard: React.FC<ScoreboardProps> = ({ isOpen, onClose, players, levelName }) => {
  if (!isOpen) return null;
  const sorted = [...players].sort((a, b) => b.score - a.score || b.kills - a.kills || a.deaths - b.deaths);

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4 backdrop-blur-md">
      <div className="w-full max-w-3xl overflow-hidden rounded-xl border border-cyan-500/40 bg-slate-950 shadow-2xl">
        <div className="flex items-center justify-between border-b border-slate-800 bg-slate-900/60 px-6 py-4">
          <div className="flex items-center gap-3"><Trophy className="h-5 w-5 text-amber-400" /><div><h2 className="font-display font-bold text-white">Combat Leaderboard</h2><p className="text-xs text-slate-400">{levelName}</p></div></div>
          <button onClick={onClose} className="cursor-pointer rounded p-2 text-slate-400 hover:bg-slate-800 hover:text-white"><X className="h-5 w-5" /></button>
        </div>
        <div className="p-5">
          <div className="grid grid-cols-[48px_1fr_80px_80px_80px_80px] gap-2 border-b border-slate-800 px-3 pb-2 text-[10px] uppercase tracking-wider text-slate-500">
            <span>#</span><span>Agent</span><span className="text-right">Frags</span><span className="text-right">Deaths</span><span className="text-right">Score</span><span className="text-right">Ping</span>
          </div>
          {sorted.map((p, idx) => (
            <div key={p.id} className="grid grid-cols-[48px_1fr_80px_80px_80px_80px] gap-2 border-b border-slate-900 px-3 py-3 text-xs">
              <span className="text-slate-500">{idx + 1}</span>
              <span className={p.isBot ? 'text-red-300' : 'text-cyan-300'}>{p.name} {p.isBot ? <span className="text-[9px] text-red-500">BOT</span> : null}</span>
              <span className="text-right text-slate-300">{p.kills}</span><span className="text-right text-slate-300">{p.deaths}</span><span className="text-right font-bold text-white">{p.score}</span><span className="text-right text-emerald-400">{p.ping} ms</span>
            </div>
          ))}
          <div className="pt-4 text-center text-[10px] text-slate-500">TAB toggles scoreboard during combat</div>
        </div>
      </div>
    </div>
  );
};
