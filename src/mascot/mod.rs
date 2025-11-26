//! Azul mascot - the cute blue octopus with animations

/// Mascot state/mood
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MascotState {
    Idle,
    Loading,
    Searching,
    Success,
    Error,
    Sleeping,
    Waving,
}

/// The Azul mascot
pub struct Mascot {
    state: MascotState,
    frame: usize,
}

impl Mascot {
    /// Create a new mascot
    pub fn new() -> Self {
        Self {
            state: MascotState::Idle,
            frame: 0,
        }
    }

    /// Set the mascot state
    pub fn set_state(&mut self, state: MascotState) {
        if self.state != state {
            self.state = state;
            self.frame = 0;
        }
    }

    /// Get current state
    pub fn state(&self) -> MascotState {
        self.state
    }

    /// Advance animation frame
    pub fn tick(&mut self) {
        self.frame = self.frame.wrapping_add(1);
    }

    /// Get the current animation frame
    pub fn frame(&self) -> usize {
        self.frame
    }

    /// Render the full mascot
    pub fn view(&self) -> &'static str {
        let frames = self.get_frames();
        let idx = self.frame % frames.len();
        frames[idx]
    }

    /// Render compact version for status bar
    pub fn view_compact(&self) -> &'static str {
        let frames = self.get_compact_frames();
        let idx = self.frame % frames.len();
        frames[idx]
    }

    /// Render mini inline version
    pub fn view_mini(&self) -> &'static str {
        let frames = self.get_mini_frames();
        let idx = self.frame % frames.len();
        frames[idx]
    }

    fn get_frames(&self) -> &'static [&'static str] {
        match self.state {
            MascotState::Idle => &IDLE_FRAMES,
            MascotState::Loading => &LOADING_FRAMES,
            MascotState::Searching => &SEARCHING_FRAMES,
            MascotState::Success => &SUCCESS_FRAMES,
            MascotState::Error => &ERROR_FRAMES,
            MascotState::Sleeping => &SLEEPING_FRAMES,
            MascotState::Waving => &WAVING_FRAMES,
        }
    }

    fn get_compact_frames(&self) -> &'static [&'static str] {
        match self.state {
            MascotState::Idle => &COMPACT_IDLE,
            MascotState::Loading => &COMPACT_LOADING,
            MascotState::Searching => &COMPACT_SEARCHING,
            MascotState::Success => &COMPACT_SUCCESS,
            MascotState::Error => &COMPACT_ERROR,
            _ => &COMPACT_IDLE,
        }
    }

    fn get_mini_frames(&self) -> &'static [&'static str] {
        match self.state {
            MascotState::Idle => &["(o^o)", "(o`o)", "(o^o)", "(`^`)"],
            MascotState::Loading => &["(o.o)", "(o..)", "(..o)", "(o.o)"],
            MascotState::Searching => &["(o_o)", "(o_`)", "(`_o)", "(`_`)"],
            MascotState::Success => &["(^o^)", "(^v^)", "(^o^)", "\\(^o^)/"],
            MascotState::Error => &["(;_;)", "(T_T)", "(;_;)", "(>_<)"],
            _ => &["(o^o)"],
        }
    }
}

impl Default for Mascot {
    fn default() -> Self {
        Self::new()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// FULL SIZE MASCOT FRAMES
// ═══════════════════════════════════════════════════════════════════════════

static IDLE_FRAMES: [&str; 4] = [
    r#"
    .-------.
    | O   O |
    |   w   |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
    ~ AZUL ~
"#,
    r#"
    .-------.
    | O   O |
    |   w   |
    '---+---'
   ///|||||\\\
   / /|||||\ \
    ~ AZUL ~
"#,
    r#"
    .-------.
    | O   O |
    |   u   |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
    ~ AZUL ~
"#,
    r#"
    .-------.
    | O   O |
    |   w   |
    '---+---'
   ///|||||\\\
   / /|||||\ \
    ~ AZUL ~
"#,
];

static LOADING_FRAMES: [&str; 4] = [
    r#"
    .-------.
    | O   O |  |
    |   o   |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
"#,
    r#"
    .-------.
    | O   O |  /
    |   o   |
    '---+---'
   ///|||||\\\
   / /|||||\ \
"#,
    r#"
    .-------.
    | O   O |  -
    |   o   |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
"#,
    r#"
    .-------.
    | O   O |  \
    |   o   |
    '---+---'
   ///|||||\\\
   / /|||||\ \
"#,
];

static SEARCHING_FRAMES: [&str; 4] = [
    r#"
    .-------.
    | O > O | (?)
    |   ?   |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
"#,
    r#"
    .-------.
    | O   > |  (?)
    |   ?   |
    '---+---'
   ///|||||\\\
   / /|||||\ \
"#,
    r#"
    .-------.
    | < O O |(?)
    |   ?   |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
"#,
    r#"
    .-------.
    | <   O | (?)
    |   ?   |
    '---+---'
   ///|||||\\\
   / /|||||\ \
"#,
];

static SUCCESS_FRAMES: [&str; 4] = [
    r#"
    .-------.
    | ^   ^ | *
    |   V   |
    '---+---'
   \\/\|||/\/
    \ ||| /
     YAY!
"#,
    r#"
   *.-------.*
    | ^   ^ |
    |   V   |
    '---+---'
   /\/\|||\/\
    / ||| \
     YAY!
"#,
    r#"
    .-------.
    | ^   ^ | *
    |   u   |
    '---+---'
   \\/\|||/\/
    \ ||| /
     YAY!
"#,
    r#"
  * .-------. *
    | ^   ^ |
    |   V   |
    '---+---'
   /\/\|||\/\
    / ||| \
     YAY!
"#,
];

static ERROR_FRAMES: [&str; 3] = [
    r#"
    .-------.
    | ;   ; | X
    |   n   |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
     oops
"#,
    r#"
    .-------.
    | T   T | X
    |  ~~~  |
    '---+---'
   ///|||||\\\
   / /|||||\ \
     oops
"#,
    r#"
    .-------.
    | ;   ; | X
    |  ~~~  |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
     oops
"#,
];

static SLEEPING_FRAMES: [&str; 4] = [
    r#"
    .-------.
    | -   - |  z
    |   w   | z
    '---+---'  Z
   ///|||||\\\
  / / ||||| \ \
"#,
    r#"
    .-------.
    | -   - |   z
    |   w   |  z
    '---+---' Z
   ///|||||\\\
  / / ||||| \ \
"#,
    r#"
    .-------.
    | -   - |    z
    |   w   |   z
    '---+---'  Z
   ///|||||\\\
  / / ||||| \ \
"#,
    r#"
    .-------.
    | -   - |
    |   w   |  z
    '---+---' z
   ///|||||\\\ Z
  / / ||||| \ \
"#,
];

static WAVING_FRAMES: [&str; 4] = [
    r#"
    .-------.
    | O   O | /
    |   V   |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
      hi!
"#,
    r#"
    .-------.
    | O   O |  |
    |   V   |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
      hi!
"#,
    r#"
    .-------.
    | O   O | \
    |   V   |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
      hi!
"#,
    r#"
    .-------.
    | O   O |  |
    |   V   |
    '---+---'
   ///|||||\\\
  / / ||||| \ \
      hi!
"#,
];

// ═══════════════════════════════════════════════════════════════════════════
// COMPACT FRAMES - For smaller spaces
// ═══════════════════════════════════════════════════════════════════════════

static COMPACT_IDLE: [&str; 2] = [
    r#".-----.
|O w O|
'-+-+-'"#,
    r#".-----.
|O u O|
'-+-+-'"#,
];

static COMPACT_LOADING: [&str; 4] = [
    r#".-----.|
|O o O|
'-+-+-'"#,
    r#".-----./
|O o O|
'-+-+-'"#,
    r#".-----.-
|O o O|
'-+-+-'"#,
    r#".-----.\
|O o O|
'-+-+-'"#,
];

static COMPACT_SEARCHING: [&str; 2] = [
    r#".-----.(?)
|O>? O|
'-+-+-'"#,
    r#".-----. (?)
|O ?<O|
'-+-+-'"#,
];

static COMPACT_SUCCESS: [&str; 2] = [
    r#".-----.*
|^ V ^|
'-\-/-'"#,
    r#".-----.*
|^ u ^|*
'-/-\-'"#,
];

static COMPACT_ERROR: [&str; 2] = [
    r#".-----.X
|; n ;|
'-+-+-'"#,
    r#".-----.X
|T~~ T|
'-+-+-'"#,
];

// ═══════════════════════════════════════════════════════════════════════════
// SPLASH SCREEN
// ═══════════════════════════════════════════════════════════════════════════

/// Get the full splash screen art for startup
pub fn splash_art() -> &'static str {
    r#"
         .-----------------.
         |                 |
         |    O       O    |
         |                 |
         |       V         |
         |                 |
         '--------+--------'
              ///|||\\\
             / / ||| \ \
            /  / ||| \  \
           /  /  |||  \  \
          /  /   |||   \  \

     ___   ____  _   _  _
    / _ \ |_  / | | | || |
   | |_| | / /  | |_| || |__
   |_| |_|/___|  \___/ |____|

        Terminal Web Browser
         ~ Surf the web! ~
"#
}

/// Get compact splash for smaller terminals
pub fn mini_splash() -> &'static str {
    r#"
    .-------.
    | O   O |
    |   V   |   AZUL
    '---+---'   Browser
   ///|||||\\\
  / / ||||| \ \
"#
}

/// Get goodbye art
pub fn bye_art() -> &'static str {
    r#"
    .-------.
    | ^   ^ |
    |   u   |  Bye-bye!
    '---+---'
   \ \ ||||| / /   See you
    \\\|||||///     soon~
"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mascot_states() {
        let mut mascot = Mascot::new();
        assert_eq!(mascot.state(), MascotState::Idle);

        mascot.set_state(MascotState::Loading);
        assert_eq!(mascot.state(), MascotState::Loading);

        // Should reset frame on state change
        mascot.tick();
        mascot.tick();
        mascot.set_state(MascotState::Success);
        assert_eq!(mascot.frame(), 0);
    }

    #[test]
    fn test_mascot_animation() {
        let mut mascot = Mascot::new();
        let frame1 = mascot.view();

        mascot.tick();
        mascot.tick();

        // Different frame after ticks
        let _frame2 = mascot.view();
    }
}
