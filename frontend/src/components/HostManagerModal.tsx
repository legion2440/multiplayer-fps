import React, { useEffect, useState } from 'react';
import { Bookmark, Cpu, Globe, Play, Server, Trash2, User } from 'lucide-react';
import { sound } from '../audio/sound';
import { ServerHost } from '../types';

interface HostManagerModalProps {
  isOpen: boolean;
  onConnect: (config: { username: string; address: string; port: number; isOfflineBotMode: boolean }) => void;
  defaultUsername: string;
  defaultHost: string;
}

const DEFAULT_HOSTS: ServerHost[] = [
  { id: 'local', alias: 'Local Dev Server', address: '127.0.0.1', port: 34254, isDefault: true },
  { id: 'campus', alias: '01-Edu Campus Node Alpha', address: '198.1.1.34', port: 34254, isDefault: true },
  { id: 'zone', alias: 'Zone01 High-Speed Arena', address: '10.0.4.12', port: 34254, isDefault: true },
];

export const HostManagerModal: React.FC<HostManagerModalProps> = ({ isOpen, onConnect, defaultUsername, defaultHost }) => {
  const [username, setUsername] = useState(defaultUsername || 'Pilot_01');
  const [address, setAddress] = useState(defaultHost.split(':')[0] || '127.0.0.1');
  const [port, setPort] = useState(Number(defaultHost.split(':')[1]) || 34254);
  const [alias, setAlias] = useState('');
  const [mode, setMode] = useState<'network' | 'local_bots'>('local_bots');
  const [hosts, setHosts] = useState<ServerHost[]>(() => {
    try {
      const saved = localStorage.getItem('mazewars_hosts');
      return saved ? JSON.parse(saved) : DEFAULT_HOSTS;
    } catch {
      return DEFAULT_HOSTS;
    }
  });

  useEffect(() => {
    try { localStorage.setItem('mazewars_hosts', JSON.stringify(hosts)); } catch {}
  }, [hosts]);

  if (!isOpen) return null;

  const saveHost = () => {
    const cleanAddress = address.trim();
    if (!cleanAddress) return;
    sound.playUiClick();
    const next: ServerHost = {
      id: `host-${Date.now()}`,
      alias: alias.trim() || `${cleanAddress}:${port}`,
      address: cleanAddress,
      port: Number(port) || 34254,
      lastConnected: Date.now(),
    };
    setHosts([next, ...hosts.filter((h) => h.address !== next.address || h.port !== next.port)]);
    setAlias('');
  };

  const submit = (e: React.FormEvent) => {
    e.preventDefault();
    if (!username.trim()) return;
    sound.playUiClick();
    onConnect({
      username: username.trim(),
      address: address.trim() || '127.0.0.1',
      port: Number(port) || 34254,
      isOfflineBotMode: mode === 'local_bots',
    });
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/85 backdrop-blur-md">
      <div className="w-full max-w-xl overflow-hidden rounded-xl border border-cyan-500/40 bg-slate-950 shadow-2xl">
        <div className="flex items-center gap-3 border-b border-slate-800 bg-slate-900/60 px-6 py-5">
          <div className="rounded-lg border border-cyan-500/30 bg-cyan-500/10 p-2.5 text-cyan-400"><Server className="h-6 w-6" /></div>
          <div>
            <h2 className="font-display text-xl font-bold tracking-wide text-white">01-EDU Host Manager & Gateway</h2>
            <p className="font-mono text-xs text-slate-400">Saved host aliases plus local bot mode. Raw UDP multiplayer is handled by the native Rust client.</p>
          </div>
        </div>

        <form onSubmit={submit} className="space-y-5 p-6">
          <div>
            <label className="mb-1.5 flex items-center gap-1.5 font-mono text-xs uppercase tracking-wider text-slate-300"><User className="h-3.5 w-3.5 text-cyan-400" />Pilot Call Sign</label>
            <input required value={username} onChange={(e) => setUsername(e.target.value)} className="w-full rounded-lg border border-slate-700 bg-slate-900 px-3.5 py-2.5 font-mono text-sm text-white outline-none focus:border-cyan-400" />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <button type="button" onClick={() => setMode('local_bots')} className={`rounded-lg border p-3 text-left ${mode === 'local_bots' ? 'border-cyan-500 bg-cyan-950/40 text-cyan-200' : 'border-slate-800 bg-slate-900/50 text-slate-400'}`}>
              <div className="mb-1 flex items-center gap-2 font-mono text-xs font-bold"><Cpu className="h-4 w-4 text-cyan-400" />AI Combat Grid</div>
              <p className="text-[11px] leading-snug text-slate-400">Immediate local combat with tactical bots.</p>
            </button>
            <button type="button" onClick={() => setMode('network')} className={`rounded-lg border p-3 text-left ${mode === 'network' ? 'border-cyan-500 bg-cyan-950/40 text-cyan-200' : 'border-slate-800 bg-slate-900/50 text-slate-400'}`}>
              <div className="mb-1 flex items-center gap-2 font-mono text-xs font-bold"><Globe className="h-4 w-4 text-cyan-400" />Server Endpoint</div>
              <p className="text-[11px] leading-snug text-slate-400">Stores the endpoint for the native UDP client workflow.</p>
            </button>
          </div>

          <div>
            <div className="mb-1.5 flex items-center justify-between">
              <label className="flex items-center gap-1.5 font-mono text-xs uppercase tracking-wider text-slate-300"><Globe className="h-3.5 w-3.5 text-cyan-400" />IP Address & UDP Port</label>
              <button type="button" onClick={saveHost} className="flex items-center gap-1 font-mono text-[11px] text-cyan-400"><Bookmark className="h-3 w-3" />Save to History</button>
            </div>
            <div className="grid grid-cols-3 gap-2">
              <input required value={address} onChange={(e) => setAddress(e.target.value)} className="col-span-2 rounded-lg border border-slate-700 bg-slate-900 px-3.5 py-2.5 font-mono text-sm text-white outline-none focus:border-cyan-400" />
              <input required type="number" value={port} onChange={(e) => setPort(Number(e.target.value))} className="rounded-lg border border-slate-700 bg-slate-900 px-3.5 py-2.5 font-mono text-sm text-white outline-none focus:border-cyan-400" />
            </div>
            <input value={alias} onChange={(e) => setAlias(e.target.value)} placeholder="Optional host alias" className="mt-2 w-full rounded-lg border border-slate-800 bg-slate-900/60 px-3 py-1.5 font-mono text-xs text-slate-300 outline-none" />
          </div>

          <div>
            <div className="mb-2 flex justify-between font-mono text-[11px] uppercase tracking-wider text-slate-400"><span>Saved Hosts & Quick Reconnect</span><span>{hosts.length} saved</span></div>
            <div className="max-h-36 space-y-1.5 overflow-y-auto pr-1">
              {hosts.map((h) => (
                <div key={h.id} onClick={() => { sound.playUiClick(); setAddress(h.address); setPort(h.port); }} className="flex cursor-pointer items-center justify-between rounded-lg border border-slate-800 bg-slate-900/40 px-3 py-2 font-mono text-xs text-slate-300 hover:border-slate-700">
                  <div className="truncate"><span className="font-semibold text-white">{h.alias}</span><span className="ml-2 text-slate-500">({h.address}:{h.port})</span></div>
                  {!h.isDefault && <button type="button" onClick={(e) => { e.stopPropagation(); setHosts(hosts.filter((x) => x.id !== h.id)); }} className="p-1 text-slate-500 hover:text-red-400"><Trash2 className="h-3.5 w-3.5" /></button>}
                </div>
              ))}
            </div>
          </div>

          <button type="submit" className="flex w-full items-center justify-center gap-2 rounded-lg bg-gradient-to-r from-cyan-500 via-blue-600 to-indigo-600 px-4 py-3 font-display text-sm font-bold tracking-wider text-slate-950"><Play className="h-4 w-4 fill-current" />ENTER THE MAZE</button>
        </form>
      </div>
    </div>
  );
};
