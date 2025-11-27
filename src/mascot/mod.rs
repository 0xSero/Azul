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

/// The Azul companion - follows you everywhere
pub struct Mascot {
    state: CompanionState,
    frame: usize,
}

impl Mascot {
    pub fn new() -> Self {
        Self {
            state: CompanionState::Idle,
            frame: 0,
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
}
