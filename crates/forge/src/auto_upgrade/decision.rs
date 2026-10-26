use std::path::Path;

use super::sources::Found;
use super::state::Outcome;
use crate::system::Git;

pub const WAITING: &str = "CI has not validated a version for this host yet; if this lasts, CI has not promoted it: check that the bot App is set up (docs/how-to/ci.md) and that the host's checks pass";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revisions<'a> {
    pub running: &'a str,
    pub next: &'a str,
    pub failed: Option<&'a str>,
    pub to: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Uncommitted,
    RunningIsCurrent,
    RolledBack,
    InstalledForNextBoot,
    NotContained,
    Upgrade,
}

pub trait History {
    fn carried(&self, commit: &str) -> Option<String>;
    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> bool;
}

#[must_use]
pub fn settle<'a>(
    running: &'a str,
    next: &'a str,
    did_not_start: impl FnOnce() -> bool,
) -> (&'a str, Option<&'a str>) {
    let next = if next.is_empty() { running } else { next };
    if next != running && did_not_start() {
        return (running, Some(next));
    }
    (next, None)
}

#[must_use]
pub fn decide(revisions: &Revisions, history: &dyn History) -> Decision {
    let Revisions {
        running,
        next,
        failed,
        to,
    } = *revisions;

    let uncommitted = |revision: &str| revision.is_empty() || revision.ends_with("-dirty");
    if uncommitted(running) || uncommitted(next) {
        return Decision::Uncommitted;
    }
    if next == to && running == to {
        return Decision::RunningIsCurrent;
    }
    if failed.is_some_and(|failed| !failed.is_empty() && failed == to) {
        return Decision::RolledBack;
    }
    if next == to {
        return Decision::InstalledForNextBoot;
    }

    let Some(carried_to) = history.carried(to) else {
        return Decision::NotContained;
    };
    let contained = |revision: &str| {
        history
            .carried(revision)
            .is_some_and(|carried| history.is_ancestor(&carried, &carried_to))
    };
    if !contained(running) || !contained(next) {
        return Decision::NotContained;
    }
    Decision::Upgrade
}

#[must_use]
pub fn untrusted(checkout: &Path) -> String {
    format!(
        "the checkout {} must be owned by root or an admin (wheel), writable by no one outside root and wheel, with a plain .git folder, for auto-upgrade to trust its origin remote",
        checkout.display()
    )
}

#[must_use]
pub fn cleartext(checkout: &Path) -> String {
    format!(
        "the origin remote of {} uses http:// or git://, which is not encrypted: anyone on the network path could choose what this machine builds; set it to an https or ssh URL",
        checkout.display()
    )
}

#[must_use]
pub fn not_found(found: Found, branch: &str) -> (Outcome, String) {
    match found {
        Found::Refused(refusals) => (
            Outcome::Refused,
            format!(
                "no source serves a {branch} commit signed for this host by a key of update.signers, so this machine stays on its version ({}): check the DEPLOY_SIGNING_KEY secret of the fork (docs/how-to/ci.md)",
                refusals.join("; ")
            ),
        ),
        Found::Waiting => (Outcome::Waiting, WAITING.to_owned()),
        Found::Unreachable | Found::Source { .. } => (
            Outcome::Unreachable,
            "no source reachable: this machine stays on its version".to_owned(),
        ),
    }
}

#[must_use]
pub fn verdict(
    decision: Decision,
    branch: &str,
    short: impl FnOnce() -> String,
) -> Option<(Outcome, String)> {
    Some(match decision {
        Decision::Uncommitted => (
            Outcome::Refused,
            "this machine runs uncommitted changes: commit and push them, CI will validate them"
                .to_owned(),
        ),
        Decision::RunningIsCurrent => {
            (Outcome::UpToDate, format!("{branch} is the running commit"))
        }
        Decision::RolledBack => (
            Outcome::RolledBack,
            format!(
                "{} did not start and the machine went back to the previous generation: fix it and push, CI will validate the fix",
                short()
            ),
        ),
        Decision::InstalledForNextBoot => (
            Outcome::UpToDate,
            format!("{branch} is installed and applies at next boot"),
        ),
        Decision::NotContained => (
            Outcome::Refused,
            format!(
                "{branch} does not contain the main commit this machine runs: push it, or the pointer was moved back"
            ),
        ),
        Decision::Upgrade => return None,
    })
}

pub struct GitHistory<'a> {
    pub git: &'a Git<'a>,
    pub host: &'a str,
}

impl History for GitHistory<'_> {
    fn carried(&self, commit: &str) -> Option<String> {
        let subject = self.git.quiet(&["log", "-1", "--format=%s", commit]);
        if !subject.success {
            return None;
        }
        let promotion = format!("deploy {}: checks passed for ", self.host);
        if !subject.line().starts_with(&promotion) {
            return Some(commit.to_owned());
        }
        let parents = self.git.loud(&["rev-list", "--parents", "-n", "1", commit]);
        parents.stdout.split_whitespace().last().map(str::to_owned)
    }

    fn is_ancestor(&self, ancestor: &str, descendant: &str) -> bool {
        self.git
            .quiet(&["merge-base", "--is-ancestor", ancestor, descendant])
            .success
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[derive(Default)]
    struct Graph {
        carried: HashMap<&'static str, &'static str>,
        ancestors: Vec<(&'static str, &'static str)>,
    }

    impl Graph {
        fn line(commits: &[&'static str]) -> Self {
            let mut graph = Self::default();
            for (index, &commit) in commits.iter().enumerate() {
                graph.carried.insert(commit, commit);
                for &older in &commits[..=index] {
                    graph.ancestors.push((older, commit));
                }
            }
            graph
        }
    }

    impl History for Graph {
        fn carried(&self, commit: &str) -> Option<String> {
            self.carried.get(commit).map(|&carried| carried.to_owned())
        }

        fn is_ancestor(&self, ancestor: &str, descendant: &str) -> bool {
            self.ancestors.contains(&(ancestor, descendant))
        }
    }

    fn decide_on(
        running: &str,
        next: &str,
        failed: Option<&str>,
        to: &str,
        graph: &Graph,
    ) -> Decision {
        decide(
            &Revisions {
                running,
                next,
                failed,
                to,
            },
            graph,
        )
    }

    #[test]
    fn a_newer_commit_containing_the_running_one_is_applied() {
        let graph = Graph::line(&["a", "b"]);

        assert_eq!(decide_on("a", "a", None, "b", &graph), Decision::Upgrade);
    }

    #[test]
    fn uncommitted_or_unknown_revisions_are_refused() {
        let graph = Graph::line(&["a", "b"]);

        assert_eq!(
            decide_on("a-dirty", "a", None, "b", &graph),
            Decision::Uncommitted
        );
        assert_eq!(
            decide_on("a", "a-dirty", None, "b", &graph),
            Decision::Uncommitted
        );
        assert_eq!(decide_on("", "a", None, "b", &graph), Decision::Uncommitted);
    }

    #[test]
    fn the_running_commit_is_up_to_date() {
        let graph = Graph::line(&["b"]);

        assert_eq!(
            decide_on("b", "b", None, "b", &graph),
            Decision::RunningIsCurrent
        );
    }

    #[test]
    fn a_commit_installed_for_next_boot_is_up_to_date() {
        let graph = Graph::line(&["a", "b"]);

        assert_eq!(
            decide_on("a", "b", None, "b", &graph),
            Decision::InstalledForNextBoot
        );
    }

    #[test]
    fn the_commit_that_did_not_start_is_not_installed_again() {
        let graph = Graph::line(&["a", "b"]);

        assert_eq!(
            decide_on("a", "a", Some("b"), "b", &graph),
            Decision::RolledBack
        );
    }

    #[test]
    fn a_fix_replaces_the_commit_that_did_not_start() {
        let mut graph = Graph::line(&["a", "d"]);
        graph.carried.insert("b", "b");
        graph.ancestors.push(("a", "b"));

        assert_eq!(
            decide_on("a", "a", Some("b"), "d", &graph),
            Decision::Upgrade
        );
    }

    #[test]
    fn a_branch_that_does_not_contain_the_running_commit_is_refused() {
        let mut graph = Graph::line(&["a", "b"]);
        graph.carried.insert("c", "c");

        assert_eq!(
            decide_on("b", "b", None, "c", &graph),
            Decision::NotContained
        );
        assert_eq!(
            decide_on("b", "b", None, "a", &graph),
            Decision::NotContained
        );
    }

    #[test]
    fn a_promotion_commit_counts_as_the_main_commit_it_carries() {
        let mut graph = Graph::line(&["b", "c"]);
        graph.carried.insert("promote-b", "b");
        graph.carried.insert("promote-c", "c");

        assert_eq!(
            decide_on("promote-b", "promote-b", None, "promote-c", &graph),
            Decision::Upgrade
        );
        assert_eq!(
            decide_on("promote-c", "promote-c", None, "promote-b", &graph),
            Decision::NotContained
        );
    }

    #[test]
    fn a_next_generation_that_did_not_start_falls_back_to_the_running_one() {
        assert_eq!(settle("a", "b", || true), ("a", Some("b")));
        assert_eq!(settle("a", "b", || false), ("b", None));
        assert_eq!(settle("a", "", || unreachable!()), ("a", None));
        assert_eq!(settle("a", "a", || unreachable!()), ("a", None));
    }
}
