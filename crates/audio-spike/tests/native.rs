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
