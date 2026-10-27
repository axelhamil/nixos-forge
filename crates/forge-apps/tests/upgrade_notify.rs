use std::cell::RefCell;
use std::fs;

use forge::system::{Answer, Commands, Invocation};
use forge_apps::upgrade_notify::{self, Paths};
use tempfile::TempDir;

struct Desktop {
    running: String,
    daemon: bool,
    notifications: RefCell<Vec<Vec<String>>>,
}

impl Commands for Desktop {
    fn run(&self, invocation: &Invocation) -> Answer {
        if invocation.program == "notify-send" {
            self.notifications
                .borrow_mut()
                .push(invocation.args.clone());
            return if self.daemon {
                Answer::ok("")
            } else {
                Answer::failed("cannot connect")
            };
        }
        assert_eq!(invocation.program, "/fake/nixos-version");
        assert_eq!(invocation.args, ["--configuration-revision"]);
        Answer::ok(&format!("{}\n", self.running))
    }
}

struct Session {
    dir: TempDir,
    paths: Paths,
}

impl Session {
    fn new(state: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("state.json"), state).unwrap();
        let paths = Paths {
            state: dir.path().join("state.json"),
            nixos_version: "/fake/nixos-version".to_owned(),
            seen_directory: dir.path().join("state/upgrade-notify"),
        };
        Self { dir, paths }
    }

    fn run(&self, desktop: &Desktop) -> (u8, String) {
        let mut err = Vec::new();
        let code = upgrade_notify::run(desktop, &self.paths, &mut err);
        (code, String::from_utf8(err).unwrap())
    }

    fn seen(&self) -> Option<String> {
        fs::read_to_string(self.dir.path().join("state/upgrade-notify/seen")).ok()
    }
}

fn desktop(running: &str) -> Desktop {
    Desktop {
        running: running.to_owned(),
        daemon: true,
        notifications: RefCell::new(Vec::new()),
    }
}

fn state(outcome: &str, changes: &str) -> String {
    format!(
        r#"{{"time":"2026-01-01T04:00:00+00:00","outcome":"{outcome}","from":"a","to":"b","source":"s","message":"applies at next boot","changes":"{changes}"}}"#
    )
}

#[test]
fn an_upgrade_waiting_for_a_restart_is_notified_once_with_its_changes() {
    let session = Session::new(&state("upgraded", "Closure size: 10 -> 12"));
    let desktop = desktop("a");

    assert_eq!(session.run(&desktop), (0, String::new()));
    assert_eq!(session.run(&desktop), (0, String::new()));

    assert_eq!(
        *desktop.notifications.borrow(),
        [[
            "--app-name=auto-upgrade",
            "System updated",
            "Closure size: 10 -> 12\nRestart to use it."
        ]]
    );
    assert_eq!(
        session.seen().as_deref(),
        Some("2026-01-01T04:00:00+00:00 b\n")
    );
}

#[test]
fn a_running_upgrade_without_changes_shows_its_message_alone() {
    let session = Session::new(&state("upgraded", ""));
    let desktop = desktop("b");

    session.run(&desktop);

    assert_eq!(desktop.notifications.borrow()[0][2], "applies at next boot");
}

#[test]
fn nothing_is_notified_without_an_upgrade_or_a_readable_state() {
    for text in [state("up-to-date", "x"), "not json".to_owned()] {
        let session = Session::new(&text);
        let desktop = desktop("a");

        assert_eq!(session.run(&desktop), (0, String::new()));

        assert!(desktop.notifications.borrow().is_empty());
        assert_eq!(session.seen(), None);
    }
}

#[test]
fn an_unanswered_notification_is_retried_at_the_next_session() {
    let session = Session::new(&state("upgraded", "x"));
    let desktop = Desktop {
        daemon: false,
        ..desktop("a")
    };

    assert_eq!(
        session.run(&desktop),
        (
            0,
            "no notification daemon answered; the upgrade is announced at the next session\n"
                .to_owned()
        )
    );
    assert_eq!(session.seen(), None);
}
