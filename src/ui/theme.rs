//! Warm Paper Design System for Azul Browser
//!
//! A papery, warm aesthetic with HSL colors anchored in the 30-40° hue range.
//! Minimal, tactile, like quality stationery—no harsh contrasts, everything warm and considered.

use ratatui::style::Color;

/// Available theme presets
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePreset {
    WarmPaperDark,  // Default - Rich charcoal with warm tones
    WarmPaperLight, // Cream/paper aesthetic
    AzulClassic,    // Original Tokyo Night based
    Monokai,
    Gruvbox,
    Nord,
    Dracula,
    Catppuccin,
}

impl ThemePreset {
    pub fn name(&self) -> &'static str {
        match self {
            ThemePreset::WarmPaperDark => "Warm Paper Dark",
            ThemePreset::WarmPaperLight => "Warm Paper Light",
            ThemePreset::AzulClassic => "Azul Classic",
            ThemePreset::Monokai => "Monokai",
            ThemePreset::Gruvbox => "Gruvbox",
            ThemePreset::Nord => "Nord",
            ThemePreset::Dracula => "Dracula",
            ThemePreset::Catppuccin => "Catppuccin",
        }
    }

    pub fn all() -> &'static [ThemePreset] {
        &[
            ThemePreset::WarmPaperDark,
            ThemePreset::WarmPaperLight,
            ThemePreset::AzulClassic,
            ThemePreset::Monokai,
            ThemePreset::Gruvbox,
            ThemePreset::Nord,
            ThemePreset::Dracula,
            ThemePreset::Catppuccin,
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
    // Primary accent colors (warm tans in Warm Paper)
    pub primary: Color,
    pub primary_bright: Color,
    pub primary_dim: Color,

    // Secondary colors
    pub secondary: Color,
    pub tertiary: Color,

    // Semantic colors (muted in Warm Paper)
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
    pub bg_secondary: Color, // Cards, elevated surfaces
    pub bg_highlight: Color, // Hover states

    // Border colors
    pub border: Color,
    pub border_active: Color,

    // Focus ring color
    pub focus_ring: Color,
}

impl ThemeColors {
    /// Get colors for a theme preset
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::WarmPaperDark => Self::warm_paper_dark(),
            ThemePreset::WarmPaperLight => Self::warm_paper_light(),
            ThemePreset::AzulClassic => Self::azul_classic(),
            ThemePreset::Monokai => Self::monokai(),
            ThemePreset::Gruvbox => Self::gruvbox(),
            ThemePreset::Nord => Self::nord(),
            ThemePreset::Dracula => Self::dracula(),
            ThemePreset::Catppuccin => Self::catppuccin(),
        }
    }

    /// Warm Paper Dark - Rich charcoal base with warm tones
    /// Base: hsl(30, 5%, 10.5%) = #1b1b1b
    /// Elevated: hsl(30, 5%, 12%) = #1e1e1e
    /// Text: hsl(40, 20%, 92%) = warm off-white
    fn warm_paper_dark() -> Self {
        Self {
            // Primary accent - warm tan/brown tones hsl(35, 25%, 50%)
            primary: Color::Rgb(159, 138, 96), // Warm tan accent
            primary_bright: Color::Rgb(194, 169, 118), // Lighter tan
            primary_dim: Color::Rgb(128, 111, 77), // Darker tan

            // Secondary - slightly cooler warm tone
            secondary: Color::Rgb(168, 142, 122), // Warm taupe
            tertiary: Color::Rgb(179, 161, 116),  // Soft gold

            // Semantic - muted, warm versions
            success: Color::Rgb(134, 156, 118), // Muted sage green
            warning: Color::Rgb(199, 163, 104), // Warm amber
            error: Color::Rgb(186, 120, 110),   // Muted terracotta
            info: Color::Rgb(138, 157, 168),    // Warm slate blue

            // Text - warm off-white hsl(40, 20%, 92%)
            text: Color::Rgb(240, 236, 224),       // Warm cream white
            text_dim: Color::Rgb(204, 198, 183),   // Dimmed cream
            text_muted: Color::Rgb(145, 140, 130), // Muted warm gray

            // Background - rich charcoal hsl(30, 5%, 10.5%)
            bg: Color::Rgb(27, 27, 27),           // #1b1b1b
            bg_secondary: Color::Rgb(30, 30, 30), // #1e1e1e - cards
            bg_highlight: Color::Rgb(42, 40, 38), // Hover highlight

            // Borders - subtle warm gray
            border: Color::Rgb(51, 49, 47),        // hsl(30, 5%, 20%)
            border_active: Color::Rgb(96, 88, 77), // Warmer active border

            // Focus ring - warm brown
            focus_ring: Color::Rgb(143, 124, 97), // hsl(35, 15%, 70%)
        }
    }

    /// Warm Paper Light - Cream/paper aesthetic
    /// Background: hsl(40, 30%, 97%) = warm cream
    /// Cards: hsl(40, 25%, 95%) = soft tan
    /// Text: hsl(30, 15%, 15%) = deep brown-charcoal
    fn warm_paper_light() -> Self {
        Self {
            // Primary accent - deeper warm tones for contrast
            primary: Color::Rgb(128, 107, 77), // Deep warm tan
            primary_bright: Color::Rgb(156, 132, 96), // Medium tan
            primary_dim: Color::Rgb(102, 85, 61), // Dark brown

            // Secondary tones
            secondary: Color::Rgb(138, 112, 92), // Warm brown
            tertiary: Color::Rgb(143, 128, 89),  // Olive tan

            // Semantic - slightly more vibrant for light mode visibility
            success: Color::Rgb(92, 128, 82),  // Olive green
            warning: Color::Rgb(176, 137, 76), // Warm amber
            error: Color::Rgb(166, 89, 78),    // Terracotta
            info: Color::Rgb(98, 122, 138),    // Warm slate

            // Text - deep brown-charcoal hsl(30, 15%, 15%)
            text: Color::Rgb(44, 40, 33),          // Deep brown
            text_dim: Color::Rgb(86, 78, 67),      // Medium brown
            text_muted: Color::Rgb(140, 131, 115), // Light brown

            // Background - warm cream hsl(40, 30%, 97%)
            bg: Color::Rgb(251, 249, 244),           // Cream paper
            bg_secondary: Color::Rgb(247, 244, 237), // Soft tan cards
            bg_highlight: Color::Rgb(240, 236, 227), // Hover cream

            // Borders - subtle muted hsl(35, 15%, 85%)
            border: Color::Rgb(223, 218, 207),        // Soft border
            border_active: Color::Rgb(194, 182, 158), // Active border

            // Focus ring - warm brown hsl(30, 20%, 25%)
            focus_ring: Color::Rgb(77, 66, 51), // Dark brown ring
        }
    }

    /// Azul Classic - Original Tokyo Night inspired
    fn azul_classic() -> Self {
        Self {
            primary: Color::Rgb(0, 191, 255),
            primary_bright: Color::Rgb(125, 207, 255),
            primary_dim: Color::Rgb(0, 140, 200),

            secondary: Color::Rgb(187, 154, 247),
            tertiary: Color::Rgb(255, 169, 0),

            success: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(255, 169, 0),
            error: Color::Rgb(247, 118, 142),
            info: Color::Rgb(122, 162, 247),

            text: Color::Rgb(192, 202, 245),
            text_dim: Color::Rgb(165, 175, 215),
            text_muted: Color::Rgb(125, 137, 168),

            bg: Color::Rgb(26, 27, 38),
            bg_secondary: Color::Rgb(36, 40, 59),
            bg_highlight: Color::Rgb(41, 46, 66),

            border: Color::Rgb(122, 162, 247),
            border_active: Color::Rgb(0, 191, 255),

            focus_ring: Color::Rgb(122, 162, 247),
        }
    }

    /// Monokai theme
    fn monokai() -> Self {
        Self {
            primary: Color::Rgb(102, 217, 239),
            primary_bright: Color::Rgb(166, 226, 46),
            primary_dim: Color::Rgb(82, 190, 200),

            secondary: Color::Rgb(174, 129, 255),
            tertiary: Color::Rgb(253, 151, 31),

            success: Color::Rgb(166, 226, 46),
            warning: Color::Rgb(253, 151, 31),
            error: Color::Rgb(249, 38, 114),
            info: Color::Rgb(102, 217, 239),

            text: Color::Rgb(248, 248, 242),
            text_dim: Color::Rgb(200, 200, 195),
            text_muted: Color::Rgb(117, 113, 94),

            bg: Color::Rgb(39, 40, 34),
            bg_secondary: Color::Rgb(49, 50, 44),
            bg_highlight: Color::Rgb(59, 60, 54),

            border: Color::Rgb(102, 217, 239),
            border_active: Color::Rgb(166, 226, 46),

            focus_ring: Color::Rgb(102, 217, 239),
        }
    }

    /// Gruvbox Dark theme
    fn gruvbox() -> Self {
        Self {
            primary: Color::Rgb(215, 153, 33),
            primary_bright: Color::Rgb(250, 189, 47),
            primary_dim: Color::Rgb(181, 118, 20),

            secondary: Color::Rgb(211, 134, 155),
            tertiary: Color::Rgb(254, 128, 25),

            success: Color::Rgb(152, 151, 26),
            warning: Color::Rgb(254, 128, 25),
            error: Color::Rgb(204, 36, 29),
            info: Color::Rgb(69, 133, 136),

            text: Color::Rgb(235, 219, 178),
            text_dim: Color::Rgb(213, 196, 161),
            text_muted: Color::Rgb(168, 153, 132),

            bg: Color::Rgb(40, 40, 40),
            bg_secondary: Color::Rgb(50, 48, 47),
            bg_highlight: Color::Rgb(60, 56, 54),

            border: Color::Rgb(215, 153, 33),
            border_active: Color::Rgb(250, 189, 47),

            focus_ring: Color::Rgb(215, 153, 33),
        }
    }

    /// Nord theme
    fn nord() -> Self {
        Self {
            primary: Color::Rgb(136, 192, 208),
            primary_bright: Color::Rgb(143, 188, 187),
            primary_dim: Color::Rgb(94, 129, 172),

            secondary: Color::Rgb(180, 142, 173),
            tertiary: Color::Rgb(208, 135, 112),

            success: Color::Rgb(163, 190, 140),
            warning: Color::Rgb(235, 203, 139),
            error: Color::Rgb(191, 97, 106),
            info: Color::Rgb(129, 161, 193),

            text: Color::Rgb(236, 239, 244),
            text_dim: Color::Rgb(216, 222, 233),
            text_muted: Color::Rgb(76, 86, 106),

            bg: Color::Rgb(46, 52, 64),
            bg_secondary: Color::Rgb(59, 66, 82),
            bg_highlight: Color::Rgb(67, 76, 94),

            border: Color::Rgb(136, 192, 208),
            border_active: Color::Rgb(143, 188, 187),

            focus_ring: Color::Rgb(136, 192, 208),
        }
    }

    /// Dracula theme
    fn dracula() -> Self {
        Self {
            primary: Color::Rgb(189, 147, 249),
            primary_bright: Color::Rgb(255, 121, 198),
            primary_dim: Color::Rgb(139, 233, 253),

            secondary: Color::Rgb(255, 121, 198),
            tertiary: Color::Rgb(255, 184, 108),

            success: Color::Rgb(80, 250, 123),
            warning: Color::Rgb(241, 250, 140),
            error: Color::Rgb(255, 85, 85),
            info: Color::Rgb(139, 233, 253),

            text: Color::Rgb(248, 248, 242),
            text_dim: Color::Rgb(226, 226, 220),
            text_muted: Color::Rgb(98, 114, 164),

            bg: Color::Rgb(40, 42, 54),
            bg_secondary: Color::Rgb(68, 71, 90),
            bg_highlight: Color::Rgb(55, 58, 76),

            border: Color::Rgb(189, 147, 249),
            border_active: Color::Rgb(255, 121, 198),

            focus_ring: Color::Rgb(189, 147, 249),
        }
    }

    /// Catppuccin Mocha theme
    fn catppuccin() -> Self {
        Self {
            primary: Color::Rgb(137, 180, 250),
            primary_bright: Color::Rgb(148, 226, 213),
            primary_dim: Color::Rgb(116, 199, 236),

            secondary: Color::Rgb(203, 166, 247),
            tertiary: Color::Rgb(250, 179, 135),

            success: Color::Rgb(166, 227, 161),
            warning: Color::Rgb(249, 226, 175),
            error: Color::Rgb(243, 139, 168),
            info: Color::Rgb(137, 220, 235),

            text: Color::Rgb(205, 214, 244),
            text_dim: Color::Rgb(186, 194, 222),
            text_muted: Color::Rgb(108, 112, 134),

            bg: Color::Rgb(30, 30, 46),
            bg_secondary: Color::Rgb(49, 50, 68),
            bg_highlight: Color::Rgb(69, 71, 90),

            border: Color::Rgb(137, 180, 250),
            border_active: Color::Rgb(203, 166, 247),

            focus_ring: Color::Rgb(137, 180, 250),
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
        // Default to Warm Paper Dark
        let preset = ThemePreset::WarmPaperDark;
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

    /// Check if current theme is light mode
    pub fn is_light_mode(&self) -> bool {
        matches!(self.current_preset, ThemePreset::WarmPaperLight)
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
        assert_eq!(tm.preset(), ThemePreset::WarmPaperDark);

        tm.cycle_next();
        assert_eq!(tm.preset(), ThemePreset::WarmPaperLight);

        tm.cycle_prev();
        assert_eq!(tm.preset(), ThemePreset::WarmPaperDark);
    }

    #[test]
    fn test_all_themes_have_colors() {
        for preset in ThemePreset::all() {
            let colors = ThemeColors::from_preset(*preset);
            // Just ensure we can access all color fields
            let _ = colors.primary;
            let _ = colors.text;
            let _ = colors.bg;
            let _ = colors.focus_ring;
        }
    }

    #[test]
    fn test_warm_paper_dark_is_default() {
        let tm = ThemeManager::new();
        assert_eq!(tm.preset().name(), "Warm Paper Dark");
    }
}
