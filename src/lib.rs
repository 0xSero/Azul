#![allow(dead_code)]
// The library is a thin facade for tests; some items are only used by the binary.

pub mod ai;
pub mod app;
pub mod browser;
pub mod chat;
pub mod config;
pub mod mascot;
pub mod memory;
pub mod rag;
pub mod scrape;
pub mod search;
pub mod storage;
pub mod tabs;
pub mod ui;

#[cfg(feature = "fixture-mode")]
pub mod fixtures;
