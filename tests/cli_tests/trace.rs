// Copyright 2023-2026 Martin Pool

//! Tests for trace-related options and behaviors of the Conserve CLI.

use std::fs::read_to_string;
use std::path::Path;

use assert_cmd::prelude::*;
use assert_fs::TempDir;
use assert_fs::prelude::*;
use predicates::prelude::*;

use crate::run_conserve;

#[test]
fn no_trace_timestamps_by_default() {
    let temp_dir = TempDir::new().unwrap();
    run_conserve()
        .args(["-D", "init"])
        .arg(temp_dir.child("archive").path())
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "TRACE conserve::termui::trace: Tracing enabled",
        ));
}

#[test]
fn trace_to_tmp_file() {
    let temp_dir = TempDir::new().unwrap();
    let assert = run_conserve()
        .args(["init", "--trace-tmp"])
        .arg(temp_dir.child("archive").path())
        .assert()
        .success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone())
        .expect("stderr should be valid UTF-8");
    println!("Stderr output:\n{stderr}");
    let tmp_file = stderr
        .lines()
        .find_map(|line| line.strip_prefix("  INFO conserve::termui::trace: Trace temp file: "))
        .expect("couldn't extract trace file path from stderr");
    let tmp_path = Path::new(tmp_file);
    assert!(tmp_path.exists(), "trace file {tmp_path:?} should exist");
    let trace_content = read_to_string(tmp_path).expect("read trace file");
    println!("Trace file content:\n{trace_content}");
    assert!(
        trace_content.contains("TRACE conserve::termui::trace: Tracing enabled"),
        "trace file should contain trace lines"
    );
}
