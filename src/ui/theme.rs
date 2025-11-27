//! Dynamic theme system for Azul browser
//!
//! Provides 7 built-in themes with the ability to cycle through them at runtime.

use ratatui::style::Color;

/// Available theme presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    AzulDark,    // Default - Tokyo Night based
    Monokai,     // Classic Monokai
    Gruvbox,     // Gruvbox Dark
    Nord,        // Nord theme
    Dracula,     // Dracula theme
    Catppuccin,  // Catppuccin Mocha
    Solarized,   // Solarized Dark
}

impl ThemePreset {
    pub fn name(&self) -> &'static str {
        match self {
            ThemePreset::AzulDark => "Azul Dark",
            ThemePreset::Monokai => "Monokai",
            ThemePreset::Gruvbox => "Gruvbox",
            ThemePreset::Nord => "Nord",
            ThemePreset::Dracula => "Dracula",
            ThemePreset::Catppuccin => "Catppuccin",
            ThemePreset::Solarized => "Solarized",
        }
    }

    pub fn all() -> &'static [ThemePreset] {
        &[
            ThemePreset::AzulDark,
            ThemePreset::Monokai,
            ThemePreset::Gruvbox,
            ThemePreset::Nord,
            ThemePreset::Dracula,
            ThemePreset::Catppuccin,
            ThemePreset::Solarized,
        ]
    }

    pub fn next(&self) -> ThemePreset {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        all[(idx + 1) % all.len()]
    }

    pub fn prev(&self) -> ThemePreset {
        let all = Self::all();
        let idx = all.iter().position(|t| t == self).unwrap_or(0);
        all[(idx + all.len() - 1) % all.len()]
    }
}

/// Theme color configuration
#[derive(Debug, Clone)]
pub struct ThemeColors {
    // Primary accent colors
    pub primary: Color,
    pub primary_bright: Color,
    pub primary_dim: Color,

    // Secondary colors
    pub secondary: Color,
    pub tertiary: Color,

    // Semantic colors
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub info: Color,

    // Text colors
    pub text: Color,
    pub text_dim: Color,
    pub text_muted: Color,

    // Background colors
    pub bg: Color,
    pub bg_secondary: Color,
    pub bg_highlight: Color,

    // Border colors
    pub border: Color,
    pub border_active: Color,
}

impl ThemeColors {
    /// Get colors for a theme preset
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::AzulDark => Self::azul_dark(),
            ThemePreset::Monokai => Self::monokai(),
            ThemePreset::Gruvbox => Self::gruvbox(),
            ThemePreset::Nord => Self::nord(),
            ThemePreset::Dracula => Self::dracula(),
            ThemePreset::Catppuccin => Self::catppuccin(),
            ThemePreset::Solarized => Self::solarized(),
        }
    }

    /// Azul Dark - Tokyo Night inspired (default)
    fn azul_dark() -> Self {
        Self {
            primary: Color::Rgb(0, 191, 255),      // Deep sky blue
            primary_bright: Color::Rgb(125, 207, 255),
            primary_dim: Color::Rgb(0, 140, 200),

            secondary: Color::Rgb(187, 154, 247),  // Purple
            tertiary: Color::Rgb(255, 169, 0),     // Orange

            success: Color::Rgb(158, 206, 106),    // Green
            warning: Color::Rgb(255, 169, 0),      // Orange
            error: Color::Rgb(247, 118, 142),      // Red
            info: Color::Rgb(122, 162, 247),       // Blue

            text: Color::Rgb(192, 202, 245),
            text_dim: Color::Rgb(165, 175, 215),
            text_muted: Color::Rgb(125, 137, 168),

            bg: Color::Rgb(26, 27, 38),
            bg_secondary: Color::Rgb(36, 40, 59),
            bg_highlight: Color::Rgb(41, 46, 66),

            border: Color::Rgb(122, 162, 247),
            border_active: Color::Rgb(0, 191, 255),
        }
    }

    /// Monokai theme
    fn monokai() -> Self {
        Self {
            primary: Color::Rgb(102, 217, 239),    // Cyan
            primary_bright: Color::Rgb(166, 226, 46),
            primary_dim: Color::Rgb(82, 190, 200),

            secondary: Color::Rgb(174, 129, 255),  // Purple
            tertiary: Color::Rgb(253, 151, 31),    // Orange

            success: Color::Rgb(166, 226, 46),     // Green
            warning: Color::Rgb(253, 151, 31),     // Orange
            error: Color::Rgb(249, 38, 114),       // Pink/Red
            info: Color::Rgb(102, 217, 239),       // Cyan

            text: Color::Rgb(248, 248, 242),
            text_dim: Color::Rgb(200, 200, 195),
            text_muted: Color::Rgb(117, 113, 94),

            bg: Color::Rgb(39, 40, 34),
            bg_secondary: Color::Rgb(49, 50, 44),
            bg_highlight: Color::Rgb(59, 60, 54),

            border: Color::Rgb(102, 217, 239),
            border_active: Color::Rgb(166, 226, 46),
        }
    }

    /// Gruvbox Dark theme
    fn gruvbox() -> Self {
        Self {
            primary: Color::Rgb(215, 153, 33),     // Yellow
            primary_bright: Color::Rgb(250, 189, 47),
            primary_dim: Color::Rgb(181, 118, 20),

            secondary: Color::Rgb(211, 134, 155),  // Purple
            tertiary: Color::Rgb(254, 128, 25),    // Orange

            success: Color::Rgb(152, 151, 26),     // Green
            warning: Color::Rgb(254, 128, 25),     // Orange
            error: Color::Rgb(204, 36, 29),        // Red
            info: Color::Rgb(69, 133, 136),        // Aqua

            text: Color::Rgb(235, 219, 178),
            text_dim: Color::Rgb(213, 196, 161),
            text_muted: Color::Rgb(168, 153, 132),

            bg: Color::Rgb(40, 40, 40),
            bg_secondary: Color::Rgb(50, 48, 47),
            bg_highlight: Color::Rgb(60, 56, 54),

            border: Color::Rgb(215, 153, 33),
            border_active: Color::Rgb(250, 189, 47),
        }
    }

    /// Nord theme
    fn nord() -> Self {
        Self {
            primary: Color::Rgb(136, 192, 208),    // Frost cyan
            primary_bright: Color::Rgb(143, 188, 187),
            primary_dim: Color::Rgb(94, 129, 172),

            secondary: Color::Rgb(180, 142, 173),  // Purple
            tertiary: Color::Rgb(208, 135, 112),   // Orange

            success: Color::Rgb(163, 190, 140),    // Green
            warning: Color::Rgb(235, 203, 139),    // Yellow
            error: Color::Rgb(191, 97, 106),       // Red
            info: Color::Rgb(129, 161, 193),       // Blue

            text: Color::Rgb(236, 239, 244),
            text_dim: Color::Rgb(216, 222, 233),
            text_muted: Color::Rgb(76, 86, 106),

            bg: Color::Rgb(46, 52, 64),
            bg_secondary: Color::Rgb(59, 66, 82),
            bg_highlight: Color::Rgb(67, 76, 94),

            border: Color::Rgb(136, 192, 208),
            border_active: Color::Rgb(143, 188, 187),
        }
    }

    /// Dracula theme
    fn dracula() -> Self {
        Self {
            primary: Color::Rgb(189, 147, 249),    // Purple
            primary_bright: Color::Rgb(255, 121, 198),
            primary_dim: Color::Rgb(139, 233, 253),

            secondary: Color::Rgb(255, 121, 198),  // Pink
            tertiary: Color::Rgb(255, 184, 108),   // Orange

            success: Color::Rgb(80, 250, 123),     // Green
            warning: Color::Rgb(241, 250, 140),    // Yellow
            error: Color::Rgb(255, 85, 85),        // Red
            info: Color::Rgb(139, 233, 253),       // Cyan

            text: Color::Rgb(248, 248, 242),
            text_dim: Color::Rgb(226, 226, 220),
            text_muted: Color::Rgb(98, 114, 164),

            bg: Color::Rgb(40, 42, 54),
            bg_secondary: Color::Rgb(68, 71, 90),
            bg_highlight: Color::Rgb(55, 58, 76),

            border: Color::Rgb(189, 147, 249),
            border_active: Color::Rgb(255, 121, 198),
        }
    }

    /// Catppuccin Mocha theme
    fn catppuccin() -> Self {
        Self {
            primary: Color::Rgb(137, 180, 250),    // Blue
            primary_bright: Color::Rgb(148, 226, 213),
            primary_dim: Color::Rgb(116, 199, 236),

            secondary: Color::Rgb(203, 166, 247),  // Mauve
            tertiary: Color::Rgb(250, 179, 135),   // Peach

            success: Color::Rgb(166, 227, 161),    // Green
            warning: Color::Rgb(249, 226, 175),    // Yellow
            error: Color::Rgb(243, 139, 168),      // Red
            info: Color::Rgb(137, 220, 235),       // Sky

            text: Color::Rgb(205, 214, 244),
            text_dim: Color::Rgb(186, 194, 222),
            text_muted: Color::Rgb(108, 112, 134),

            bg: Color::Rgb(30, 30, 46),
            bg_secondary: Color::Rgb(49, 50, 68),
            bg_highlight: Color::Rgb(69, 71, 90),

            border: Color::Rgb(137, 180, 250),
            border_active: Color::Rgb(203, 166, 247),
        }
    }

    /// Solarized Dark theme
    fn solarized() -> Self {
        Self {
            primary: Color::Rgb(38, 139, 210),     // Blue
            primary_bright: Color::Rgb(42, 161, 152),
            primary_dim: Color::Rgb(108, 113, 196),

            secondary: Color::Rgb(211, 54, 130),   // Magenta
            tertiary: Color::Rgb(203, 75, 22),     // Orange

            success: Color::Rgb(133, 153, 0),      // Green
            warning: Color::Rgb(181, 137, 0),      // Yellow
            error: Color::Rgb(220, 50, 47),        // Red
            info: Color::Rgb(42, 161, 152),        // Cyan

            text: Color::Rgb(131, 148, 150),
            text_dim: Color::Rgb(147, 161, 161),
            text_muted: Color::Rgb(88, 110, 117),

            bg: Color::Rgb(0, 43, 54),
            bg_secondary: Color::Rgb(7, 54, 66),
            bg_highlight: Color::Rgb(0, 57, 71),

            border: Color::Rgb(38, 139, 210),
            border_active: Color::Rgb(42, 161, 152),
        }
    }
}

/// Theme manager for runtime theme switching
pub struct ThemeManager {
    current_preset: ThemePreset,
    colors: ThemeColors,
}

impl ThemeManager {
    pub fn new() -> Self {
        let preset = ThemePreset::AzulDark;
        Self {
            current_preset: preset,
            colors: ThemeColors::from_preset(preset),
        }
    }

    pub fn with_preset(preset: ThemePreset) -> Self {
        Self {
            current_preset: preset,
            colors: ThemeColors::from_preset(preset),
        }
    }

    /// Get current theme preset
    pub fn preset(&self) -> ThemePreset {
        self.current_preset
    }

    /// Get current theme colors
    pub fn colors(&self) -> &ThemeColors {
        &self.colors
    }

    /// Cycle to next theme, returns the new theme name
    pub fn cycle_next(&mut self) -> &'static str {
        self.current_preset = self.current_preset.next();
        self.colors = ThemeColors::from_preset(self.current_preset);
        self.current_preset.name()
    }

    /// Cycle to previous theme, returns the new theme name
    pub fn cycle_prev(&mut self) -> &'static str {
        self.current_preset = self.current_preset.prev();
        self.colors = ThemeColors::from_preset(self.current_preset);
        self.current_preset.name()
    }

    /// Set a specific theme
    pub fn set_theme(&mut self, preset: ThemePreset) {
        self.current_preset = preset;
        self.colors = ThemeColors::from_preset(preset);
    }
}

impl Default for ThemeManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_cycling() {
        let mut tm = ThemeManager::new();
        assert_eq!(tm.preset(), ThemePreset::AzulDark);

        tm.cycle_next();
        assert_eq!(tm.preset(), ThemePreset::Monokai);

        tm.cycle_prev();
        assert_eq!(tm.preset(), ThemePreset::AzulDark);
    }

    #[test]
    fn test_all_themes_have_colors() {
        for preset in ThemePreset::all() {
            let colors = ThemeColors::from_preset(*preset);
            // Just ensure we can access all color fields
            let _ = colors.primary;
            let _ = colors.text;
            let _ = colors.bg;
        }
    }
}
