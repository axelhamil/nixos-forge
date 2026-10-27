#[path = "../../forge/tests/common/mod.rs"]
mod common;

use std::path::Path;

#[test]
fn help_lists_every_subcommand() {
    let dir = tempfile::tempdir().unwrap();
    let output = common::sandboxed(
        Path::new(env!("CARGO_BIN_EXE_forge-ci")),
        dir.path(),
        dir.path(),
        dir.path(),
    )
    .arg("--help")
    .output()
    .unwrap();

    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    for subcommand in [
        "plan",
        "build",
        "report",
        "promote",
        "badges",
        "reproducibility",
        "flaky",
        "flaky-plan",
        "flaky-report",
    ] {
        assert!(
            help.contains(&format!("  {subcommand} ")),
            "{subcommand} is missing from {help}"
        );
    }
}
