use kyuutube_audio_spike::{local_media, Player};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};

#[test]
fn remote_url_and_missing_sample_are_rejected() {
    assert!(local_media(Path::new("https://www.youtube.com/watch?v=example")).is_err());
    assert!(local_media(Path::new("/nonexistent-kyuutube.wav")).is_err());
    assert!(local_media(Path::new(env!("CARGO_MANIFEST_DIR"))).is_err());
}

#[test]
fn local_playback_pause_seek_and_validation() {
    let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/fixtures/tone.wav");
    assert!(
        sample.is_file(),
        "Generate tools/fixtures/tone.wav with tools/make_sample.py first"
    );
    let player =
        Player::new(true).expect("Install libmpv for the required native integration test");
    player.load(&sample).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while player.snapshot().position_seconds.unwrap_or(0.0) < 0.2 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(50));
    }
    assert!(
        player.snapshot().position_seconds.unwrap_or(0.0) >= 0.2,
        "Decoder timeline did not advance"
    );
    player.pause(true).unwrap();
    let paused = player.snapshot();
    assert!(paused.paused);
    thread::sleep(Duration::from_millis(300));
    assert!(
        (player.snapshot().position_seconds.unwrap() - paused.position_seconds.unwrap()).abs()
            < 0.15
    );
    player.seek(5.0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while (player.snapshot().position_seconds.unwrap_or(0.0) - 5.0).abs() > 0.5
        && Instant::now() < deadline
    {
        thread::sleep(Duration::from_millis(50));
    }
    assert!((player.snapshot().position_seconds.unwrap() - 5.0).abs() < 0.5);
    assert!(player.seek(f64::NAN).is_err());
    assert!(player.seek(-1.0).is_err());
    assert!(player.volume(f64::INFINITY).is_err());
    assert!(player.volume(-1.0).is_err());
    assert!(player.volume(101.0).is_err());
    player.volume(25.0).unwrap();
    player.pause(false).unwrap();
    assert!(!player.snapshot().paused);
    // libmpv must report command errors instead of terminating the process.
    assert!(player
        .command(&["kyuutube-command-that-does-not-exist"])
        .is_err());
}

#[test]
fn signed_google_media_addresses_are_bounded_and_not_arbitrary_network_input() {
    use kyuutube_audio_spike::RemoteMedia;
    use std::time::{SystemTime, UNIX_EPOCH};
    let expiry = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;
    assert!(RemoteMedia::google_audio(format!(
        "https://r1.googlevideo.com/videoplayback?expire={expiry}"
    ))
    .is_ok());
    for address in [
        format!("http://r1.googlevideo.com/videoplayback?expire={expiry}"),
        format!("https://googlevideo.com.evil.invalid/videoplayback?expire={expiry}"),
        format!("https://127.0.0.1/videoplayback?expire={expiry}"),
        format!("https://user:password@r1.googlevideo.com/videoplayback?expire={expiry}"),
        format!("https://r1.googlevideo.com:444/videoplayback?expire={expiry}"),
        format!("https://r1.googlevideo.com/videoplayback?expire={expiry}&expire={expiry}"),
        "https://r1.googlevideo.com/videoplayback?expire=1".into(),
    ] {
        assert!(RemoteMedia::google_audio(address).is_err());
    }
}

#[test]
fn native_network_playback_advances_pauses_seeks_and_stops() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        },
    };
    let data = Arc::new(
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/fixtures/tone.wav"))
            .unwrap(),
    );
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    server.set_nonblocking(true).unwrap();
    let address = server.local_addr().unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let requests = Arc::new(AtomicUsize::new(0));
    let stopping = stop.clone();
    let count = requests.clone();
    let worker = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !stopping.load(Ordering::SeqCst) && Instant::now() < deadline {
            let (mut stream, _) = match server.accept() {
                Ok(connection) => connection,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                }
                Err(_) => break,
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            stream
                .set_write_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut bytes = [0u8; 8192];
            let n = stream.read(&mut bytes).unwrap_or(0);
            let header = String::from_utf8_lossy(&bytes[..n]);
            let offset = header
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("range: bytes=")
                        .and_then(|value| value.split('-').next())
                        .and_then(|value| value.parse::<usize>().ok())
                })
                .unwrap_or(0)
                .min(data.len());
            count.fetch_add(1, Ordering::SeqCst);
            let body = &data[offset..];
            let status = if offset > 0 {
                "206 Partial Content"
            } else {
                "200 OK"
            };
            let range = if offset > 0 {
                format!(
                    "Content-Range: bytes {offset}-{}/{}\r\n",
                    data.len() - 1,
                    data.len()
                )
            } else {
                String::new()
            };
            if write!(stream,"HTTP/1.1 {status}\r\nContent-Type: audio/wav\r\nAccept-Ranges: bytes\r\n{range}Content-Length: {}\r\nConnection: close\r\n\r\n", body.len()).is_ok() { let _ = stream.write_all(body); }
        }
    });
    let player = Player::new(true).unwrap();
    // Loopback fixture only. Production load_stream accepts HTTPS Google media
    // leases and retains inherited remote-network proxy/TLS configuration.
    player.command(&["set", "http-proxy", ""]).unwrap();
    player
        .command(&["loadfile", &format!("http://{address}/tone.wav"), "replace"])
        .unwrap();
    player.pause(false).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while player.snapshot().position_seconds.unwrap_or(0.0) < 0.2 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(50));
    }
    assert!(player.snapshot().position_seconds.unwrap_or(0.0) >= 0.2);
    player.pause(true).unwrap();
    let position = player.snapshot().position_seconds.unwrap();
    thread::sleep(Duration::from_millis(200));
    assert!((player.snapshot().position_seconds.unwrap() - position).abs() < 0.15);
    player.seek(4.0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while (player.snapshot().position_seconds.unwrap_or(0.0) - 4.0).abs() > 0.5
        && Instant::now() < deadline
    {
        thread::sleep(Duration::from_millis(50));
    }
    assert!((player.snapshot().position_seconds.unwrap() - 4.0).abs() < 0.5);
    player.stop().unwrap();
    assert!(requests.load(Ordering::SeqCst) >= 1);
    stop.store(true, Ordering::SeqCst);
    worker.join().unwrap();
}
