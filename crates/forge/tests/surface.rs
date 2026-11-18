mod common;

fn forge(args: &[&str], envs: &[(&str, &str)]) -> String {
    common::surface(env!("CARGO_BIN_EXE_forge"), args, envs)
}

#[test]
fn surface_forge_no_arguments() {
    insta::assert_snapshot!(forge(&[], &[]));
}

#[test]
fn surface_forge_help_flag() {
    insta::assert_snapshot!(forge(&["--help"], &[]));
}

#[test]
fn surface_forge_help_subcommand() {
    insta::assert_snapshot!(forge(&["help"], &[]));
}

#[test]
fn surface_forge_version() {
    insta::assert_snapshot!(forge(&["--version"], &[]));
}

#[test]
fn surface_forge_doctor_all_with_history() {
    insta::assert_snapshot!(forge(&["doctor", "--all", "--history"], &[]));
}

#[test]
fn surface_forge_enroll_without_method() {
    insta::assert_snapshot!(forge(&["enroll"], &[]));
}

#[test]
fn surface_forge_dual_boot_without_action() {
    insta::assert_snapshot!(forge(&["dual-boot"], &[]));
}

#[test]
fn surface_forge_dual_boot_unknown_action() {
    insta::assert_snapshot!(forge(&["dual-boot", "bogus"], &[]));
}

#[test]
fn surface_forge_dual_boot_install_beside_windows_without_arguments() {
    insta::assert_snapshot!(forge(&["dual-boot", "install-beside-windows"], &[]));
}

#[test]
fn surface_forge_upstream_two_starters() {
    insta::assert_snapshot!(forge(&["upstream", "a", "b"], &[]));
}

#[test]
fn surface_forge_upstream_help() {
    insta::assert_snapshot!(forge(&["upstream", "--help"], &[]));
}

#[test]
fn surface_forge_init_help() {
    insta::assert_snapshot!(forge(&["init", "--help"], &[]));
}

#[test]
fn surface_forge_init_unknown_flag() {
    insta::assert_snapshot!(forge(&["init", "--bogus"], &[]));
}

#[test]
fn surface_forge_update_without_flake() {
    insta::assert_snapshot!(forge(&["update"], &[]));
}

#[test]
fn surface_forge_auto_upgrade_without_config() {
    insta::assert_snapshot!(forge(&["auto-upgrade"], &[]));
}

#[test]
fn surface_forge_doctor_missing_config_file() {
    insta::assert_snapshot!(forge(
        &["doctor"],
        &[("FORGE_DOCTOR_CONFIG", "{TMP}/missing.json")]
    ));
}

#[test]
fn surface_forge_unknown_command() {
    insta::assert_snapshot!(forge(&["nothere"], &[]));
}
