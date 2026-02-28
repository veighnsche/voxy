use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    process::Command,
};

const PLAYER_CHAIN: [&str; 4] = ["mpv", "ffplay", "vlc", "gst-play-1.0"];

pub fn play_fixture_audio(fixture_id: u8) -> Result<(), String> {
    let fixture_path = fixture_audio_path(fixture_id);
    if !fixture_path.is_file() {
        return Err(format!(
            "audio fixture not found: {}",
            fixture_path.display()
        ));
    }

    let mut missing_players = Vec::new();
    for player in PLAYER_CHAIN {
        match launch_player(player, &fixture_path) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                missing_players.push(player);
            }
            Err(error) => {
                return Err(format!(
                    "failed to launch fixture playback with '{player}': {error}"
                ));
            }
        }
    }

    Err(format!(
        "no supported playback tool found; install one of: {}",
        missing_players.join(", ")
    ))
}

fn fixture_audio_path(fixture_id: u8) -> PathBuf {
    workspace_root()
        .join("tests")
        .join("fixtures")
        .join("audio")
        .join(format!("test_{fixture_id}.mp3"))
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root should be parent of voxy-app")
        .to_path_buf()
}

fn launch_player(player: &str, fixture_path: &Path) -> std::io::Result<()> {
    let fixture = fixture_path.to_string_lossy().to_string();
    let mut command = Command::new(player);
    match player {
        "ffplay" => {
            command.args(["-nodisp", "-autoexit", &fixture]);
        }
        "gst-play-1.0" => {
            command.args(["--no-interactive", &fixture]);
        }
        _ => {
            command.arg(&fixture);
        }
    }

    command.spawn().map(|_| ())
}
