export type VisualTheme = 'classic' | 'cyberpunk' | 'green' | 'amber';

export interface Position { x: number; y: number; }

export interface Player {
  id: string;
  name: string;
  x: number;
  y: number;
  angle: number;
  health: number;
  maxHealth: number;
  ammo: number;
  maxAmmo: number;
  score: number;
  kills: number;
  deaths: number;
  isBot: boolean;
  color: string;
  avatarType: 'eyeball' | 'cyborg' | 'recon';
  ping: number;
  alive: boolean;
  deathTimer?: number;
}

export interface Projectile {
  id: string;
  shooterId: string;
  x: number;
  y: number;
  vx: number;
  vy: number;
  life: number;
  color: string;
}

export interface PickupItem {
  id: string;
  type: 'health' | 'ammo' | 'speed';
  x: number;
  y: number;
  active: boolean;
  respawnTime: number;
}

export interface GameLevel {
  id: string;
  name: string;
  difficulty: 'Novice' | 'Intermediate' | 'Master' | 'Custom';
  width: number;
  height: number;
  grid: number[][];
  playerSpawn: Position;
  enemySpawns: Position[];
  pickups: PickupItem[];
  description: string;
}

export interface ServerHost {
  id: string;
  alias: string;
  address: string;
  port: number;
  lastConnected?: number;
  ping?: number;
  isDefault?: boolean;
}

export interface KillEvent { id: string; killer: string; victim: string; timestamp: number; }

export interface NetworkPacket {
  type: 'JOIN' | 'POS' | 'SHOOT' | 'HIT' | 'RESPAWN' | 'PING' | 'PONG' | 'CHAT';
  senderId: string;
  payload: unknown;
  timestamp: number;
}

export interface GameSettings {
  soundEnabled: boolean;
  soundVolume: number;
  theme: VisualTheme;
  showMinimap: boolean;
  showFpsCounter: boolean;
  showRaylinesOnMinimap: boolean;
  mouseSensitivity: number;
  fov: number;
  renderDistance: number;
  crtEffect: boolean;
  botDifficulty: 'Easy' | 'Medium' | 'Hard';
  botCount: number;
}
