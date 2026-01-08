#![cfg(feature = "fixture-mode")]

mod support;

use portable_pty::PtySize;
use std::time::Duration;

use support::pty::{Key, PtyHarness};

fn resolve_bin() -> String {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_azul") {
        return path;
    }

    let mut path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    if cfg!(windows) {
        path.push("azul.exe");
    } else {
        path.push("azul");
    }
    path.to_string_lossy().to_string()
}

fn spawn_app() -> (PtyHarness, tempfile::TempDir) {
    let bin = resolve_bin();
    let size = PtySize {
        rows: 40,
        cols: 120,
        pixel_width: 0,
        pixel_height: 0,
    };

    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let fixture_root = support::fixture_root();
    let config_path = fixture_root.join("config/test_config.json");

    let env = vec![
        (
            "AZUL_FIXTURE_DIR".to_string(),
            fixture_root.to_string_lossy().to_string(),
        ),
        (
            "AZUL_CONFIG_PATH".to_string(),
            config_path.to_string_lossy().to_string(),
        ),
        (
            "XDG_DATA_HOME".to_string(),
            temp_dir.path().to_string_lossy().to_string(),
        ),
        ("TERM".to_string(), "xterm-256color".to_string()),
    ];

    (PtyHarness::spawn(&bin, size, &env), temp_dir)
}

#[test]
fn e2e_help_panel() {
    let (mut app, _temp_dir) = spawn_app();

    app.wait_for("WELCOME TO AZUL", Duration::from_secs(3));
    app.send("?");
    app.wait_for("azul help", Duration::from_secs(3));
    app.send_key(Key::Esc);
    app.wait_for("WELCOME TO AZUL", Duration::from_secs(3));
    app.send("q");
    app.wait_for_exit(Duration::from_secs(3));
}

#[test]
fn e2e_fixture_navigation() {
    let (mut app, _temp_dir) = spawn_app();

    app.wait_for("WELCOME TO AZUL", Duration::from_secs(3));
    app.send("/");
    app.send("https://fixture.local/article");
    app.send_key(Key::Enter);
    app.wait_for("FIXTURE ARTICLE", Duration::from_secs(4));
    app.send("q");
    app.wait_for_exit(Duration::from_secs(3));
}

#[test]
fn e2e_focus_mode_and_chat_toggle() {
    let (mut app, _temp_dir) = spawn_app();

    app.wait_for("WELCOME TO AZUL", Duration::from_secs(3));
    app.send("/");
    app.send("https://fixture.local/links");
    app.send_key(Key::Enter);
    app.wait_for("FIXTURE LINKS", Duration::from_secs(4));

    app.wait_for(" 10 ", Duration::from_secs(3));
    app.send("z");
    app.wait_for_absence(" 10 ", Duration::from_secs(3));
    app.send("z");
    app.wait_for(" 10 ", Duration::from_secs(3));

    app.send("c");
    app.wait_for(" chat ", Duration::from_secs(3));
    app.send_key(Key::Esc);
    app.wait_for_absence(" chat ", Duration::from_secs(3));

    app.send("q");
    app.wait_for_exit(Duration::from_secs(3));
}
