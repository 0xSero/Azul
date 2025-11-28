//! Azul Companion - Ultra-compact animated mascot
//!
//! A tiny, dense companion that follows you around the UI.
//! Inspired by Claude's thinking indicator - minimal but expressive.

/// Companion state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionState {
    Idle,
    Loading,
    Searching,
    Success,
    Error,
    Thinking,
}

/// Walking direction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkDirection {
    Left,
    Right,
    Idle,
}

/// The Azul companion - follows you everywhere
pub struct Mascot {
    state: CompanionState,
    frame: usize,
    // Walking mascot properties
    pub position: i16,       // X position on screen
    pub direction: WalkDirection,
    pub screen_width: u16,   // Track screen width for bounds
    walk_frame: usize,
    idle_counter: usize,     // How long we've been idle
}

impl Mascot {
    pub fn new() -> Self {
        Self {
            state: CompanionState::Idle,
            frame: 0,
            position: 10,
            direction: WalkDirection::Right,
            screen_width: 80,
            walk_frame: 0,
            idle_counter: 0,
        }
    }

    pub fn set_state(&mut self, state: CompanionState) {
        if self.state != state {
            self.state = state;
            self.frame = 0;
        }
    }

    pub fn state(&self) -> CompanionState {
        self.state
    }

    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
        self.walk_frame = self.walk_frame.wrapping_add(1);

        // Update walking position every few frames
        if self.walk_frame % 3 == 0 {
            match self.direction {
                WalkDirection::Right => {
                    self.position += 1;
                    // Turn around at edge (leave room for mascot width ~6 chars)
                    if self.position >= (self.screen_width as i16 - 8) {
                        self.direction = WalkDirection::Left;
                        self.idle_counter = 0;
                    }
                }
                WalkDirection::Left => {
                    self.position -= 1;
                    // Turn around at left edge
                    if self.position <= 2 {
                        self.direction = WalkDirection::Right;
                        self.idle_counter = 0;
                    }
                }
                WalkDirection::Idle => {
                    self.idle_counter += 1;
                    // Start walking again after being idle
                    if self.idle_counter > 20 {
                        self.direction = if self.position < (self.screen_width as i16 / 2) {
                            WalkDirection::Right
                        } else {
                            WalkDirection::Left
                        };
                        self.idle_counter = 0;
                    }
                }
            }
        }

        // Occasionally pause to look around
        if self.walk_frame % 50 == 0 && self.direction != WalkDirection::Idle {
            if rand_simple() % 4 == 0 {
                self.direction = WalkDirection::Idle;
            }
        }
    }

    /// Set screen width for boundary checking
    pub fn set_screen_width(&mut self, width: u16) {
        self.screen_width = width;
        // Clamp position if screen got smaller
        if self.position >= (width as i16 - 8) {
            self.position = (width as i16 - 8).max(2);
        }
    }

    /// Get the walking mascot sprite - cute little creature
    pub fn walking_sprite(&self) -> &'static str {
        match self.state {
            CompanionState::Loading | CompanionState::Searching => {
                // Running/busy - excited!
                let frames = ["ᕕ(°▽°)ᕗ", "ᕕ(°◡°)ᕗ", "ᕕ(°▽°)ᕗ", "ᕕ(°ᴗ°)ᕗ"];
                frames[self.walk_frame % frames.len()]
            }
            CompanionState::Success => {
                "\\(◕‿◕)/"  // celebrating!
            }
            CompanionState::Error => {
                "(◕︵◕)"  // sad
            }
            CompanionState::Thinking => {
                let frames = ["(◕.◕)", "(◕..)", "(..◕)", "(◕.◕)"];
                frames[self.walk_frame % frames.len()]
            }
            _ => {
                // Cute idle/walking
                match self.direction {
                    WalkDirection::Right => {
                        let frames = ["(◕ᴗ◕)>", "(◕‿◕)ᐳ", "(◕ᴗ◕)›", "(◕‿◕)>"];
                        frames[self.walk_frame % frames.len()]
                    }
                    WalkDirection::Left => {
                        let frames = ["<(◕ᴗ◕)", "ᐸ(◕‿◕)", "‹(◕ᴗ◕)", "<(◕‿◕)"];
                        frames[self.walk_frame % frames.len()]
                    }
                    WalkDirection::Idle => {
                        let frames = ["(◕‿◕)", "(◕ᴗ◕)", "(◕‿◕)", "(◕◡◕)"];
                        frames[self.walk_frame % frames.len()]
                    }
                }
            }
        }
    }

    /// Get position as usize for rendering
    pub fn x_position(&self) -> u16 {
        self.position.max(0) as u16
    }

    /// The main companion - ultra compact, 1-2 chars
    pub fn view(&self) -> &'static str {
        let frames = match self.state {
            CompanionState::Idle => &COMPANION_IDLE[..],
            CompanionState::Loading => &COMPANION_LOADING[..],
            CompanionState::Searching => &COMPANION_SEARCHING[..],
            CompanionState::Success => &COMPANION_SUCCESS[..],
            CompanionState::Error => &COMPANION_ERROR[..],
            CompanionState::Thinking => &COMPANION_THINKING[..],
        };
        frames[self.frame % frames.len()]
    }

    /// Companion with context label
    pub fn view_with_label(&self) -> &'static str {
        let frames = match self.state {
            CompanionState::Idle => &LABELED_IDLE[..],
            CompanionState::Loading => &LABELED_LOADING[..],
            CompanionState::Searching => &LABELED_SEARCHING[..],
            CompanionState::Success => &LABELED_SUCCESS[..],
            CompanionState::Error => &LABELED_ERROR[..],
            CompanionState::Thinking => &LABELED_THINKING[..],
        };
        frames[self.frame % frames.len()]
    }

    /// Status bar companion - shows activity
    pub fn view_status(&self) -> &'static str {
        let frames = match self.state {
            CompanionState::Idle => &STATUS_IDLE[..],
            CompanionState::Loading => &STATUS_LOADING[..],
            CompanionState::Searching => &STATUS_SEARCHING[..],
            CompanionState::Success => &STATUS_SUCCESS[..],
            CompanionState::Error => &STATUS_ERROR[..],
            CompanionState::Thinking => &STATUS_THINKING[..],
        };
        frames[self.frame % frames.len()]
    }

    /// For backwards compatibility
    pub fn view_mini(&self) -> &'static str {
        self.view()
    }

    pub fn view_compact(&self) -> &'static str {
        self.view_with_label()
    }
}

impl Default for Mascot {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export the old state names for compatibility
pub use CompanionState as MascotState;

// ═══════════════════════════════════════════════════════════════════════════
// COMPANION CORE - Ultra minimal (1-2 chars)
// ═══════════════════════════════════════════════════════════════════════════

// Idle: gentle pulse
static COMPANION_IDLE: [&str; 6] = ["◉", "◎", "○", "◎", "◉", "●"];

// Loading: smooth Braille spinner
static COMPANION_LOADING: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// Searching: scanning dots
static COMPANION_SEARCHING: [&str; 6] = ["⣾", "⣽", "⣻", "⢿", "⡿", "⣟"];

// Success: sparkle
static COMPANION_SUCCESS: [&str; 4] = ["✦", "★", "✧", "☆"];

// Error: pulse warning
static COMPANION_ERROR: [&str; 4] = ["◈", "◇", "◈", "!"];

// Thinking: contemplation
static COMPANION_THINKING: [&str; 8] = ["⠁", "⠂", "⠄", "⡀", "⢀", "⠠", "⠐", "⠈"];

// ═══════════════════════════════════════════════════════════════════════════
// LABELED COMPANION - With state context
// ═══════════════════════════════════════════════════════════════════════════

static LABELED_IDLE: [&str; 6] = [
    "◉ ready",
    "◎ ready",
    "○ ready",
    "◎ ready",
    "◉ ready",
    "● ready",
];

static LABELED_LOADING: [&str; 10] = [
    "⠋ loading",
    "⠙ loading",
    "⠹ loading",
    "⠸ loading",
    "⠼ loading",
    "⠴ loading",
    "⠦ loading",
    "⠧ loading",
    "⠇ loading",
    "⠏ loading",
];

static LABELED_SEARCHING: [&str; 6] = [
    "⣾ searching",
    "⣽ searching",
    "⣻ searching",
    "⢿ searching",
    "⡿ searching",
    "⣟ searching",
];

static LABELED_SUCCESS: [&str; 4] = ["✦ done", "★ done", "✧ done", "☆ done"];

static LABELED_ERROR: [&str; 4] = ["◈ error", "◇ error", "◈ oops", "! error"];

static LABELED_THINKING: [&str; 8] = [
    "⠁ thinking",
    "⠂ thinking",
    "⠄ thinking",
    "⡀ thinking",
    "⢀ thinking",
    "⠠ thinking",
    "⠐ thinking",
    "⠈ thinking",
];

// ═══════════════════════════════════════════════════════════════════════════
// STATUS BAR COMPANION - Activity indicator
// ═══════════════════════════════════════════════════════════════════════════

static STATUS_IDLE: [&str; 4] = ["[◉]", "[◎]", "[○]", "[◎]"];

static STATUS_LOADING: [&str; 8] = [
    "[▰▱▱]",
    "[▰▰▱]",
    "[▰▰▰]",
    "[▱▰▰]",
    "[▱▱▰]",
    "[▱▱▱]",
    "[▱▰▱]",
    "[▰▱▰]",
];

static STATUS_SEARCHING: [&str; 6] = [
    "[◐]", "[◓]", "[◑]", "[◒]", "[◐]", "[◓]",
];

static STATUS_SUCCESS: [&str; 3] = ["[✓]", "[★]", "[✓]"];

static STATUS_ERROR: [&str; 3] = ["[✗]", "[!]", "[✗]"];

static STATUS_THINKING: [&str; 4] = ["[·]", "[··]", "[···]", "[··]"];

// ═══════════════════════════════════════════════════════════════════════════
// SPLASH - Minimal splash for empty state
// ═══════════════════════════════════════════════════════════════════════════

pub fn mini_splash() -> &'static str {
    r#"
          ◉

    A Z U L

    / to search
    ? for help
"#
}

pub fn splash_art() -> &'static str {
    mini_splash()
}

pub fn bye_art() -> &'static str {
    "◉ bye!"
}

/// Simple pseudo-random for mascot behavior (no external deps)
fn rand_simple() -> usize {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    nanos as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_companion_states() {
        let mut mascot = Mascot::new();
        assert_eq!(mascot.state(), CompanionState::Idle);

        mascot.set_state(CompanionState::Loading);
        assert_eq!(mascot.state(), CompanionState::Loading);
    }

    #[test]
    fn test_companion_animation() {
        let mut mascot = Mascot::new();
        mascot.set_state(CompanionState::Loading);

        let first = mascot.view();
        mascot.tick();
        let second = mascot.view();

        // Should cycle through frames
        assert!(!first.is_empty());
        assert!(!second.is_empty());
    }

    #[test]
    fn test_all_views() {
        let mascot = Mascot::new();
        assert!(!mascot.view().is_empty());
        assert!(!mascot.view_with_label().is_empty());
        assert!(!mascot.view_status().is_empty());
    }

    #[test]
    fn test_walking() {
        let mut mascot = Mascot::new();
        mascot.set_screen_width(80);
        let start_pos = mascot.position;
        for _ in 0..10 {
            mascot.tick();
        }
        // Should have moved
        assert_ne!(mascot.position, start_pos);
    }
}
