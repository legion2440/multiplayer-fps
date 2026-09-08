use multiplayer_fps::protocol::{parse_server, ServerMessage};
use std::net::UdpSocket;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

struct ServerGuard(Child);

impl Drop for ServerGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn server_accepts_at_least_ten_udp_clients() {
    let probe = UdpSocket::bind("127.0.0.1:0").expect("reserve test port");
    let port = probe.local_addr().expect("read test port").port();
    drop(probe);

    let address = format!("127.0.0.1:{port}");
    let child = Command::new(env!("CARGO_BIN_EXE_server"))
        .args(["--bind", &address, "--bots", "0"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start server binary");
    let _server = ServerGuard(child);

    thread::sleep(Duration::from_millis(150));

    let mut clients = Vec::new();
    for index in 0..10 {
        let socket = UdpSocket::bind("127.0.0.1:0").expect("bind UDP client");
        socket.connect(&address).expect("connect UDP client");
        socket
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set read timeout");
        socket
            .send(format!("JOIN|audit_{index}\n").as_bytes())
            .expect("send JOIN");

        let deadline = Instant::now() + Duration::from_secs(2);
        let mut welcomed = false;
        let mut buf = [0u8; 8192];
        while Instant::now() < deadline {
            match socket.recv(&mut buf) {
                Ok(amount) => {
                    let packet = String::from_utf8_lossy(&buf[..amount]);
                    if matches!(parse_server(&packet), Some(ServerMessage::Welcome { .. })) {
                        welcomed = true;
                        break;
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) => {}
                Err(error) => panic!("receive WELCOME: {error}"),
            }
        }
        assert!(welcomed, "client {index} did not receive WELCOME");
        clients.push(socket);
    }

    for socket in &clients {
        socket.send(b"PING\n").expect("send PING");
    }

    let first = &clients[0];
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut observed_players = 0usize;
    let mut buf = [0u8; 8192];
    while Instant::now() < deadline {
        match first.recv(&mut buf) {
            Ok(amount) => {
                let packet = String::from_utf8_lossy(&buf[..amount]);
                if let Some(ServerMessage::State { players, .. }) = parse_server(&packet) {
                    observed_players = observed_players.max(players.len());
                    if observed_players >= 10 {
                        break;
                    }
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(error) => panic!("receive STATE: {error}"),
        }
    }

    assert!(
        observed_players >= 10,
        "expected at least 10 players in a snapshot, observed {observed_players}"
    );
}
