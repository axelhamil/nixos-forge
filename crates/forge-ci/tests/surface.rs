#[path = "../../forge/tests/common/mod.rs"]
mod common;

fn forge_ci(args: &[&str]) -> String {
    common::surface(env!("CARGO_BIN_EXE_forge-ci"), args, &[])
}

#[test]
fn surface_forge_ci_no_arguments() {
    insta::assert_snapshot!(forge_ci(&[]));
}

#[test]
fn surface_forge_ci_help() {
    insta::assert_snapshot!(forge_ci(&["--help"]));
}

#[test]
fn surface_forge_ci_plan_without_arguments() {
    insta::assert_snapshot!(forge_ci(&["plan"]));
}

#[test]
fn surface_forge_ci_plan_with_an_extra_argument() {
    insta::assert_snapshot!(forge_ci(&["plan", "push", "extra"]));
}

#[test]
fn surface_forge_ci_build_without_arguments() {
    insta::assert_snapshot!(forge_ci(&["build"]));
}

#[test]
fn surface_forge_ci_build_with_an_extra_argument() {
    insta::assert_snapshot!(forge_ci(&[
        "build",
        "x86_64-linux",
        "check",
        "out",
        "extra"
    ]));
}

#[test]
fn surface_forge_ci_report_without_arguments() {
    insta::assert_snapshot!(forge_ci(&["report"]));
}

#[test]
fn surface_forge_ci_report_with_an_extra_argument() {
    insta::assert_snapshot!(forge_ci(&["report", "extra"]));
}

#[test]
fn surface_forge_ci_promote_without_arguments() {
    insta::assert_snapshot!(forge_ci(&["promote"]));
}

#[test]
fn surface_forge_ci_promote_with_an_extra_argument() {
    insta::assert_snapshot!(forge_ci(&["promote", "results", "sha", "extra"]));
}

#[test]
fn surface_forge_ci_badges_without_arguments() {
    insta::assert_snapshot!(forge_ci(&["badges"]));
}

#[test]
fn surface_forge_ci_badges_with_an_extra_argument() {
    insta::assert_snapshot!(forge_ci(&["badges", "checks", "out", "extra"]));
}

#[test]
fn surface_forge_ci_reproducibility_without_arguments() {
    insta::assert_snapshot!(forge_ci(&["reproducibility"]));
}

#[test]
fn surface_forge_ci_reproducibility_with_an_extra_argument() {
    insta::assert_snapshot!(forge_ci(&["reproducibility", "--bogus"]));
}

#[test]
fn surface_forge_ci_flaky_without_arguments() {
    insta::assert_snapshot!(forge_ci(&["flaky"]));
}

#[test]
fn surface_forge_ci_flaky_with_an_extra_argument() {
    insta::assert_snapshot!(forge_ci(&[
        "flaky",
        "x86_64-linux",
        "check",
        "1",
        "out",
        "extra"
    ]));
}

#[test]
fn surface_forge_ci_flaky_plan_without_arguments() {
    insta::assert_snapshot!(forge_ci(&["flaky-plan"]));
}

#[test]
fn surface_forge_ci_flaky_plan_with_an_extra_argument() {
    insta::assert_snapshot!(forge_ci(&["flaky-plan", "extra"]));
}

#[test]
fn surface_forge_ci_flaky_report_without_arguments() {
    insta::assert_snapshot!(forge_ci(&["flaky-report"]));
}

#[test]
fn surface_forge_ci_flaky_report_with_an_extra_argument() {
    insta::assert_snapshot!(forge_ci(&["flaky-report", "results", "history", "extra"]));
}
