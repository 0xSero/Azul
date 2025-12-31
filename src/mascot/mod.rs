//! Azul Status Indicator - Minimal, clean
//!
//! Simple ASCII-based status indicator. No emoji, no unicode symbols.

/// Status state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompanionState {
    Idle,
    Loading,
    Searching,
    Success,
    Error,
    Thinking,
}

/// Minimal status indicator - ASCII only
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

    /// Clean ASCII status indicator
    pub fn view(&self) -> &'static str {
        match self.state {
            CompanionState::Idle => "-",
            CompanionState::Loading => {
                let frames = ["-", "\\", "|", "/"];
                frames[self.frame % frames.len()]
            }
            CompanionState::Searching => {
                let frames = ["-", "\\", "|", "/"];
                frames[self.frame % frames.len()]
            }
            CompanionState::Success => "+",
            CompanionState::Error => "!",
            CompanionState::Thinking => {
                let frames = [".", "..", "..."];
                frames[self.frame % frames.len()]
            }
        }
    }

    /// Status bar version - same indicator, just formatted for status
    pub fn view_status(&self) -> &'static str {
        self.view()
    }

    /// For backwards compatibility - all point to the same view
    pub fn view_mini(&self) -> &'static str {
        self.view()
    }

    pub fn view_compact(&self) -> &'static str {
        self.view()
    }

    pub fn view_with_label(&self) -> &'static str {
        self.view()
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
// SPLASH - Minimal splash for empty state
// ═══════════════════════════════════════════════════════════════════════════

pub fn mini_splash() -> &'static str {
    r#"

    a z u l

    / search
    ? help
"#
}

pub fn splash_art() -> &'static str {
    mini_splash()
}

pub fn bye_art() -> &'static str {
    "bye"
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
