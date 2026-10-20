use kyuutube_audio_spike::Player;
use std::{
    path::Path,
    time::{Duration, Instant},
};

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let path = args
        .get(1)
        .ok_or("Usage: kyuutube-audio-spike LOCAL_SAMPLE [SECONDS] [--null-output]")?;
    let seconds: u64 = args
        .get(2)
        .map(|s| s.parse())
        .transpose()
        .map_err(|_| "Invalid duration")?
        .unwrap_or(15);
    if !(1..=7200).contains(&seconds) {
        return Err("Duration must be 1–7200 seconds".into());
    }
    let player = Player::new(args.iter().any(|a| a == "--null-output"))?;
    player.load(Path::new(path))?;
    let started = Instant::now();
    let mut progressed = false;
    while started.elapsed().as_secs() < seconds {
        std::thread::sleep(Duration::from_secs(1));
        let snapshot = player.snapshot();
        progressed |= snapshot.position_seconds.is_some_and(|p| p > 0.1);
        println!(
            "{}",
            serde_json::to_string(&snapshot).map_err(|_| "Could not serialize snapshot")?
        );
    }
    if !progressed {
        return Err("No advancing playback timeline was observed".into());
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
