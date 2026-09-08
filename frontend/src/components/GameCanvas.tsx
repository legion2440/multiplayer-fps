import React, { useRef, useEffect, useState, useCallback } from 'react';
import {
  GameLevel,
  GameSettings,
  KillEvent,
  PickupItem,
  Player,
  Projectile,
} from '../types';
import { renderRaycasterScene } from '../engine/raycaster';
import { updateBots } from '../engine/botAI';
import { sound } from '../audio/sound';
import { HUD } from './HUD';
import { Minimap } from './Minimap';

interface GameCanvasProps {
  level: GameLevel;
  player: Player;
  otherPlayers: Player[];
  settings: GameSettings;
  serverAddress: string;
  onPlayerUpdate: (p: Player) => void;
  onOtherPlayersUpdate: (players: Player[]) => void;
  onLevelComplete?: () => void;
  onOpenScoreboard: () => void;
}

export const GameCanvas: React.FC<GameCanvasProps> = ({
  level,
  player: initialPlayer,
  otherPlayers: initialOtherPlayers,
  settings,
  serverAddress,
  onPlayerUpdate,
  onOtherPlayersUpdate,
  onOpenScoreboard,
}) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);

  // Mutable game state refs for high performance 60+ FPS loop
  const playerRef = useRef<Player>({ ...initialPlayer });
  const otherPlayersRef = useRef<Player[]>([...initialOtherPlayers]);
  const projectilesRef = useRef<Projectile[]>([]);
  const pickupsRef = useRef<PickupItem[]>([...level.pickups]);
  const botStatesRef = useRef<Map<string, any>>(new Map());

  // Input states
  const keysRef = useRef<{ [key: string]: boolean }>({});
  const isFiringRef = useRef<boolean>(false);
  const shootCooldownRef = useRef<number>(0);
  const mouseDeltaXRef = useRef<number>(0);
  const fallbackDragRef = useRef<boolean>(false);

  // Weapon bobbing & effects
  const weaponBobRef = useRef<number>(0);
  const muzzleFlashRef = useRef<number>(0);
  const damageFlashRef = useRef<number>(0);

  // Performance metrics (01-edu required > 50 FPS)
  const [fps, setFps] = useState<number>(60);
  const [frameTimeMs, setFrameTimeMs] = useState<number>(16.6);
  const [recentKills, setRecentKills] = useState<KillEvent[]>([]);
  const [activePlayer, setActivePlayer] = useState<Player>(initialPlayer);

  // Keep parent in sync periodically
  useEffect(() => {
    playerRef.current = { ...initialPlayer };
  }, [initialPlayer.id, initialPlayer.name]);

  useEffect(() => {
    pickupsRef.current = [...level.pickups];
    otherPlayersRef.current = [...initialOtherPlayers];
    projectilesRef.current = [];
    botStatesRef.current.clear();
  }, [level, initialOtherPlayers]);

  // Handle shooting
  const fireWeapon = useCallback(() => {
    const p = playerRef.current;
    if (!p.alive || p.ammo <= 0 || shootCooldownRef.current > 0) return;

    sound.playShoot();
    shootCooldownRef.current = 0.22; // Seconds between shots
    muzzleFlashRef.current = 0.08;
    p.ammo -= 1;

    // Spawn projectile from player position
    const speed = 12.0;
    const proj: Projectile = {
      id: `proj-${Date.now()}-${Math.random()}`,
      shooterId: p.id,
      x: p.x + Math.cos(p.angle) * 0.4,
      y: p.y + Math.sin(p.angle) * 0.4,
      vx: Math.cos(p.angle) * speed,
      vy: Math.sin(p.angle) * speed,
      life: 2.5,
      color: '#38bdf8',
    };
    projectilesRef.current.push(proj);
  }, []);

  // Bot shooting callback
  const handleBotShoot = useCallback((bot: Player) => {
    sound.playShoot();
    const speed = 10.0;
    const proj: Projectile = {
      id: `proj-bot-${Date.now()}-${Math.random()}`,
      shooterId: bot.id,
      x: bot.x + Math.cos(bot.angle) * 0.4,
      y: bot.y + Math.sin(bot.angle) * 0.4,
      vx: Math.cos(bot.angle) * speed,
      vy: Math.sin(bot.angle) * speed,
      life: 2.5,
      color: '#ef4444',
    };
    projectilesRef.current.push(proj);
  }, []);

  // Key Listeners
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      // Avoid capturing browser shortcuts
      if (['Tab'].includes(e.code)) {
        e.preventDefault();
        if (e.code === 'Tab') onOpenScoreboard();
        return;
      }
      if (['Space', 'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight'].includes(e.code)) {
        e.preventDefault();
      }

      keysRef.current[e.code] = true;

      if (e.code === 'Space') {
        fireWeapon();
      }
      if (e.code === 'KeyR') {
        // Quick reload
        if (playerRef.current.ammo < playerRef.current.maxAmmo) {
          playerRef.current.ammo = playerRef.current.maxAmmo;
          sound.playPickup();
        }
      }
    };

    const onKeyUp = (e: KeyboardEvent) => {
      keysRef.current[e.code] = false;
    };

    window.addEventListener('keydown', onKeyDown);
    window.addEventListener('keyup', onKeyUp);

    return () => {
      window.removeEventListener('keydown', onKeyDown);
      window.removeEventListener('keyup', onKeyUp);
    };
  }, [fireWeapon, onOpenScoreboard]);

  // Pointer lock / mouse look. Pointer lock gives normal FPS mouse control;
  // click-drag is a fallback for embedded previews that block Pointer Lock.
  useEffect(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const requestLock = () => {
      try {
        const result = canvas.requestPointerLock?.();
        if (result && typeof (result as Promise<void>).catch === 'function') {
          (result as Promise<void>).catch(() => {
            fallbackDragRef.current = true;
          });
        }
      } catch {
        fallbackDragRef.current = true;
      }
    };

    const handleMouseDown = (e: MouseEvent) => {
      if (e.button !== 0) return;
      if (document.pointerLockElement === canvas) {
        fireWeapon();
        return;
      }
      fallbackDragRef.current = true;
      requestLock();
    };

    const handleMouseUp = () => {
      fallbackDragRef.current = false;
    };

    const handleMouseMove = (e: MouseEvent) => {
      if (document.pointerLockElement === canvas || fallbackDragRef.current) {
        mouseDeltaXRef.current += e.movementX;
      }
    };

    const handlePointerLockChange = () => {
      if (document.pointerLockElement === canvas) {
        fallbackDragRef.current = false;
      }
    };

    const handlePointerLockError = () => {
      fallbackDragRef.current = true;
    };

    container.addEventListener('mousedown', handleMouseDown);
    window.addEventListener('mouseup', handleMouseUp);
    window.addEventListener('mousemove', handleMouseMove);
    document.addEventListener('pointerlockchange', handlePointerLockChange);
    document.addEventListener('pointerlockerror', handlePointerLockError);

    return () => {
      container.removeEventListener('mousedown', handleMouseDown);
      window.removeEventListener('mouseup', handleMouseUp);
      window.removeEventListener('mousemove', handleMouseMove);
      document.removeEventListener('pointerlockchange', handlePointerLockChange);
      document.removeEventListener('pointerlockerror', handlePointerLockError);
    };
  }, [fireWeapon]);

  // Main 60+ FPS Game Loop
  useEffect(() => {
    let animId: number;
    let lastTime = performance.now();
    let frameCount = 0;
    let fpsAccumulator = 0;

    const loop = (currentTime: number) => {
      const dt = Math.min(0.1, (currentTime - lastTime) / 1000); // Delta time in seconds
      lastTime = currentTime;

      frameCount++;
      fpsAccumulator += dt;
      if (fpsAccumulator >= 0.5) {
        const measuredFps = Math.round(frameCount / fpsAccumulator);
        setFps(measuredFps);
        setFrameTimeMs(1000 / Math.max(1, measuredFps));
        frameCount = 0;
        fpsAccumulator = 0;
      }

      const p = playerRef.current;
      const { grid } = level;

      // Decrement shoot cooldown
      if (shootCooldownRef.current > 0) {
        shootCooldownRef.current -= dt;
      }
      if (muzzleFlashRef.current > 0) {
        muzzleFlashRef.current -= dt;
      }
      if (damageFlashRef.current > 0) {
        damageFlashRef.current -= dt;
      }

      // Handle Respawn timer if dead
      if (!p.alive) {
        p.deathTimer = (p.deathTimer || 0) - dt;
        if (p.deathTimer <= 0) {
          p.alive = true;
          p.health = p.maxHealth;
          p.ammo = p.maxAmmo;
          p.x = level.playerSpawn.x;
          p.y = level.playerSpawn.y;
          p.angle = 0;
        }
      }

      // Respawn dead bots
      otherPlayersRef.current.forEach((bot) => {
        if (!bot.alive) {
          bot.deathTimer = (bot.deathTimer || 0) - dt;
          if (bot.deathTimer <= 0) {
            bot.alive = true;
            bot.health = bot.maxHealth;
            // Pick an enemy spawn point
            const sp = level.enemySpawns[Math.floor(Math.random() * level.enemySpawns.length)] || {
              x: 2.5,
              y: 2.5,
            };
            bot.x = sp.x;
            bot.y = sp.y;
          }
        }
      });

      // 1. Process Input & Player Movement
      if (p.alive) {
        const moveSpeed = 3.5;
        const rotSpeed = 2.4;

        // Turn via keyboard
        if (keysRef.current['ArrowLeft'] || keysRef.current['KeyQ']) {
          p.angle -= rotSpeed * dt;
        }
        if (keysRef.current['ArrowRight'] || keysRef.current['KeyE']) {
          p.angle += rotSpeed * dt;
        }

        // Turn via mouse delta
        if (mouseDeltaXRef.current !== 0) {
          p.angle += mouseDeltaXRef.current * 0.0025 * settings.mouseSensitivity;
          mouseDeltaXRef.current = 0;
        }

        // Normalize angle to [0, 2PI)
        p.angle = (p.angle + Math.PI * 2) % (Math.PI * 2);

        // Forward / Backward / Strafe
        let moveX = 0;
        let moveY = 0;

        if (keysRef.current['KeyW'] || keysRef.current['ArrowUp']) {
          moveX += Math.cos(p.angle);
          moveY += Math.sin(p.angle);
        }
        if (keysRef.current['KeyS'] || keysRef.current['ArrowDown']) {
          moveX -= Math.cos(p.angle);
          moveY -= Math.sin(p.angle);
        }
        if (keysRef.current['KeyA']) {
          moveX += Math.sin(p.angle);
          moveY -= Math.cos(p.angle);
        }
        if (keysRef.current['KeyD']) {
          moveX -= Math.sin(p.angle);
          moveY += Math.cos(p.angle);
        }

        const isMoving = moveX !== 0 || moveY !== 0;
        if (isMoving) {
          weaponBobRef.current += dt * 10;
          const len = Math.hypot(moveX, moveY);
          const step = (moveSpeed * dt) / len;
          const nextX = p.x + moveX * step;
          const nextY = p.y + moveY * step;

          // Wall collision with padding radius
          const padding = 0.28;
          const canMoveX =
            grid[Math.floor(p.y)][Math.floor(nextX + Math.sign(moveX) * padding)] === 0;
          const canMoveY =
            grid[Math.floor(nextY + Math.sign(moveY) * padding)][Math.floor(p.x)] === 0;

          if (canMoveX) p.x = nextX;
          if (canMoveY) p.y = nextY;
        }
      }

      // 2. Update Pickups
      pickupsRef.current.forEach((pickup) => {
        if (!pickup.active) {
          pickup.respawnTime -= dt;
          if (pickup.respawnTime <= 0) {
            pickup.active = true;
          }
          return;
        }

        // Check distance to player
        const dist = Math.hypot(p.x - pickup.x, p.y - pickup.y);
        if (dist < 0.65 && p.alive) {
          if (pickup.type === 'health' && p.health < p.maxHealth) {
            p.health = Math.min(p.maxHealth, p.health + 40);
            pickup.active = false;
            pickup.respawnTime = 12.0;
            sound.playPickup();
          } else if (pickup.type === 'ammo' && p.ammo < p.maxAmmo) {
            p.ammo = Math.min(p.maxAmmo, p.ammo + 8);
            pickup.active = false;
            pickup.respawnTime = 8.0;
            sound.playPickup();
          }
        }
      });

      // 3. Update Bots
      updateBots(
        otherPlayersRef.current,
        botStatesRef.current,
        p,
        grid,
        dt,
        settings.botDifficulty,
        handleBotShoot
      );

      // 4. Update Projectiles
      const nextProjectiles: Projectile[] = [];
      projectilesRef.current.forEach((proj) => {
        proj.life -= dt;
        if (proj.life <= 0) return;

        const nextX = proj.x + proj.vx * dt;
        const nextY = proj.y + proj.vy * dt;

        // Check wall collision
        const mapX = Math.floor(nextX);
        const mapY = Math.floor(nextY);
        if (
          mapY < 0 ||
          mapY >= grid.length ||
          mapX < 0 ||
          mapX >= grid[0].length ||
          grid[mapY][mapX] > 0
        ) {
          // Hit wall
          sound.playHit();
          return;
        }

        // Check collision with human player
        if (proj.shooterId !== p.id && p.alive) {
          const distToPlayer = Math.hypot(p.x - nextX, p.y - nextY);
          if (distToPlayer < 0.45) {
            p.health -= 25;
            damageFlashRef.current = 0.3;
            sound.playPlayerHurt();

            if (p.health <= 0) {
              p.alive = false;
              p.deaths += 1;
              p.deathTimer = 3.0; // 3 seconds respawn
              sound.playDeathSound?.();

              // Record kill
              const shooter = otherPlayersRef.current.find((b) => b.id === proj.shooterId);
              if (shooter) {
                shooter.kills += 1;
                shooter.score += 100;
              }
              setRecentKills((prev) => [
                ...prev.slice(-4),
                {
                  id: `k-${Date.now()}`,
                  killer: shooter?.name || 'Tactical Bot',
                  victim: p.name,
                  timestamp: Date.now(),
                },
              ]);
            }
            return;
          }
        }

        // Check collision with other bots
        let hitBot = false;
        for (const bot of otherPlayersRef.current) {
          if (proj.shooterId !== bot.id && bot.alive) {
            const dist = Math.hypot(bot.x - nextX, bot.y - nextY);
            if (dist < 0.5) {
              hitBot = true;
              bot.health -= 35;
              sound.playHit();

              if (bot.health <= 0) {
                bot.alive = false;
                bot.deaths += 1;
                bot.deathTimer = 4.0;
                sound.playKill();

                if (proj.shooterId === p.id) {
                  p.kills += 1;
                  p.score += 150;
                  setRecentKills((prev) => [
                    ...prev.slice(-4),
                    {
                      id: `k-${Date.now()}`,
                      killer: p.name,
                      victim: bot.name,
                      timestamp: Date.now(),
                    },
                  ]);
                }
              }
              break;
            }
          }
        }

        if (!hitBot) {
          proj.x = nextX;
          proj.y = nextY;
          nextProjectiles.push(proj);
        }
      });
      projectilesRef.current = nextProjectiles;

      // 5. Render Canvas
      const canvas = canvasRef.current;
      if (canvas) {
        const ctx = canvas.getContext('2d');
        if (ctx) {
          renderRaycasterScene(
            ctx,
            canvas.width,
            canvas.height,
            p,
            level,
            otherPlayersRef.current,
            projectilesRef.current,
            pickupsRef.current,
            {
              theme: settings.theme,
              fov: settings.fov,
              renderDistance: settings.renderDistance,
              crtEffect: settings.crtEffect,
            }
          );
        }
      }

      // Sync active player to React state for HUD updates
      setActivePlayer({ ...p });

      animId = requestAnimationFrame(loop);
    };

    animId = requestAnimationFrame(loop);

    return () => {
      cancelAnimationFrame(animId);
    };
  }, [level, settings, handleBotShoot]);

  // Weapon bob calculation
  const bobOffsetY = Math.sin(weaponBobRef.current) * 8;
  const bobOffsetX = Math.cos(weaponBobRef.current * 0.5) * 6;

  return (
    <div
      ref={containerRef}
      className="relative w-full h-[620px] max-h-[75vh] bg-black rounded-xl overflow-hidden border border-cyan-500/30 shadow-2xl select-none"
    >
      {/* 3D Raycasting Canvas */}
      <canvas
        ref={canvasRef}
        width={720}
        height={450}
        className="w-full h-full block object-cover cursor-crosshair"
      />

      {/* Retro CRT Scanlines overlay */}
      {settings.crtEffect && (
        <div className="absolute inset-0 pointer-events-none crt-scanlines z-1" />
      )}

      {/* Damage Flash Red Vignette */}
      {damageFlashRef.current > 0 && (
        <div
          className="absolute inset-0 pointer-events-none z-2 transition-opacity duration-75"
          style={{
            backgroundColor: 'rgba(239, 68, 68, 0.35)',
            boxShadow: 'inset 0 0 80px rgba(220, 38, 38, 0.8)',
          }}
        />
      )}

      {/* Weapon Billboard (Classic First-Person Pulse Cannon) */}
      <div
        className="absolute bottom-0 right-1/2 translate-x-1/2 pointer-events-none z-3 transition-transform duration-75"
        style={{
          transform: `translate(calc(50% + ${bobOffsetX}px), ${bobOffsetY}px)`,
        }}
      >
        <div className="relative flex flex-col items-center">
          {/* Muzzle Flash */}
          {muzzleFlashRef.current > 0 && (
            <div className="absolute -top-10 w-16 h-16 rounded-full bg-cyan-300 blur-sm shadow-[0_0_30px_#38bdf8] animate-ping" />
          )}

          {/* Retro Stylized Blaster Barrel */}
          <div className="w-14 h-28 bg-gradient-to-t from-slate-950 via-slate-800 to-slate-700 border-x-2 border-t-2 border-cyan-500/60 rounded-t-lg shadow-2xl flex flex-col items-center justify-between p-1.5">
            <div className="w-6 h-4 bg-cyan-400 rounded-sm shadow-[0_0_12px_#38bdf8]" />
            <div className="w-10 h-2 bg-slate-900 rounded-xs" />
            <div className="w-12 h-6 bg-slate-900/80 rounded border border-cyan-500/30 flex items-center justify-center text-[8px] font-mono text-cyan-300">
              MK-IV
            </div>
          </div>
        </div>
      </div>

      {/* Head-Up Display (HUD) with live FPS telemetry */}
      <HUD
        player={activePlayer}
        fps={fps}
        frameTime={frameTimeMs}
        recentKills={recentKills}
        ping={activePlayer.ping || 14}
        serverAddress={serverAddress}
      />

      {/* Minimap Widget (Top-Right overlay) */}
      {settings.showMinimap && (
        <div className="absolute top-4 right-4 z-20 pointer-events-auto">
          <Minimap
            level={level}
            player={activePlayer}
            otherPlayers={otherPlayersRef.current}
            pickups={pickupsRef.current}
            showRays={settings.showRaylinesOnMinimap}
            fov={settings.fov}
          />
        </div>
      )}

      {/* Respawn Overlay if Dead */}
      {!activePlayer.alive && (
        <div className="absolute inset-0 bg-red-950/80 backdrop-blur-sm z-30 flex flex-col items-center justify-center text-center p-6 animate-fade-in">
          <h2 className="text-3xl font-display font-black text-red-500 tracking-wider mb-2">
            HULL INTEGRITY COMPROMISED
          </h2>
          <p className="text-slate-300 font-mono text-sm mb-4">
            Reconstructing optic telemetry in {(activePlayer.deathTimer || 0).toFixed(1)}s...
          </p>
          <div className="w-48 h-2 bg-slate-900 rounded-full overflow-hidden border border-red-500/50">
            <div
              className="h-full bg-red-500 transition-all duration-100"
              style={{
                width: `${Math.max(0, 100 - ((activePlayer.deathTimer || 0) / 3) * 100)}%`,
              }}
            />
          </div>
        </div>
      )}

      {/* Controls helper bar at bottom-center */}
      <div className="absolute bottom-2 left-1/2 -translate-x-1/2 z-10 pointer-events-none text-[10px] font-mono text-slate-400 bg-slate-950/80 px-3 py-1 rounded-full border border-slate-800 backdrop-blur-xs flex items-center gap-3">
        <span><strong className="text-cyan-400">WASD</strong> Move</span>
        <span>•</span>
        <span><strong className="text-cyan-400">Mouse / ←→</strong> Turn</span>
        <span>•</span>
        <span><strong className="text-cyan-400">Space / Click</strong> Fire Blaster</span>
        <span>•</span>
        <span><strong className="text-cyan-400">TAB</strong> Leaderboard</span>
      </div>
    </div>
  );
};
