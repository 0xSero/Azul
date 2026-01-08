#![allow(dead_code)]

use std::path::PathBuf;

pub mod pty;

pub fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[cfg(feature = "test-support")]
use std::fs;

#[cfg(feature = "test-support")]
use azul_browse::app::App;
#[cfg(feature = "test-support")]
use azul_browse::browser::Page;
#[cfg(feature = "test-support")]
use azul_browse::ui;
#[cfg(feature = "test-support")]
use ratatui::backend::TestBackend;
#[cfg(feature = "test-support")]
use ratatui::Terminal;

#[cfg(feature = "test-support")]
pub fn load_page(name: &str) -> Page {
    let path = fixture_root().join("pages").join(format!("{}.json", name));
    let data = fs::read(&path).unwrap_or_else(|err| {
        panic!("failed to read fixture {}: {}", path.display(), err);
    });
    serde_json::from_slice(&data).unwrap_or_else(|err| {
        panic!("failed to parse fixture {}: {}", path.display(), err);
    })
}

#[cfg(feature = "test-support")]
pub fn test_app() -> App {
    let mut app = App::new_test();
    app.animation_tick = 0;
    app.focus_pulse_phase = 0;
    app
}

#[cfg(feature = "test-support")]
pub fn set_page(app: &mut App, page: Page) {
    if let Some(tab) = app.tabs.active_tab_mut() {
        tab.set_page(page);
    }
}

#[cfg(feature = "test-support")]
pub fn render_app(app: &mut App, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("failed to create test terminal");
    terminal
        .draw(|frame| ui::render(frame, app))
        .expect("failed to render UI");
    buffer_to_string(terminal.backend().buffer())
}

#[cfg(feature = "test-support")]
fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let mut out = String::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            line.push_str(buffer.cell((x, y)).expect("missing cell").symbol());
        }
        while line.ends_with(' ') {
            line.pop();
        }
        out.push_str(&line);
        if y + 1 < buffer.area.height {
            out.push('\n');
        }
    }
    out
}
