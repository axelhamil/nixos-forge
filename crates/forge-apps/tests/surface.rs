#[path = "../../forge/tests/common/mod.rs"]
mod common;

fn stow_dotfiles(args: &[&str], envs: &[(&str, &str)]) -> String {
    common::surface(env!("CARGO_BIN_EXE_stow-dotfiles"), args, envs)
}

#[test]
fn surface_stow_dotfiles_no_arguments() {
    insta::assert_snapshot!(stow_dotfiles(&[], &[]));
}

#[test]
fn surface_stow_dotfiles_help() {
    insta::assert_snapshot!(stow_dotfiles(&["--help"], &[]));
}

#[test]
fn surface_stow_dotfiles_unknown_flag() {
    insta::assert_snapshot!(stow_dotfiles(
        &["--bogus", "dotfiles", "target", "state"],
        &[("USER", "alice")]
    ));
}
