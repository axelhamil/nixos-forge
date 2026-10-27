mod common;

use common::write_executable;
use std::fs;
use std::process::Command;

#[test]
fn a_leading_double_dash_reaches_nh_with_the_arguments_after_it() {
    let dir = tempfile::tempdir().unwrap();
    let seen = dir.path().join("seen");
    let nh = dir.path().join("nh");
    write_executable(
        &nh,
        &format!("#!/bin/sh\nprintf '%s\\n' \"$@\" > {}\n", seen.display()),
    );

    let status = Command::new(env!("CARGO_BIN_EXE_forge"))
        .args(["update", "--", "--show-trace"])
        .env("FORGE_UPDATE_FLAKE", "github:alice/nixos")
        .env("PATH", dir.path())
        .status()
        .unwrap();

    assert!(status.success());
    assert_eq!(
        fs::read_to_string(&seen).unwrap(),
        "os\nswitch\n--\n--show-trace\n"
    );
}
