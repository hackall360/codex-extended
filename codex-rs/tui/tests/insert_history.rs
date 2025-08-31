#![cfg(windows)]

use std::io;

use codex_tui::insert_history::{ResetScrollRegion, SetScrollRegion};
use crossterm::Command;

#[test]
fn set_scroll_region_execute_winapi_returns_error() {
    let cmd = SetScrollRegion(1..2);
    let err = cmd.execute_winapi().expect_err("expected error");
    assert_eq!(err.kind(), io::ErrorKind::Other);
}

#[test]
fn reset_scroll_region_execute_winapi_returns_error() {
    let cmd = ResetScrollRegion;
    let err = cmd.execute_winapi().expect_err("expected error");
    assert_eq!(err.kind(), io::ErrorKind::Other);
}
