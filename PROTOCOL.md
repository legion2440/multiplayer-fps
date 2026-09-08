# UDP protocol

Default port: **34254/UDP**.

Packets are UTF-8 text datagrams terminated by `\n`. The server is authoritative: clients send movement intent and accumulated mouse-look deltas, not trusted world coordinates.

## Client -> server

```text
JOIN|<username>
INPUT|<sequence>|<forward>|<strafe>|<keyboard_turn>|<mouse_look_delta>
SHOOT|<sequence>
PING
NEXT
SETLEVEL|<level_index>
CUSTOM|<width>|<height>|<seed>|<wall_bits>
LEAVE
```

Movement axes are clamped to `[-1, 1]`. `mouse_look_delta` is an accumulated angular impulse in radians since the previous INPUT packet, so mouse sensitivity is independent of render FPS. Usernames are reduced to 16 protocol-safe ASCII characters.

Custom mazes are bounded to odd dimensions from 7 through 31 and use one `0`/`1` bit per cell. The server validates them, installs them as level index 3, and broadcasts the maze to every client.

## Server -> client

```text
WELCOME|<player_id>|<level_index>|<spawn_x>|<spawn_y>
STATE|<tick>|<level_index>|<player>;<player>;...
SHOT|<shooter_id>|<target_id_or_0>
LEVEL|<level_index>
CUSTOMLEVEL|<width>|<height>|<seed>|<wall_bits>
PONG
REJECT|<reason>
```

A player inside `STATE` is:

```text
id,name,x,y,angle,health,score,is_bot
```

The server simulates at **60 Hz** and broadcasts world snapshots every two ticks (**30 Hz**). Client rendering is independent of the network tick rate and interpolates remote players.

## UDP behaviour

- `PING` / input packets refresh the player's timeout.
- Clients are dropped after 5 seconds without activity.
- A repeated `JOIN` from the same socket returns the existing `WELCOME` packet, which makes the initial handshake tolerant of UDP packet loss.
- If the active level is custom, the server also sends its maze payload during the handshake.
- Old snapshots are ignored by the client using the server tick number.
- The server accepts up to 64 network clients, exceeding the subject minimum of 10.
