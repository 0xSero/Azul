use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

// Warm Paper border - golden glow animation
pub const NEON_COLORS: &[Color] = &[
    Color::Rgb(194, 154, 108),  // Warm gold
    Color::Rgb(214, 168, 92),   // Bright amber
    Color::Rgb(180, 140, 90),   // Deep gold
    Color::Rgb(160, 128, 80),   // Tan
    Color::Rgb(140, 115, 75),   // Bronze
];

pub struct NeonBorder {
    pub phase: usize,
}

impl Widget for NeonBorder {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 2 || area.height < 2 {
            return;
        }

        let mut idx = 0usize;
        let x0 = area.x;
        let y0 = area.y;
        let w = area.width;
        let h = area.height;

        // Top edge
        for dx in 0..w {
            let color = color_at(self.phase, idx);
            if let Some(cell) = buf.cell_mut((x0 + dx, y0)) {
                cell.set_symbol("─").set_style(Style::default().fg(color));
            }
            idx += 1;
        }

        // Right edge (excluding corners)
        for dy in 1..h.saturating_sub(1) {
            let color = color_at(self.phase, idx);
            if let Some(cell) = buf.cell_mut((x0 + w - 1, y0 + dy)) {
                cell.set_symbol("│").set_style(Style::default().fg(color));
            }
            idx += 1;
        }

        // Bottom edge
        if h > 1 {
            for dx in (0..w).rev() {
                let color = color_at(self.phase, idx);
                if let Some(cell) = buf.cell_mut((x0 + dx, y0 + h - 1)) {
                    cell.set_symbol("─").set_style(Style::default().fg(color));
                }
                idx += 1;
            }
        }

        // Left edge (excluding corners)
        if w > 1 {
            for dy in (1..h.saturating_sub(1)).rev() {
                let color = color_at(self.phase, idx);
                if let Some(cell) = buf.cell_mut((x0, y0 + dy)) {
                    cell.set_symbol("│").set_style(Style::default().fg(color));
                }
                idx += 1;
            }
        }

        let top_left_idx = 0usize;
        let top_right_idx = (w - 1) as usize;
        let bottom_right_idx = w as usize + h.saturating_sub(2) as usize;
        let bottom_left_idx = bottom_right_idx + (w - 1) as usize;

        if let Some(cell) = buf.cell_mut((x0, y0)) {
            cell.set_symbol("╭")
                .set_fg(color_at(self.phase, top_left_idx));
        }
        if let Some(cell) = buf.cell_mut((x0 + w - 1, y0)) {
            cell.set_symbol("╮")
                .set_fg(color_at(self.phase, top_right_idx));
        }
        if let Some(cell) = buf.cell_mut((x0 + w - 1, y0 + h - 1)) {
            cell.set_symbol("╯")
                .set_fg(color_at(self.phase, bottom_right_idx));
        }
        if let Some(cell) = buf.cell_mut((x0, y0 + h - 1)) {
            cell.set_symbol("╰")
                .set_fg(color_at(self.phase, bottom_left_idx));
        }
    }
}

fn color_at(phase: usize, step: usize) -> Color {
    let palette = NEON_COLORS;
    palette[(phase + step) % palette.len()]
}
