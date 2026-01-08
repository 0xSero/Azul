#![cfg(feature = "test-support")]

mod support;

use azul_browse::app::{Focus, PanelMode, ViewMode};
use support::{load_page, render_app, set_page, test_app};

fn app_with_page(name: &str) -> azul_browse::app::App {
    let mut app = test_app();
    set_page(&mut app, load_page(name));
    app.focus = Focus::Content;
    app.panel_mode = PanelMode::None;
    app.chat_focused = false;
    app
}

#[test]
fn snapshot_default_120x40() {
    let mut app = app_with_page("fixture_local_article");
    app.view_mode = ViewMode::Rendered;
    app.format_text = true;

    let output = render_app(&mut app, 120, 40);
    insta::assert_snapshot!("default_120x40", output);
}

#[test]
fn snapshot_focus_mode_120x40() {
    let mut app = app_with_page("fixture_local_article");
    app.panes.focus_mode = true;

    let output = render_app(&mut app, 120, 40);
    insta::assert_snapshot!("focus_mode_120x40", output);
}

#[test]
fn snapshot_chat_visible_120x40() {
    let mut app = app_with_page("fixture_local_article");
    app.panes.show_chat();
    app.chat_focused = true;

    let output = render_app(&mut app, 120, 40);
    insta::assert_snapshot!("chat_visible_120x40", output);
}

#[test]
fn snapshot_links_sidebar_120x40() {
    let mut app = app_with_page("fixture_local_links");
    app.view_mode = ViewMode::Rendered;
    app.format_text = true;

    let output = render_app(&mut app, 120, 40);
    insta::assert_snapshot!("links_sidebar_120x40", output);
}

#[test]
fn snapshot_pdf_view_120x40() {
    let mut app = app_with_page("fixture_local_sample_pdf");
    app.view_mode = ViewMode::Rendered;
    app.format_text = false;

    let output = render_app(&mut app, 120, 40);
    insta::assert_snapshot!("pdf_view_120x40", output);
}

#[test]
fn snapshot_rendered_vs_raw_120x40() {
    let mut rendered = app_with_page("fixture_local_article");
    rendered.view_mode = ViewMode::Rendered;
    rendered.format_text = true;
    let rendered_output = render_app(&mut rendered, 120, 40);
    insta::assert_snapshot!("content_rendered_120x40", rendered_output);

    let mut raw = app_with_page("fixture_local_article");
    raw.view_mode = ViewMode::Raw;
    raw.format_text = false;
    let raw_output = render_app(&mut raw, 120, 40);
    insta::assert_snapshot!("content_raw_120x40", raw_output);
}

#[test]
fn snapshot_default_90x30() {
    let mut app = app_with_page("fixture_local_article");
    app.view_mode = ViewMode::Rendered;
    app.format_text = true;

    let output = render_app(&mut app, 90, 30);
    insta::assert_snapshot!("default_90x30", output);
}
