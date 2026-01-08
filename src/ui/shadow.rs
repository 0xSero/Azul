//! Shadow Border Widget
//!
//! Creates depth illusion by using darker colors on bottom/right edges,
//! simulating a light source from top-left casting shadows.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// Shadow colors for depth effect (matching OpenCode-inspired theme)
pub const SHADOW_DARK: Color = Color::Rgb(10, 10, 12); // Deep shadow (bottom-right) #0a0a0c
pub const SHADOW_MID: Color = Color::Rgb(14, 14, 16); // Medium shadow (transition) #0e0e10

/// A border widget that creates depth through asymmetric coloring.
/// Top/left edges use the light color, bottom/right use shadow colors.
pub struct ShadowBorder {
    /// Color for illuminated edges (top, left)
    pub light_color: Color,
    /// Color for shadowed edges (bottom, right) - auto-derived if None
    pub shadow_color: Option<Color>,
    /// Whether to use rounded corners
    pub rounded: bool,
}

impl ShadowBorder {
    pub fn new(light_color: Color) -> Self {
        Self {
            light_color,
            shadow_color: None,
            rounded: true,
        }
    }

    pub fn with_shadow(mut self, shadow: Color) -> Self {
        self.shadow_color = Some(shadow);
        self
    }

    pub fn rounded(mut self, rounded: bool) -> Self {
        self.rounded = rounded;
        self
    }

    /// Derive shadow color from light color (darken by ~40%)
    fn derive_shadow(&self) -> Color {
        if let Some(shadow) = self.shadow_color {
            return shadow;
        }

        match self.light_color {
            Color::Rgb(r, g, b) => {
                // Darken by reducing values by 40%
                Color::Rgb(
                    (r as f32 * 0.6) as u8,
                    (g as f32 * 0.6) as u8,
                    (b as f32 * 0.6) as u8,
                )
            }
            _ => SHADOW_DARK,
        }
    }
}

impl Widget for ShadowBorder {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let x0 = area.x;
        let y0 = area.y;
        let w = area.width;
        let h = area.height;

        let light_style = Style::default().fg(self.light_color);
        let shadow = self.derive_shadow();
        let shadow_style = Style::default().fg(shadow);

        // Corner characters
        let (tl, tr, bl, br) = if self.rounded {
            ("╭", "╮", "╰", "╯")
        } else {
            ("┌", "┐", "└", "┘")
        };

        // Top edge (light) - excluding right corner
        for dx in 1..w.saturating_sub(1) {
            if let Some(cell) = buf.cell_mut((x0 + dx, y0)) {
                cell.set_symbol("─").set_style(light_style);
            }
        }

        // Left edge (light) - excluding corners
        for dy in 1..h.saturating_sub(1) {
            if let Some(cell) = buf.cell_mut((x0, y0 + dy)) {
                cell.set_symbol("│").set_style(light_style);
            }
        }

        // Bottom edge (shadow) - excluding left corner
        for dx in 1..w {
            if let Some(cell) = buf.cell_mut((x0 + dx, y0 + h - 1)) {
                cell.set_symbol("─").set_style(shadow_style);
            }
        }

        // Right edge (shadow) - excluding corners
        for dy in 1..h.saturating_sub(1) {
            if let Some(cell) = buf.cell_mut((x0 + w - 1, y0 + dy)) {
                cell.set_symbol("│").set_style(shadow_style);
            }
        }

        // Corners: top-left and top-right are light, bottom corners are shadow
        if let Some(cell) = buf.cell_mut((x0, y0)) {
            cell.set_symbol(tl).set_style(light_style);
        }
        if let Some(cell) = buf.cell_mut((x0 + w - 1, y0)) {
            // Top-right: transition - use mid tone
            cell.set_symbol(tr)
                .set_style(Style::default().fg(SHADOW_MID));
        }
        if let Some(cell) = buf.cell_mut((x0, y0 + h - 1)) {
            // Bottom-left: transition - use mid tone
            cell.set_symbol(bl)
                .set_style(Style::default().fg(SHADOW_MID));
        }
        if let Some(cell) = buf.cell_mut((x0 + w - 1, y0 + h - 1)) {
            cell.set_symbol(br).set_style(shadow_style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shadow_derive() {
        let border = ShadowBorder::new(Color::Rgb(100, 100, 100));
        let shadow = border.derive_shadow();
        assert!(matches!(shadow, Color::Rgb(60, 60, 60)));
    }
}
