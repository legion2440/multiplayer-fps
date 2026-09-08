# UDP protocol

Default port: **34254/UDP**.

Packets are UTF-8 text datagrams terminated by `\n`. The server is authoritative: clients send input intent, not trusted world coordinates.

## Client → server

```text
JOIN|<username>
INPUT|<sequence>|<forward>|<strafe>|<turn>
SHOOT|<sequence>
PING
NEXT
SETLEVEL|<level_index>
LEAVE
```

Input axes are clamped to `[-1, 1]`. Usernames are reduced to 16 protocol-safe ASCII characters.

## Server → client

```text
WELCOME|<player_id>|<level_index>|<spawn_x>|<spawn_y>
STATE|<tick>|<level_index>|<player>;<player>;...
SHOT|<shooter_id>|<target_id_or_0>
LEVEL|<level_index>
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
- Old snapshots are ignored by the client using the server tick number.
- The server accepts up to 64 network clients, exceeding the subject minimum of 10.
