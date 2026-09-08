/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import React, { useState, useCallback } from 'react';
import { GameLevel, GameSettings, Player } from './types';
import { PRESET_LEVELS } from './engine/presetLevels';
import { generateProceduralMaze } from './engine/mazeGenerator';
import { GameCanvas } from './components/GameCanvas';
import { HostManagerModal } from './components/HostManagerModal';
import { LevelEditor } from './components/LevelEditor';
import { Scoreboard } from './components/Scoreboard';
import { SettingsModal } from './components/SettingsModal';
import { sound } from './audio/sound';
import {
  Server,
  Grid,
  Sparkles,
  Trophy,
  Settings as SettingsIcon,
  CheckCircle2,
} from 'lucide-react';

export default function App() {
  // Current Level
  const [currentLevel, setCurrentLevel] = useState<GameLevel>(PRESET_LEVELS[0]);

  // Server & Connection state
  const [serverAddress, setServerAddress] = useState<string>('127.0.0.1:34254');
  const [isOfflineBotMode, setIsOfflineBotMode] = useState<boolean>(true);

  // Human Player
  const [player, setPlayer] = useState<Player>({
    id: 'player-local',
    name: 'Pilot_01',
    x: PRESET_LEVELS[0].playerSpawn.x,
    y: PRESET_LEVELS[0].playerSpawn.y,
    angle: 0,
    health: 100,
    maxHealth: 100,
    ammo: 24,
    maxAmmo: 24,
    score: 0,
    kills: 0,
    deaths: 0,
    isBot: false,
    color: '#38bdf8',
    avatarType: 'eyeball',
    ping: 12,
    alive: true,
  });

  // Bots / Network Players
  const [otherPlayers, setOtherPlayers] = useState<Player[]>(() => {
    return PRESET_LEVELS[0].enemySpawns.map((sp, idx) => ({
      id: `bot-${idx}`,
      name: `Combatant_0${idx + 1}`,
      x: sp.x,
      y: sp.y,
      angle: Math.PI * (idx * 0.5),
      health: 100,
      maxHealth: 100,
      ammo: 100,
      maxAmmo: 100,
      score: 0,
      kills: 0,
      deaths: 0,
      isBot: true,
      color: '#ef4444',
      avatarType: 'eyeball',
      ping: 8 + idx * 3,
      alive: true,
    }));
  });

  // Settings
  const [settings, setSettings] = useState<GameSettings>({
    soundEnabled: true,
    soundVolume: 0.4,
    theme: 'classic',
    showMinimap: true,
    showFpsCounter: true,
    showRaylinesOnMinimap: true,
    mouseSensitivity: 1.0,
    fov: 1.15, // ~66 deg
    renderDistance: 20,
    crtEffect: false,
    botDifficulty: 'Medium',
    botCount: 4,
  });

  // Modals
  const [isHostModalOpen, setIsHostModalOpen] = useState(false);
  const [isLevelEditorOpen, setIsLevelEditorOpen] = useState(false);
  const [isScoreboardOpen, setIsScoreboardOpen] = useState(false);
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);

  // Switch level
  const handleSelectLevel = useCallback((lvl: GameLevel) => {
    sound.playUiClick();
    setCurrentLevel(lvl);

    setPlayer((prev) => ({
      ...prev,
      x: lvl.playerSpawn.x,
      y: lvl.playerSpawn.y,
      angle: 0,
      health: prev.maxHealth,
      ammo: prev.maxAmmo,
      alive: true,
    }));

    const newBots: Player[] = lvl.enemySpawns.map((sp, idx) => ({
      id: `bot-${idx}`,
      name: `Hostile_0${idx + 1}`,
      x: sp.x,
      y: sp.y,
      angle: Math.random() * Math.PI * 2,
      health: 100,
      maxHealth: 100,
      ammo: 100,
      maxAmmo: 100,
      score: 0,
      kills: 0,
      deaths: 0,
      isBot: true,
      color: '#ef4444',
      avatarType: 'eyeball',
      ping: 10 + idx * 4,
      alive: true,
    }));
    setOtherPlayers(newBots);
  }, []);

  const handleGenerateProcedural = useCallback(() => {
    sound.playUiClick();
    const diff = settings.botDifficulty === 'Easy' ? 'Novice' : settings.botDifficulty === 'Hard' ? 'Master' : 'Intermediate';
    const dim = diff === 'Novice' ? 14 : diff === 'Master' ? 22 : 18;
    const procLevel = generateProceduralMaze(dim, dim, diff);
    handleSelectLevel(procLevel);
  }, [settings.botDifficulty, handleSelectLevel]);

  const handleConnectHost = useCallback(
    (config: { username: string; address: string; port: number; isOfflineBotMode: boolean }) => {
      setServerAddress(`${config.address}:${config.port}`);
      setIsOfflineBotMode(config.isOfflineBotMode);
      setPlayer((prev) => ({
        ...prev,
        name: config.username,
        ping: config.isOfflineBotMode ? 8 : 28,
      }));
      setIsHostModalOpen(false);
    },
    []
  );

  return (
    <div className="min-h-screen bg-[#070911] text-slate-100 flex flex-col font-mono selection:bg-cyan-500/30 selection:text-cyan-200">
      <header className="border-b border-slate-800 bg-slate-950/80 backdrop-blur-md sticky top-0 z-30">
        <div className="max-w-7xl mx-auto px-4 py-2.5 flex flex-wrap items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="w-8 h-8 rounded-lg bg-gradient-to-br from-cyan-500 to-blue-600 flex items-center justify-center font-display font-black text-slate-950 text-sm shadow-md shadow-cyan-500/20">MW</div>
            <div>
              <div className="flex items-center gap-2">
                <span className="font-display font-black tracking-wider text-base text-white">MAZE WARS 3D</span>
                <span className="text-[10px] font-mono px-2 py-0.5 rounded bg-cyan-500/10 text-cyan-400 border border-cyan-500/30 font-semibold">01-EDU MULTIPLAYER FPS</span>
              </div>
              <div className="text-[11px] text-slate-400 font-mono flex items-center gap-2">
                <span>DDA Raycasting</span><span>•</span><span>UDP Backend + Native Client</span><span>•</span>
                <span className="text-emerald-400 font-semibold">&gt;50 FPS Target</span>
              </div>
            </div>
          </div>

          <div className="flex items-center gap-1 bg-slate-900/80 p-1 rounded-lg border border-slate-800">
            {PRESET_LEVELS.map((lvl, index) => {
              const isCurrent = currentLevel.id === lvl.id;
              return (
                <button key={lvl.id} onClick={() => handleSelectLevel(lvl)} className={`px-3 py-1.5 rounded-md text-xs font-mono transition-all cursor-pointer ${isCurrent ? 'bg-cyan-500/20 text-cyan-300 border border-cyan-500/40 font-bold shadow-sm' : 'text-slate-400 hover:text-slate-200 hover:bg-slate-800/60'}`} title={lvl.description}>
                  L{index + 1}: {lvl.difficulty}
                </button>
              );
            })}
            <button onClick={handleGenerateProcedural} className={`flex items-center gap-1 px-3 py-1.5 rounded-md text-xs font-mono transition-all cursor-pointer ${currentLevel.id.startsWith('procedural') ? 'bg-purple-500/20 text-purple-300 border border-purple-500/40 font-bold' : 'text-purple-400 hover:bg-purple-950/30'}`} title="Generate new labyrinth automatically using DFS algorithm">
              <Sparkles className="w-3.5 h-3.5" /> Procedural
            </button>
          </div>

          <div className="flex items-center gap-2">
            <button onClick={() => { sound.playUiClick(); setIsHostModalOpen(true); }} className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-300 rounded-lg text-xs font-mono transition-colors cursor-pointer" title="Connect to UDP server / Manage host aliases">
              <Server className="w-3.5 h-3.5 text-cyan-400" /><span className="hidden sm:inline">Gateway:</span><span className="text-cyan-400 font-bold truncate max-w-[90px]">{serverAddress.split(':')[0]}</span>
            </button>
            <button onClick={() => { sound.playUiClick(); setIsLevelEditorOpen(true); }} className="flex items-center gap-1.5 px-3 py-1.5 bg-slate-900 hover:bg-slate-800 border border-slate-700 text-slate-300 rounded-lg text-xs font-mono transition-colors cursor-pointer" title="Open Visual Maze Editor">
              <Grid className="w-3.5 h-3.5 text-emerald-400" /><span className="hidden sm:inline">Editor</span>
            </button>
            <button onClick={() => { sound.playUiClick(); setIsScoreboardOpen(true); }} className="p-2 bg-slate-900 hover:bg-slate-800 border border-slate-800 text-slate-400 hover:text-white rounded-lg transition-colors cursor-pointer" title="Match Leaderboard [TAB]"><Trophy className="w-4 h-4 text-amber-400" /></button>
            <button onClick={() => { sound.playUiClick(); setIsSettingsOpen(true); }} className="p-2 bg-slate-900 hover:bg-slate-800 border border-slate-800 text-slate-400 hover:text-white rounded-lg transition-colors cursor-pointer" title="Settings & Themes"><SettingsIcon className="w-4 h-4" /></button>
          </div>
        </div>
      </header>

      <main className="flex-1 max-w-7xl w-full mx-auto p-4 flex flex-col gap-4">
        <div className="flex items-center justify-between text-xs text-slate-400">
          <div className="flex items-center gap-2"><span className="w-2 h-2 rounded-full bg-cyan-400 animate-pulse" /><span className="text-white font-bold">{currentLevel.name}</span><span>•</span><span>{currentLevel.width}x{currentLevel.height} Sector</span><span>•</span><span className="text-slate-400">{currentLevel.description}</span></div>
          <div className="hidden md:flex items-center gap-3"><span>Click viewport to lock mouse cursor</span><span>•</span><span>Press <kbd className="px-1.5 py-0.5 bg-slate-800 rounded text-slate-300 font-bold">R</kbd> to reload blaster</span></div>
        </div>

        <GameCanvas level={currentLevel} player={player} otherPlayers={otherPlayers} settings={settings} serverAddress={serverAddress} onPlayerUpdate={setPlayer} onOtherPlayersUpdate={setOtherPlayers} onOpenScoreboard={() => setIsScoreboardOpen(true)} />

        <div className="grid grid-cols-1 md:grid-cols-4 gap-3 pt-2">
          <div className="p-3.5 bg-slate-950/60 border border-slate-800 rounded-xl"><div className="flex items-center gap-2 text-cyan-400 text-xs font-bold mb-1"><CheckCircle2 className="w-4 h-4 text-emerald-400" />Authentic Maze Wars 3D</div><p className="text-[11px] text-slate-400 leading-relaxed">DDA Raycasting with iconic Eyeball avatars that track player orientation, retro wireframes, and &gt;50 FPS requirement.</p></div>
          <div className="p-3.5 bg-slate-950/60 border border-slate-800 rounded-xl"><div className="flex items-center gap-2 text-cyan-400 text-xs font-bold mb-1"><CheckCircle2 className="w-4 h-4 text-emerald-400" />UDP Client-Server Architecture</div><p className="text-[11px] text-slate-400 leading-relaxed">Rust UDP server and native network client remain the authoritative multiplayer path; this web client currently provides the ported game UI and local bot mode.</p></div>
          <div className="p-3.5 bg-slate-950/60 border border-slate-800 rounded-xl"><div className="flex items-center gap-2 text-cyan-400 text-xs font-bold mb-1"><CheckCircle2 className="w-4 h-4 text-emerald-400" />3 Levels + Procedural Generator</div><p className="text-[11px] text-slate-400 leading-relaxed">Novice, Intermediate, and Master sectors with progressive dead ends, plus Recursive Backtracker procedural generation.</p></div>
          <div className="p-3.5 bg-slate-950/60 border border-slate-800 rounded-xl"><div className="flex items-center gap-2 text-cyan-400 text-xs font-bold mb-1"><CheckCircle2 className="w-4 h-4 text-emerald-400" />Level Editor & Custom Maps</div><p className="text-[11px] text-slate-400 leading-relaxed">In-game grid painter with instant 3D testing plus JSON and Rust matrix export for custom arenas.</p></div>
        </div>
      </main>

      <HostManagerModal isOpen={isHostModalOpen} onConnect={handleConnectHost} defaultUsername={player.name} defaultHost={serverAddress} />
      <LevelEditor isOpen={isLevelEditorOpen} onClose={() => setIsLevelEditorOpen(false)} onPlayLevel={(customLevel) => { handleSelectLevel(customLevel); }} />
      <Scoreboard isOpen={isScoreboardOpen} onClose={() => setIsScoreboardOpen(false)} players={[player, ...otherPlayers]} levelName={currentLevel.name} />
      <SettingsModal isOpen={isSettingsOpen} onClose={() => setIsSettingsOpen(false)} settings={settings} onUpdateSettings={setSettings} />
    </div>
  );
}
