import React, { useState } from 'react';
import { Check, Code, Download, Grid, Play, RefreshCw, Shield, Trash2, User, Users, X, Zap } from 'lucide-react';
import { sound } from '../audio/sound';
import { GameLevel, PickupItem, Position } from '../types';

interface LevelEditorProps {
  isOpen: boolean;
  onClose: () => void;
  onPlayLevel: (level: GameLevel) => void;
}

type ToolType = 'wall' | 'empty' | 'player' | 'enemy' | 'health' | 'ammo';

function emptyGrid(size: number): number[][] {
  const grid = Array.from({ length: size }, () => Array(size).fill(0));
  for (let i = 0; i < size; i++) {
    grid[0][i] = 1;
    grid[size - 1][i] = 1;
    grid[i][0] = 1;
    grid[i][size - 1] = 1;
  }
  return grid;
}

export const LevelEditor: React.FC<LevelEditorProps> = ({ isOpen, onClose, onPlayLevel }) => {
  const [size, setSize] = useState(14);
  const [tool, setTool] = useState<ToolType>('wall');
  const [grid, setGrid] = useState<number[][]>(() => emptyGrid(14));
  const [playerSpawn, setPlayerSpawn] = useState<Position>({ x: 1.5, y: 1.5 });
  const [enemySpawns, setEnemySpawns] = useState<Position[]>([{ x: 12.5, y: 1.5 }, { x: 1.5, y: 12.5 }]);
  const [pickups, setPickups] = useState<PickupItem[]>([{ id: 'p-edit-1', type: 'health', x: 6.5, y: 6.5, active: true, respawnTime: 0 }]);
  const [dragging, setDragging] = useState(false);
  const [copied, setCopied] = useState(false);

  if (!isOpen) return null;

  const resize = (next: number) => {
    setSize(next);
    setGrid(emptyGrid(next));
    setPlayerSpawn({ x: 1.5, y: 1.5 });
    setEnemySpawns([{ x: next - 1.5, y: 1.5 }, { x: 1.5, y: next - 1.5 }]);
    setPickups([]);
  };

  const paint = (x: number, y: number, drag = false) => {
    if (x === 0 || y === 0 || x === size - 1 || y === size - 1) return;
    if (drag && tool !== 'wall' && tool !== 'empty') return;

    if (tool === 'wall' || tool === 'empty') {
      setGrid((prev) => prev.map((row, ry) => row.map((cell, rx) => rx === x && ry === y ? (tool === 'wall' ? 1 : 0) : cell)));
      return;
    }

    const pos = { x: x + 0.5, y: y + 0.5 };
    setGrid((prev) => prev.map((row, ry) => row.map((cell, rx) => rx === x && ry === y ? 0 : cell)));
    if (tool === 'player') setPlayerSpawn(pos);
    if (tool === 'enemy') {
      setEnemySpawns((prev) => prev.some((p) => Math.floor(p.x) === x && Math.floor(p.y) === y) ? prev.filter((p) => Math.floor(p.x) !== x || Math.floor(p.y) !== y) : [...prev, pos]);
    }
    if (tool === 'health' || tool === 'ammo') {
      setPickups((prev) => {
        const exists = prev.some((p) => Math.floor(p.x) === x && Math.floor(p.y) === y);
        if (exists) return prev.filter((p) => Math.floor(p.x) !== x || Math.floor(p.y) !== y);
        return [...prev, { id: `pick-${Date.now()}`, type: tool, ...pos, active: true, respawnTime: 0 }];
      });
    }
  };

  const level = (): GameLevel => ({
    id: `custom-${Date.now()}`,
    name: 'Custom Engineered Arena',
    difficulty: 'Custom',
    width: size,
    height: size,
    grid,
    playerSpawn,
    enemySpawns,
    pickups,
    description: 'Custom level generated with the in-game visual editor.',
  });

  const randomize = () => {
    const next = emptyGrid(size);
    for (let y = 2; y < size - 2; y += 2) {
      for (let x = 2; x < size - 2; x += 2) if (Math.random() > 0.3) next[y][x] = 1;
    }
    setGrid(next);
  };

  const exportRust = async () => {
    const text = `pub const CUSTOM_MAZE: [[u8; ${size}]; ${size}] = [\n${grid.map((r) => `    [${r.join(', ')}],`).join('\n')}\n];`;
    await navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 1800);
  };

  const exportJson = () => {
    const blob = new Blob([JSON.stringify(level(), null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `custom_maze_${size}x${size}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const tools: { id: ToolType; label: string; icon?: React.FC<{ className?: string }> }[] = [
    { id: 'wall', label: 'Solid Wall (1)' }, { id: 'empty', label: 'Walkable Pathway (0)' },
    { id: 'player', label: 'Player Spawn', icon: User }, { id: 'enemy', label: 'Enemy Spawn', icon: Users },
    { id: 'health', label: 'Health Pack', icon: Shield }, { id: 'ammo', label: 'Ammo Crate', icon: Zap },
  ];

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/85 p-4 backdrop-blur-md" onMouseUp={() => setDragging(false)}>
      <div className="flex max-h-[92vh] w-full max-w-4xl flex-col overflow-hidden rounded-xl border border-cyan-500/40 bg-slate-950 shadow-2xl">
        <div className="flex items-center justify-between border-b border-slate-800 bg-slate-900/60 px-6 py-4">
          <div className="flex items-center gap-3"><Grid className="h-5 w-5 text-cyan-400" /><div><h2 className="font-display text-lg font-bold text-white">Tactical Maze Level Editor</h2><p className="font-mono text-xs text-slate-400">Paint walls, spawns and pickups, then launch directly into 3D.</p></div></div>
          <div className="flex gap-2"><button onClick={() => { sound.playUiClick(); onPlayLevel(level()); onClose(); }} className="flex items-center gap-2 rounded-lg bg-emerald-500 px-4 py-2 font-mono text-xs font-bold text-slate-950"><Play className="h-4 w-4 fill-current" />PLAY LEVEL NOW</button><button onClick={onClose} className="p-2 text-slate-400"><X className="h-5 w-5" /></button></div>
        </div>

        <div className="grid flex-1 grid-cols-1 overflow-hidden md:grid-cols-12">
          <div className="space-y-4 overflow-y-auto border-r border-slate-800 p-5 md:col-span-4">
            <div><div className="mb-2 font-mono text-[11px] uppercase tracking-wider text-slate-400">Grid Dimensions</div><div className="grid grid-cols-3 gap-2">{[10,14,18].map((n) => <button key={n} onClick={() => resize(n)} className={`rounded border py-1.5 font-mono text-xs ${size === n ? 'border-cyan-400 bg-cyan-500/20 text-cyan-300' : 'border-slate-800 bg-slate-900 text-slate-400'}`}>{n}x{n}</button>)}</div></div>
            <div><div className="mb-2 font-mono text-[11px] uppercase tracking-wider text-slate-400">Active Paint Brush</div><div className="space-y-1.5">{tools.map((item) => <button key={item.id} onClick={() => setTool(item.id)} className={`flex w-full items-center justify-between rounded-lg border px-3 py-2 font-mono text-xs ${tool === item.id ? 'border-cyan-400 bg-cyan-950/40 text-cyan-200' : 'border-slate-800 bg-slate-900/40 text-slate-300'}`}><span>{item.label}</span>{tool === item.id && <Check className="h-3.5 w-3.5 text-cyan-400" />}</button>)}</div></div>
            <button onClick={randomize} className="flex w-full items-center justify-center gap-1.5 rounded-lg border border-slate-700 bg-slate-900 py-2 font-mono text-xs text-slate-300"><RefreshCw className="h-3.5 w-3.5 text-cyan-400" />Randomize Pillars</button>
            <button onClick={() => { setGrid(emptyGrid(size)); setEnemySpawns([]); setPickups([]); }} className="flex w-full items-center justify-center gap-1.5 rounded-lg border border-slate-800 bg-slate-900 py-2 font-mono text-xs text-slate-400"><Trash2 className="h-3.5 w-3.5" />Clear Interior</button>
            <div className="grid grid-cols-2 gap-2"><button onClick={exportRust} className="flex items-center justify-center gap-1 rounded border border-slate-700 bg-slate-900 px-2 py-1.5 font-mono text-xs text-slate-300"><Code className="h-3.5 w-3.5 text-amber-400" />{copied ? 'Copied!' : 'Copy Rust'}</button><button onClick={exportJson} className="flex items-center justify-center gap-1 rounded border border-slate-700 bg-slate-900 px-2 py-1.5 font-mono text-xs text-slate-300"><Download className="h-3.5 w-3.5 text-cyan-400" />Save JSON</button></div>
          </div>

          <div className="flex flex-col items-center justify-center overflow-auto bg-[#070913] p-6 md:col-span-8">
            <div className="grid select-none gap-0.5 rounded-lg border-2 border-cyan-500/40 bg-slate-950 p-2" style={{ gridTemplateColumns: `repeat(${size}, minmax(0, 1fr))` }} onMouseDown={() => setDragging(true)}>
              {grid.map((row, y) => row.map((value, x) => {
                const me = Math.floor(playerSpawn.x) === x && Math.floor(playerSpawn.y) === y;
                const enemy = enemySpawns.some((p) => Math.floor(p.x) === x && Math.floor(p.y) === y);
                const pickup = pickups.find((p) => Math.floor(p.x) === x && Math.floor(p.y) === y);
                return <div key={`${x}-${y}`} onClick={() => paint(x,y)} onMouseEnter={() => dragging && paint(x,y,true)} className={`flex h-7 w-7 cursor-pointer items-center justify-center rounded-xs border text-[10px] sm:h-8 sm:w-8 ${value ? 'border-slate-600 bg-slate-700' : 'border-slate-900 bg-slate-950/60'}`}>{me ? <span className="flex h-5 w-5 items-center justify-center rounded-full bg-cyan-400 font-bold text-slate-950">P</span> : enemy ? <span className="flex h-5 w-5 items-center justify-center rounded-full bg-red-500 font-bold text-white">E</span> : pickup ? <span className={`flex h-4 w-4 items-center justify-center rounded-xs font-bold text-white ${pickup.type === 'health' ? 'bg-emerald-500' : 'bg-blue-500'}`}>{pickup.type === 'health' ? '+' : '⚡'}</span> : null}</div>;
              }))}
            </div>
            <p className="mt-4 text-center font-mono text-xs text-slate-400">Click or drag to paint. Use spawn tools to place players and pickups.</p>
          </div>
        </div>
      </div>
    </div>
  );
};
