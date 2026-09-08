import React from 'react';
import { GameSettings, VisualTheme } from '../types';
import { Settings, Volume2, VolumeX, Eye, Cpu, X } from 'lucide-react';
import { sound } from '../audio/sound';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
  settings: GameSettings;
  onUpdateSettings: (newSettings: GameSettings) => void;
}

export const SettingsModal: React.FC<SettingsModalProps> = ({ isOpen, onClose, settings, onUpdateSettings }) => {
  if (!isOpen) return null;

  const handleThemeChange = (theme: VisualTheme) => {
    sound.playUiClick();
    onUpdateSettings({ ...settings, theme });
  };

  const handleAudioToggle = () => {
    const nextSound = !settings.soundEnabled;
    sound.setEnabled(nextSound);
    onUpdateSettings({ ...settings, soundEnabled: nextSound });
  };

  const handleVolumeChange = (vol: number) => {
    sound.setVolume(vol);
    onUpdateSettings({ ...settings, soundVolume: vol });
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-fade-in">
      <div className="w-full max-w-lg bg-slate-950 border border-cyan-500/40 rounded-xl shadow-2xl overflow-hidden">
        <div className="flex items-center justify-between px-6 py-4 border-b border-slate-800 bg-slate-900/60">
          <div className="flex items-center gap-3">
            <div className="p-2 rounded-lg bg-cyan-500/10 border border-cyan-500/30 text-cyan-400"><Settings className="w-5 h-5" /></div>
            <div><h3 className="text-base font-display font-bold text-white tracking-wide">Simulation & Graphics Configuration</h3><p className="text-xs text-slate-400 font-mono">Visual rendering pipelines, bots, and audio synthesis</p></div>
          </div>
          <button onClick={onClose} className="p-1.5 text-slate-400 hover:text-white rounded-lg hover:bg-slate-800 transition-colors cursor-pointer"><X className="w-5 h-5" /></button>
        </div>

        <div className="p-6 space-y-5">
          <div>
            <label className="block text-xs font-mono uppercase tracking-wider text-slate-300 mb-2 flex items-center gap-1.5"><Eye className="w-4 h-4 text-cyan-400" />Render Architecture & Aesthetic</label>
            <div className="grid grid-cols-2 gap-2">
              {[
                { id: 'classic', label: 'Classic 1974 (Maze Wars)', desc: 'Original high-contrast wireframe with eyeball avatars' },
                { id: 'cyberpunk', label: 'Cyberpunk Neon Vector', desc: 'Saturated neon gradients and floor perspective lines' },
                { id: 'green', label: 'Phosphor Green CRT', desc: 'Vintage military terminal raster glow' },
                { id: 'amber', label: 'Amber Monochrome', desc: 'Industrial amber monochrome display' },
              ].map((th) => (
                <button key={th.id} onClick={() => handleThemeChange(th.id as VisualTheme)} className={`p-2.5 rounded-lg border text-left transition-all cursor-pointer ${settings.theme === th.id ? 'border-cyan-400 bg-cyan-950/40 text-cyan-200' : 'border-slate-800 bg-slate-900/40 text-slate-400 hover:border-slate-700'}`}>
                  <div className="text-xs font-mono font-bold mb-0.5">{th.label}</div><div className="text-[10px] text-slate-500 leading-tight">{th.desc}</div>
                </button>
              ))}
            </div>
          </div>

          <div>
            <label className="block text-xs font-mono uppercase tracking-wider text-slate-300 mb-2 flex items-center gap-1.5"><Cpu className="w-4 h-4 text-cyan-400" />Tactical AI Difficulty (01-Edu Bonus)</label>
            <div className="grid grid-cols-3 gap-2">
              {(['Easy', 'Medium', 'Hard'] as const).map((diff) => (
                <button key={diff} onClick={() => { sound.playUiClick(); onUpdateSettings({ ...settings, botDifficulty: diff }); }} className={`py-2 rounded-lg font-mono text-xs border transition-colors cursor-pointer ${settings.botDifficulty === diff ? 'bg-cyan-500/20 border-cyan-400 text-cyan-300 font-bold' : 'bg-slate-900 border-slate-800 text-slate-400 hover:border-slate-700'}`}>{diff}</button>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3 pt-2 border-t border-slate-800">
            <label className="flex items-center justify-between p-2.5 rounded-lg bg-slate-900/50 border border-slate-800 cursor-pointer"><span className="text-xs font-mono text-slate-300">CRT Scanlines</span><input type="checkbox" checked={settings.crtEffect} onChange={(e) => { sound.playUiClick(); onUpdateSettings({ ...settings, crtEffect: e.target.checked }); }} className="w-4 h-4 accent-cyan-400 rounded" /></label>
            <label className="flex items-center justify-between p-2.5 rounded-lg bg-slate-900/50 border border-slate-800 cursor-pointer"><span className="text-xs font-mono text-slate-300">Tactical Minimap</span><input type="checkbox" checked={settings.showMinimap} onChange={(e) => { sound.playUiClick(); onUpdateSettings({ ...settings, showMinimap: e.target.checked }); }} className="w-4 h-4 accent-cyan-400 rounded" /></label>
          </div>

          <div className="pt-2 border-t border-slate-800">
            <div className="flex items-center justify-between mb-2">
              <label className="text-xs font-mono uppercase tracking-wider text-slate-300 flex items-center gap-1.5">{settings.soundEnabled ? <Volume2 className="w-4 h-4 text-cyan-400" /> : <VolumeX className="w-4 h-4 text-slate-500" />}Web Audio FX Synthesizer</label>
              <button onClick={handleAudioToggle} className="text-xs font-mono text-cyan-400 hover:text-cyan-300 cursor-pointer">{settings.soundEnabled ? 'Disable' : 'Enable'}</button>
            </div>
            <input type="range" min="0" max="1" step="0.05" value={settings.soundVolume} onChange={(e) => handleVolumeChange(parseFloat(e.target.value))} disabled={!settings.soundEnabled} className="w-full accent-cyan-400 cursor-pointer" />
          </div>
        </div>

        <div className="px-6 py-3 border-t border-slate-800 bg-slate-900/40 flex justify-end"><button onClick={onClose} className="px-4 py-2 bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-mono font-bold text-xs rounded-lg transition-colors cursor-pointer">Apply & Return</button></div>
      </div>
    </div>
  );
};
