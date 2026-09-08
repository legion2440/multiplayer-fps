# UDP protocol

Default port: **34254/UDP**.

Packets are UTF-8 text datagrams terminated by `\n`. The server is authoritative: clients send movement intent and their current view angle, not trusted world coordinates.

## Client -> server

```text
JOIN|<username>
INPUT|<sequence>|<forward>|<strafe>|<keyboard_turn>|<view_angle>
SHOOT|<sequence>
PING
NEXT
SETLEVEL|<level_index>
CUSTOM|<width>|<height>|<seed>|<wall_bits>
LEAVE
```

Movement axes are clamped to `[-1, 1]`. `view_angle` is the client's absolute current angle in radians; sending absolute state makes a later INPUT repair any angle packet lost by UDP. Mouse sensitivity remains independent of render FPS because frame mouse displacement is applied directly to the local angle. Usernames are reduced to 16 protocol-safe ASCII characters.

Custom mazes are bounded to odd dimensions from 7 through 31, use one `0`/`1` bit per cell, keep solid outer walls, and must have one connected open region. The server validates them, installs them as level index 3, and broadcasts the maze to every client.

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
- If the active level is custom, the server also sends its maze payload during the handshake. A client that sees an unknown level index re-sends `JOIN`; periodic `STATE` packets therefore recover a lost `CUSTOMLEVEL` datagram.
- Old snapshots are ignored by the client using the server tick number.
- The server accepts up to 64 network clients, exceeding the subject minimum of 10.
