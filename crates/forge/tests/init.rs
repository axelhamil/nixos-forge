mod common;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use common::{find_program, sandboxed};
use fancy_regex::Regex;
use tempfile::TempDir;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/init");

struct Fork {
    dir: TempDir,
    root: PathBuf,
    home: PathBuf,
    runtime: PathBuf,
    bin: PathBuf,
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).unwrap();
        }
    }
}

impl Fork {
    fn new() -> Self {
        Self::named(OsStr::new("fork"))
    }

    fn named(name: &OsStr) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join(name);
        let home = dir.path().join("home");
        let runtime = dir.path().join("runtime");
        let bin = dir.path().join("bin");
        copy_tree(Path::new(FIXTURES), &root);
        for made in [&home, &runtime, &bin] {
            fs::create_dir_all(made).unwrap();
        }
        fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();
        for program in ["git", "ssh-keygen"] {
            symlink(find_program(program), bin.join(program)).unwrap();
        }
        let fork = Self {
            dir,
            root,
            home,
            runtime,
            bin,
        };
        fork.git(&["init", "-q"]);
        fork.git(&["add", "-A"]);
        fork.git(&[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.org",
            "commit",
            "-q",
            "-m",
            "fork",
        ]);
        fork
    }

    fn with_origin(origin: &str) -> Self {
        let fork = Self::new();
        fork.git(&["remote", "add", "origin", origin]);
        fork
    }

    fn command(&self, program: &Path) -> Command {
        let mut command = sandboxed(program, &self.root, &self.bin, &self.home);
        command
            .env("TMPDIR", self.dir.path())
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .stdin(Stdio::null());
        command
    }

    fn git(&self, args: &[&str]) -> String {
        let output = self
            .command(&self.bin.join("git"))
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success(), "git {args:?}: {output:?}");
        String::from_utf8(output.stdout).unwrap()
    }

    fn init(&self, args: &[&str]) -> Output {
        self.command(Path::new(env!("CARGO_BIN_EXE_forge")))
            .arg("init")
            .args(args)
            .output()
            .unwrap()
    }

    fn initialised(&self, args: &[&str]) -> String {
        let output = self.init(args);
        assert!(output.status.success(), "init {args:?}: {output:?}");
        String::from_utf8(output.stdout).unwrap()
    }

    fn refuses(&self, expected: &str, args: &[&str]) {
        let output = self.init(args);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(1), "init {args:?}: {stderr}");
        assert!(
            stderr.contains(expected),
            "init {args:?} failed with: {stderr}"
        );
    }

    fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.root.join(relative)).unwrap()
    }

    fn exists(&self, relative: &str) -> bool {
        self.root.join(relative).exists()
    }

    fn status(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .git(&["status", "--porcelain", "--no-renames"])
            .lines()
            .map(str::to_owned)
            .collect();
        lines.sort();
        lines
    }
}

fn has_line(text: &str, pattern: &str) -> bool {
    let regex = Regex::new(&format!("^(?:{pattern})$")).unwrap();
    text.lines().any(|line| regex.is_match(line).unwrap())
}

fn assert_lines(fork: &Fork, file: &str, patterns: &[&str]) {
    let text = fork.read(file);
    for pattern in patterns {
        assert!(
            has_line(&text, pattern),
            "{file} has no line matching: {pattern}\n{text}"
        );
    }
}

fn signer(text: &str) -> Option<String> {
    let line = text
        .lines()
        .skip_while(|line| *line != "    signers = [")
        .nth(1)?;
    let regex = Regex::new(r#"^      "(ssh-ed25519 [A-Za-z0-9+/]*=* deploy)"$"#).unwrap();
    let captures = regex.captures(line).ok()??;
    Some(captures.get(1)?.as_str().to_owned())
}

const LAPTOP: [&str; 24] = [
    "--yes",
    "--host",
    "my-box",
    "--profile",
    "laptop",
    "--login",
    "bob",
    "--full-name",
    "Bob \"the\" Builder ${x}",
    "--email",
    "bob@example.net",
    "--time-zone",
    "Europe/Paris",
    "--keyboard",
    "fr",
    "--locale",
    "fr_FR.UTF-8",
    "--features",
    "disk,docker",
    "--hardware",
    "nvidia framework-13-7040-amd",
    "--apps",
    "firefox,vlc",
    "--keep-pi",
];

#[test]
fn a_laptop_fork_gets_its_host_person_and_signing_key_and_loses_the_examples() {
    let fork = Fork::new();
    fork.initialised(&LAPTOP);

    let host = "modules/hosts/my-box/default.nix";
    assert_lines(
        &fork,
        host,
        &[
            r"  configurations\.nixos\.my-box = \{",
            r#"    system = "x86_64-linux";.*"#,
            r#"    profile = "laptop";.*"#,
            r#"    flake = "/home/bob/nixos";"#,
            r#"    # mirrors = \[ "https://codeberg.org/bob/nixos.git" \]; # .*"#,
            r#"    timeZone = "Europe/Paris";.*"#,
            r#"    keyboard = "fr";.*"#,
            r#"    locale = "fr_FR\.UTF-8";.*"#,
            r"    users\.bob = \{",
            r#"      "disk" # .*"#,
            r#"      "docker" # .*"#,
            r#"      # "secure-boot" # .*"#,
            r#"      # "libvirt" # .*"#,
            r#"      "nvidia" # .*"#,
            r#"      "framework-13-7040-amd""#,
            r#"      # "common-pc-ssd" # .*"#,
            r#"      "firefox" # .*"#,
            r#"      "vlc""#,
            r#"      # "hyprland" # .*"#,
            r#"      # "direnv" # .*"#,
            r"    signers = \[.*",
        ],
    );
    assert!(
        !fork.read(host).contains("alice"),
        "{host} still names alice"
    );
    let pi = "modules/hosts/example-pi/default.nix";
    assert!(
        fork.read(pi)
            .lines()
            .any(|line| line == "    users.bob = {")
    );

    let person = "modules/users/bob/default.nix";
    assert_lines(
        &fork,
        person,
        &[
            r"  users\.bob = \{",
            r#"    fullName = "Bob \\"the\\" Builder \\\$\{x\}";.*"#,
            r#"    email = "bob@example\.net";.*"#,
        ],
    );
    assert!(!fork.read(person).contains("alice"));
    assert!(!fork.exists("modules/users/alice"));
    assert!(!fork.exists("modules/hosts/example"));
    assert!(!fork.exists("modules/hosts/example-server"));
    assert!(fork.exists(pi));
    assert_eq!(
        fork.status(),
        [
            "A  modules/hosts/my-box/default.nix",
            "A  modules/users/bob/default.nix",
            "D  modules/hosts/example-server/default.nix",
            "D  modules/hosts/example/default.nix",
            "D  modules/users/alice/default.nix",
            "M  README.md",
            "M  modules/hosts/example-pi/default.nix",
        ]
    );

    fork.refuses(
        "modules/hosts/example/default.nix is missing: init starts from the example host, which a previous run already replaced",
        &[
            "--yes",
            "--host",
            "other",
            "--login",
            "bob",
            "--full-name",
            "Bob",
            "--email",
            "",
        ],
    );
}

#[test]
fn the_private_key_stays_outside_the_fork_readable_by_its_owner_and_matches_the_signers() {
    let fork = Fork::new();
    let printed = fork.initialised(&LAPTOP);

    let signing_key = signer(&fork.read("modules/hosts/my-box/default.nix"))
        .expect("the host holds no deploy signing key");
    let pi = "modules/hosts/example-pi/default.nix";
    assert_eq!(
        signer(&fork.read(pi)).as_deref(),
        Some(signing_key.as_str())
    );

    let command = Regex::new(
        r"^  gh secret set DEPLOY_SIGNING_KEY --env deploy < ([^ ]+) && shred -u ([^ ]+)$",
    )
    .unwrap();
    let (stored, shredded) = printed
        .lines()
        .find_map(|line| {
            let captures = command.captures(line).ok()??;
            Some((captures[1].to_owned(), captures[2].to_owned()))
        })
        .unwrap_or_else(|| panic!("init does not print the command storing the key: {printed}"));
    assert_eq!(stored, shredded);
    let key_file = PathBuf::from(&stored);
    assert!(key_file.starts_with(&fork.runtime), "{stored}");
    let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&key_file), 0o600);
    assert_eq!(mode(key_file.parent().unwrap()), 0o700);
    assert!(!key_file.with_extension("pub").exists());
    assert!(
        printed
            .lines()
            .filter(|line| !line.starts_with("  gh secret set"))
            .any(|line| line.ends_with(&format!("shred -u {stored}"))),
        "init does not print the way through the GitHub web page: {printed}"
    );
    assert!(!printed.contains("PRIVATE KEY"));

    let public = fork
        .command(&fork.bin.join("ssh-keygen"))
        .args(["-y", "-f", &stored])
        .output()
        .unwrap();
    let public = String::from_utf8(public.stdout).unwrap();
    let public: Vec<&str> = public.split_whitespace().take(2).collect();
    assert_eq!(format!("{} deploy", public.join(" ")), signing_key);
    let key_line = fs::read_to_string(&key_file)
        .unwrap()
        .lines()
        .nth(1)
        .unwrap()
        .to_owned();
    for entry in walkdir::WalkDir::new(&fork.root)
        .into_iter()
        .filter_entry(|entry| entry.file_name() != ".git")
        .flatten()
        .filter(|entry| entry.file_type().is_file())
    {
        let text = fs::read(entry.path()).unwrap();
        assert!(
            !String::from_utf8_lossy(&text).contains(&key_line),
            "{} holds the private key",
            entry.path().display()
        );
    }
    let staged = fork
        .command(&fork.bin.join("git"))
        .args(["grep", "--cached", "-qF", "--", &key_line])
        .status()
        .unwrap();
    assert!(!staged.success(), "the private key is staged");
}

#[test]
fn refused_answers_leave_the_checkout_untouched() {
    let fork = Fork::new();
    let base = ["--yes", "--full-name", "Bob", "--email", ""];
    let with =
        |extra: &[&'static str]| -> Vec<&str> { base.iter().chain(extra).copied().collect() };

    fork.refuses(
        "--host: modules/hosts/example-server already exists: pick another host name, init never overwrites a host",
        &with(&["--host", "example-server", "--login", "bob", "--keep-server"]),
    );
    fork.refuses("1 to 63", &with(&["--host", "-bad", "--login", "bob"]));
    fork.refuses("--login is required with --yes", &with(&["--host", "box"]));
    fork.refuses(
        "--email is required with --yes (--email '' for none)",
        &[
            "--yes",
            "--host",
            "box",
            "--login",
            "bob",
            "--full-name",
            "Bob",
        ],
    );
    fork.refuses(
        "--features: unknown feature \"nope\"; known: disk secure-boot fido2",
        &with(&["--host", "box", "--login", "bob", "--features", "disk,nope"]),
    );
    fs::create_dir_all(fork.root.join("modules/users/carol")).unwrap();
    fork.refuses(
        "modules/users/carol already exists",
        &with(&["--host", "box", "--login", "carol"]),
    );
    fs::remove_dir(fork.root.join("modules/users/carol")).unwrap();

    assert!(fork.status().is_empty(), "{:?}", fork.status());
    assert_eq!(fs::read_dir(&fork.runtime).unwrap().count(), 0);
}

#[test]
fn usage_errors_exit_2_and_help_exits_0() {
    let fork = Fork::new();

    let unknown = fork.init(&["--yes", "--hostname", "box"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&unknown.stderr),
        "forge init: unknown option --hostname (see --help)\n"
    );

    let missing = fork.init(&["--host"]);
    assert_eq!(missing.status.code(), Some(2));
    assert_eq!(
        String::from_utf8_lossy(&missing.stderr),
        "forge init: --host needs a value\n"
    );

    let help = fork.init(&["--help"]);
    assert!(help.status.success());
    assert!(
        String::from_utf8_lossy(&help.stdout).starts_with("Usage: nix run .#init -- [options]")
    );
    assert_eq!(fork.status(), Vec::<String>::new());
}

#[test]
fn outside_a_git_checkout_init_says_where_to_run_it() {
    let fork = Fork::new();
    let outside = fork.home.clone();

    let output = fork
        .command(Path::new(env!("CARGO_BIN_EXE_forge")))
        .current_dir(outside)
        .args(["init", "--yes"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "forge init: run it inside a git checkout of your fork\n"
    );
}

#[test]
fn an_aarch64_fork_defaults_to_the_pi_profile_and_gets_a_key_of_its_own() {
    let laptop = Fork::new();
    laptop.initialised(&LAPTOP);
    let laptop_key = signer(&laptop.read("modules/hosts/my-box/default.nix"));

    let fork = Fork::new();
    fork.initialised(&[
        "--yes",
        "--host",
        "pi-box",
        "--system",
        "aarch64-linux",
        "--login",
        "bob",
        "--full-name",
        "Bob",
        "--email",
        "",
    ]);

    let host = "modules/hosts/pi-box/default.nix";
    let key = signer(&fork.read(host));
    assert!(key.is_some() && key != laptop_key);
    assert_lines(
        &fork,
        host,
        &[
            r#"    system = "aarch64-linux";.*"#,
            r#"    profile = "pi";.*"#,
            r#"      "ssh" # .*"#,
            r#"      # "disk" # .*"#,
            r#"      "raspberry-pi-5" # .*"#,
            r#"      # "firefox" # .*"#,
            r#"      # "hyprland" # .*"#,
        ],
    );
    assert_lines(
        &fork,
        "modules/users/bob/default.nix",
        &[r#"    # email = "bob@example\.org";.*"#],
    );
    assert!(!fork.exists("modules/hosts/example-pi"));
}

#[test]
fn nix_keywords_are_quoted_and_the_readme_points_at_the_new_host_and_person() {
    let fork = Fork::new();
    fork.initialised(&[
        "--yes",
        "--host",
        "rec",
        "--login",
        "in",
        "--full-name",
        "In",
        "--email",
        "",
    ]);

    assert_lines(
        &fork,
        "modules/hosts/rec/default.nix",
        &[
            r#"  configurations\.nixos\."rec" = \{"#,
            r#"    users\."in" = \{"#,
            r#"      "hyprland" # .*"#,
            r#"      "kitty" # .*"#,
            r#"      "lock" # .*"#,
            r#"      "direnv" # .*"#,
            r#"      "nix-ld" # .*"#,
            r#"      "greetd" # .*"#,
            r#"      "printing" # .*"#,
            r#"      "firmware" # .*"#,
            r#"      "audio" # .*"#,
            r#"      "bluetooth" # .*"#,
        ],
    );
    assert_lines(
        &fork,
        "modules/users/in/default.nix",
        &[
            r#"      "zsh" # .*"#,
            r#"  users\."in" = \{"#,
            r#".* an account with .users\."in".*"#,
        ],
    );
    let readme = fork.read("README.md");
    assert!(readme.contains("Your [`rec`](modules/hosts/rec/default.nix) host"));
    assert!(readme.contains("[`modules/users/in/default.nix`](modules/users/in/default.nix)"));
}

#[test]
fn keeping_alice_keeps_her_account_on_the_kept_server() {
    let fork = Fork::new();
    fork.initialised(&[
        "--yes",
        "--host",
        "keeps-alice",
        "--login",
        "alice",
        "--full-name",
        "Alice Fork",
        "--email",
        "",
        "--keep-server",
    ]);

    assert_lines(
        &fork,
        "modules/users/alice/default.nix",
        &[r#"    fullName = "Alice Fork";.*"#],
    );
    assert_lines(
        &fork,
        "modules/hosts/example-server/default.nix",
        &[
            r"    users\.alice = \{",
            r#"      # "auto-upgrade" # .*"#,
            r#"    # flake = "github:alice/nixos";"#,
        ],
    );
}

fn kept_flake(origin: &str) -> Fork {
    let fork = Fork::with_origin(origin);
    fork.initialised(&[
        "--yes",
        "--host",
        "box",
        "--login",
        "bob",
        "--full-name",
        "Bob",
        "--email",
        "",
        "--keep-server",
        "--keep-pi",
    ]);
    fork
}

const SERVER: &str = "modules/hosts/example-server/default.nix";

#[test]
fn a_github_fork_becomes_the_flake_of_the_kept_examples() {
    let fork = kept_flake("git@github.com:bob/my-nixos.git");

    assert_lines(
        &fork,
        SERVER,
        &[
            r#"      "auto-upgrade" # .*"#,
            r#"    flake = "github:bob/my-nixos";"#,
        ],
    );
    assert_lines(
        &fork,
        "modules/hosts/example-pi/default.nix",
        &[r#"    # flake = "github:bob/my-nixos";"#],
    );
}

#[test]
fn another_https_forge_is_a_git_flake() {
    let fork = kept_flake("https://git.example.org/bob/nixos.git");

    assert_lines(
        &fork,
        SERVER,
        &[
            r#"      "auto-upgrade" # .*"#,
            r#"    flake = "git\+https://git\.example\.org/bob/nixos\.git";"#,
        ],
    );
}

#[test]
fn a_clone_of_upstream_comments_out_the_flake_and_auto_upgrade() {
    let fork = kept_flake("https://github.com/axelhamil/nixos-forge.git");

    assert_lines(
        &fork,
        SERVER,
        &[
            r#"      # "auto-upgrade" # .*"#,
            r#"    # flake = "github:alice/nixos";"#,
        ],
    );
}

#[test]
fn the_repo_of_a_github_fork_is_named_in_the_secret_command() {
    let fork = Fork::with_origin("https://github.com/bob/my-nixos");
    let printed = fork.initialised(&[
        "--yes",
        "--host",
        "box",
        "--login",
        "bob",
        "--full-name",
        "Bob",
        "--email",
        "",
    ]);

    assert!(
        printed.lines().any(|line| line
            .starts_with("  gh secret set DEPLOY_SIGNING_KEY --env deploy --repo bob/my-nixos < ")),
        "{printed}"
    );
}

#[test]
fn a_checkout_whose_path_is_not_utf8_is_initialised() {
    let fork = Fork::named(OsStr::from_bytes(b"fork-\xff"));

    fork.initialised(&LAPTOP);

    assert!(fork.exists("modules/hosts/my-box/default.nix"));
    assert!(
        fork.status()
            .contains(&"A  modules/hosts/my-box/default.nix".to_owned())
    );
}

#[test]
fn a_failure_after_the_key_exists_keeps_it_and_says_where_once() {
    let fork = Fork::new();
    fs::write(fork.root.join(".git/index.lock"), "").unwrap();

    let output = fork.init(&LAPTOP);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    let kept: Vec<&str> = stderr
        .lines()
        .filter(|line| line.starts_with("forge init: the deploy signing key is kept in "))
        .collect();
    assert_eq!(kept.len(), 1, "{stderr}");
    let file = kept[0]
        .trim_start_matches("forge init: the deploy signing key is kept in ")
        .split(", its public key")
        .next()
        .unwrap();
    assert!(Path::new(file).is_file(), "{file}");
    assert!(Path::new(file).starts_with(&fork.runtime));
}

#[test]
fn interactive_answers_are_asked_in_order_and_a_wrong_one_is_asked_again() {
    let fork = Fork::new();
    let typed = [
        "-bad", "box", "", "", "bob", "Bob", "", "UTC", "", "", "disk", "", "", "n", "y",
    ];

    let mut child = fork
        .command(Path::new(env!("CARGO_BIN_EXE_forge")))
        .arg("init")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        use std::io::Write as _;
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all((typed.join("\n") + "\n").as_bytes())
            .unwrap();
    }
    let output = child.wait_with_output().unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(
        stderr.contains("  a host name has 1 to 63 letters"),
        "{stderr}"
    );
    assert!(
        stderr.contains("Features (each one is described in modules/hosts/example/default.nix):")
    );
    assert!(
        stderr.contains("  [x] disk           LUKS and btrfs"),
        "{stderr}"
    );
    assert_lines(
        &fork,
        "modules/hosts/box/default.nix",
        &[
            r#"    profile = "desktop";.*"#,
            r#"    timeZone = "UTC";.*"#,
            r#"    keyboard = "us";.*"#,
            r#"      "disk" # .*"#,
            r#"      # "secure-boot" # .*"#,
            r#"      "hyprland" # .*"#,
        ],
    );
    assert_lines(
        &fork,
        "modules/users/bob/default.nix",
        &[r#"    # email = "bob@example\.org";.*"#],
    );
    assert!(fork.exists("modules/hosts/example-pi"));
    assert!(!fork.exists("modules/hosts/example-server"));
}

#[test]
fn input_that_ends_before_an_answer_names_the_flag_to_pass() {
    let fork = Fork::new();

    let output = fork.init(&["--host", "box"]);

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "forge init: no answer for System (x86_64-linux aarch64-linux): pass --system, or --yes for the defaults\n"
    );
    assert_eq!(fork.status(), Vec::<String>::new());
}
