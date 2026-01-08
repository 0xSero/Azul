use ratatui::layout::Rect;

use crate::app::Focus;

const GAP_WIDTH: u16 = 1;
const MIN_CONTENT_WIDTH: u16 = 32;
const MIN_SIDEBAR_WIDTH: u16 = 18;
const MIN_ASSISTANT_WIDTH: u16 = 24;
const ASSISTANT_FRACTION: f32 = 0.28;

#[derive(Debug, Clone, Copy)]
pub struct PaneContext {
    pub focus: Focus,
    pub chat_focused: bool,
    pub hard_content: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneLayout {
    pub sidebar: Option<Rect>,
    pub content: Option<Rect>,
    pub assistant: Option<Rect>,
}

#[derive(Debug, Clone)]
pub struct PaneManager {
    pub sidebar_width: u16,
    pub focus_mode: bool,
    pub chat_visible: bool,
    pub assistant_dismissed: bool,
}

impl PaneManager {
    pub fn new(sidebar_width: u16) -> Self {
        Self {
            sidebar_width,
            focus_mode: false,
            chat_visible: false,
            assistant_dismissed: false,
        }
    }

    pub fn assistant_visible(&self, context: PaneContext) -> bool {
        if self.chat_visible || context.chat_focused {
            return true;
        }

        context.hard_content && !self.assistant_dismissed
    }

    pub fn show_chat(&mut self) {
        self.chat_visible = true;
        self.assistant_dismissed = false;
    }

    pub fn hide_chat(&mut self) {
        self.chat_visible = false;
        self.assistant_dismissed = true;
    }

    pub fn clear_auto_dismiss(&mut self) {
        self.assistant_dismissed = false;
    }

    pub fn toggle_focus_mode(&mut self) {
        self.focus_mode = !self.focus_mode;
    }

    pub fn layout(&self, area: Rect, context: PaneContext) -> PaneLayout {
        let assistant_visible = self.assistant_visible(context);

        if self.focus_mode {
            return self.focused_layout(area, context, assistant_visible);
        }

        if assistant_visible {
            self.three_pane_layout(area)
        } else {
            self.two_pane_layout(area)
        }
    }

    fn focused_layout(
        &self,
        area: Rect,
        context: PaneContext,
        assistant_visible: bool,
    ) -> PaneLayout {
        let show_assistant = assistant_visible && context.chat_focused;
        let show_sidebar = !show_assistant && context.focus == Focus::Sidebar;
        let show_content = !show_assistant && !show_sidebar;

        PaneLayout {
            sidebar: show_sidebar.then_some(area),
            content: show_content.then_some(area),
            assistant: show_assistant.then_some(area),
        }
    }

    fn two_pane_layout(&self, area: Rect) -> PaneLayout {
        let available = area.width.saturating_sub(GAP_WIDTH);
        let sidebar_width = clamp_sidebar_width(self.sidebar_width, available, MIN_CONTENT_WIDTH);

        let sidebar = Rect {
            x: area.x,
            y: area.y,
            width: sidebar_width,
            height: area.height,
        };

        let content_x = area.x + sidebar_width + GAP_WIDTH;
        let content_width = area.width.saturating_sub(sidebar_width + GAP_WIDTH);
        let content = Rect {
            x: content_x,
            y: area.y,
            width: content_width,
            height: area.height,
        };

        PaneLayout {
            sidebar: Some(sidebar),
            content: Some(content),
            assistant: None,
        }
    }

    fn three_pane_layout(&self, area: Rect) -> PaneLayout {
        let available = area.width.saturating_sub(GAP_WIDTH * 2);
        let sidebar_width = clamp_sidebar_width(
            self.sidebar_width,
            available,
            MIN_CONTENT_WIDTH + MIN_ASSISTANT_WIDTH,
        );

        let remaining = available.saturating_sub(sidebar_width);
        let preferred_assistant = ((area.width as f32) * ASSISTANT_FRACTION).round() as u16;
        let assistant_width =
            clamp_assistant_width(preferred_assistant, remaining, MIN_CONTENT_WIDTH);
        let content_width = remaining.saturating_sub(assistant_width);

        let sidebar = Rect {
            x: area.x,
            y: area.y,
            width: sidebar_width,
            height: area.height,
        };

        let content_x = area.x + sidebar_width + GAP_WIDTH;
        let content = Rect {
            x: content_x,
            y: area.y,
            width: content_width,
            height: area.height,
        };

        let assistant_x = content_x + content_width + GAP_WIDTH;
        let assistant = Rect {
            x: assistant_x,
            y: area.y,
            width: assistant_width,
            height: area.height,
        };

        PaneLayout {
            sidebar: Some(sidebar),
            content: Some(content),
            assistant: Some(assistant),
        }
    }
}

fn clamp_sidebar_width(requested: u16, available: u16, min_content: u16) -> u16 {
    if available == 0 {
        return 0;
    }

    let max_sidebar = available.saturating_sub(min_content).max(1);
    let min_sidebar = MIN_SIDEBAR_WIDTH.min(max_sidebar);
    requested.clamp(min_sidebar, max_sidebar)
}

fn clamp_assistant_width(requested: u16, available: u16, min_content: u16) -> u16 {
    if available == 0 {
        return 0;
    }

    let max_assistant = available.saturating_sub(min_content).max(1);
    let min_assistant = MIN_ASSISTANT_WIDTH.min(max_assistant);
    requested.clamp(min_assistant, max_assistant)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context() -> PaneContext {
        PaneContext {
            focus: Focus::Content,
            chat_focused: false,
            hard_content: false,
        }
    }

    #[test]
    fn default_layout_is_two_pane() {
        let panes = PaneManager::new(26);
        let layout = panes.layout(Rect::new(0, 0, 120, 40), context());

        assert!(layout.assistant.is_none());
        assert_eq!(layout.sidebar.unwrap().width, 26);
        assert_eq!(layout.content.unwrap().width, 120 - 26 - GAP_WIDTH);
    }

    #[test]
    fn focus_mode_shows_single_pane() {
        let mut panes = PaneManager::new(26);
        panes.focus_mode = true;

        let mut ctx = context();
        ctx.focus = Focus::Sidebar;
        let layout = panes.layout(Rect::new(0, 0, 120, 40), ctx);
        assert!(layout.content.is_none());
        assert!(layout.assistant.is_none());
        assert_eq!(layout.sidebar.unwrap().width, 120);
    }

    #[test]
    fn assistant_auto_visibility_for_hard_content() {
        let panes = PaneManager::new(26);
        let mut ctx = context();
        ctx.hard_content = true;
        let layout = panes.layout(Rect::new(0, 0, 120, 40), ctx);
        assert!(layout.assistant.is_some());
    }

    #[test]
    fn assistant_toggle_changes_visibility() {
        let mut panes = PaneManager::new(26);
        let ctx = context();
        assert!(!panes.assistant_visible(ctx));

        panes.show_chat();
        assert!(panes.assistant_visible(ctx));

        panes.hide_chat();
        assert!(!panes.assistant_visible(ctx));
    }

    #[test]
    fn assistant_dismissed_blocks_auto_show() {
        let mut panes = PaneManager::new(26);
        panes.assistant_dismissed = true;

        let mut ctx = context();
        ctx.hard_content = true;
        assert!(!panes.assistant_visible(ctx));
    }
}
