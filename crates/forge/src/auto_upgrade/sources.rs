use std::time::Duration;

use fancy_regex::Regex;

use super::Error;
use super::signature::Signers;
use crate::system::Git;

const SOURCE_TIMEOUT: Duration = Duration::from_secs(60);

const SSH_URL: &str = r"^ssh://([^@/]+@)?([^/:]+)(:[0-9]+)?/(.+)$";
const SCP_LIKE: &str = r"^([^@/:]+@)?([^/:]+):(.+)$";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    Source { source: String, to: String },
    Refused(Vec<String>),
    Waiting,
    Unreachable,
}

#[must_use]
pub fn over_https(url: &str) -> String {
    if let Some([host, path]) = groups(SSH_URL, url, [2, 4]) {
        return format!("https://{host}/{path}");
    }
    if !url.contains("://")
        && let Some([host, path]) = groups(SCP_LIKE, url, [2, 3])
    {
        return format!("https://{host}/{}", path.strip_prefix('/').unwrap_or(path));
    }
    url.to_owned()
}

fn groups<'a>(pattern: &str, url: &'a str, indexes: [usize; 2]) -> Option<[&'a str; 2]> {
    let captures = Regex::new(pattern).ok()?.captures(url).ok()??;
    let group = |index: usize| captures.get(index).map(|found| found.as_str());
    Some([group(indexes[0])?, group(indexes[1])?])
}

pub fn find(
    git: &Git,
    sources: &[String],
    branch: &str,
    signers: Option<&Signers>,
) -> Result<Found, Error> {
    let head = format!("refs/heads/{branch}");
    let mut answered = false;
    let mut refusals = Vec::new();

    for (index, candidate) in sources.iter().enumerate() {
        let listed = git.remote(
            &[
                "ls-remote",
                "--quiet",
                "--exit-code",
                "--end-of-options",
                candidate,
                &head,
            ],
            SOURCE_TIMEOUT,
        );
        if listed.code == Some(2) {
            answered = true;
            continue;
        }
        if !listed.success {
            continue;
        }

        let candidate_ref = format!("refs/candidates/{index}");
        let refspec = format!("+{head}:{candidate_ref}");
        let fetched = git.fetch(
            &[
                "--quiet",
                "--no-tags",
                "--end-of-options",
                candidate,
                &refspec,
            ],
            SOURCE_TIMEOUT,
        );
        if !fetched.success {
            continue;
        }

        let parsed = git.loud(&["rev-parse", &candidate_ref]);
        if !parsed.success {
            return Err(Error::Git(format!("rev-parse {candidate_ref}")));
        }
        let fetched = parsed.line().to_owned();

        if let Some(reason) = signers.and_then(|signers| signers.refusal(&fetched)) {
            refusals.push(format!("{candidate}: {reason}"));
            continue;
        }
        return Ok(Found::Source {
            source: candidate.clone(),
            to: fetched,
        });
    }

    if !refusals.is_empty() {
        return Ok(Found::Refused(refusals));
    }
    if answered {
        return Ok(Found::Waiting);
    }
    Ok(Found::Unreachable)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::Path;

    use super::*;
    use crate::system::{Answer, Commands, Invocation};

    #[test]
    fn ssh_and_scp_like_urls_are_fetched_over_https() {
        let cases = [
            (
                "ssh://git@github.com/alice/nixos.git",
                "https://github.com/alice/nixos.git",
            ),
            (
                "ssh://github.com:22/alice/nixos.git",
                "https://github.com/alice/nixos.git",
            ),
            (
                "git@github.com:alice/nixos.git",
                "https://github.com/alice/nixos.git",
            ),
            ("nas:/srv/mirror.git", "https://nas/srv/mirror.git"),
            (
                "https://codeberg.org/alice/nixos.git",
                "https://codeberg.org/alice/nixos.git",
            ),
            ("file:///srv/mirror.git", "file:///srv/mirror.git"),
            ("/srv/mirror.git", "/srv/mirror.git"),
            ("ssh://host:port/x", "ssh://host:port/x"),
        ];

        for (url, expected) in cases {
            assert_eq!(over_https(url), expected, "{url}");
        }
    }

    #[derive(Default)]
    struct Remote {
        answers: HashMap<String, Answer>,
        ran: RefCell<Vec<String>>,
    }

    impl Remote {
        fn answer(mut self, command_line: &str, answer: Answer) -> Self {
            self.answers.insert(command_line.to_owned(), answer);
            self
        }

        fn listed(self, url: &str, code: i32) -> Self {
            self.answer(
                &format!(
                    "git ls-remote --quiet --exit-code --end-of-options {url} refs/heads/deploy/machine"
                ),
                Answer::exited(code),
            )
        }

        fn fetched(self, url: &str, index: usize, commit: &str) -> Self {
            self.answer(
                &format!("git -C /repo fetch --quiet --no-tags --end-of-options {url} +refs/heads/deploy/machine:refs/candidates/{index}"),
                Answer::ok(""),
            )
            .answer(
                &format!("git -C /repo rev-parse refs/candidates/{index}"),
                Answer::ok(&format!("{commit}\n")),
            )
        }

        fn signed(self, commit: &str, verified: bool, subject: &str) -> Self {
            let signers = "git -C /repo -c gpg.format=ssh -c gpg.ssh.allowedSignersFile=/signers -c gpg.openpgp.program=false -c gpg.x509.program=false";
            self.answer(
                &format!("{signers} verify-commit {commit}"),
                Answer::exited(i32::from(!verified)),
            )
            .answer(
                &format!("git -C /repo log -1 --format=%s {commit}"),
                Answer::ok(&format!("{subject}\n")),
            )
            .answer(
                &format!("{signers} log -1 --format=%G? {commit}"),
                Answer::ok("N\n"),
            )
            .answer(
                &format!("git -C /repo cat-file commit {commit}"),
                Answer::ok("tree t\n\nsubject\n"),
            )
        }

        fn find(&self, sources: &[&str], signed: bool) -> Found {
            let git = Git::new(self, "/repo");
            let signers = Signers::new(&git, "machine", Path::new("/signers"));
            let sources: Vec<String> = sources.iter().map(|&url| url.to_owned()).collect();
            find(&git, &sources, "deploy/machine", signed.then_some(&signers)).unwrap()
        }
    }

    impl Commands for Remote {
        fn run(&self, invocation: &Invocation) -> Answer {
            let line = invocation.command_line();
            self.ran.borrow_mut().push(line.clone());
            self.answers
                .get(&line)
                .cloned()
                .unwrap_or_else(|| Answer::exited(128))
        }
    }

    #[test]
    fn a_source_without_the_branch_means_ci_has_not_promoted_yet() {
        let remote = Remote::default().listed("https://a", 2);

        assert_eq!(remote.find(&["https://a"], false), Found::Waiting);
    }

    #[test]
    fn no_source_answering_is_unreachable() {
        let remote = Remote::default().listed("https://a", 128);

        assert_eq!(
            remote.find(&["https://a", "https://b"], false),
            Found::Unreachable
        );
    }

    #[test]
    fn a_failed_fetch_falls_through_to_the_next_source() {
        let remote = Remote::default()
            .listed("https://a", 0)
            .listed("https://b", 0)
            .fetched("https://b", 1, "beef");

        assert_eq!(
            remote.find(&["https://a", "https://b"], false),
            Found::Source {
                source: "https://b".to_owned(),
                to: "beef".to_owned()
            }
        );
    }

    #[test]
    fn the_first_source_serving_the_branch_wins() {
        let remote = Remote::default()
            .listed("https://a", 0)
            .fetched("https://a", 0, "cafe");

        assert_eq!(
            remote.find(&["https://a", "https://b"], false),
            Found::Source {
                source: "https://a".to_owned(),
                to: "cafe".to_owned()
            }
        );
        assert!(
            remote
                .ran
                .borrow()
                .iter()
                .all(|line| !line.contains("https://b"))
        );
    }

    #[test]
    fn a_signature_refusal_is_collected_and_the_next_source_tried() {
        let remote = Remote::default()
            .listed("https://a", 0)
            .fetched("https://a", 0, "bad")
            .signed("bad", false, "deploy machine: checks passed for x")
            .listed("https://b", 0)
            .fetched("https://b", 1, "good")
            .signed("good", true, "deploy machine: checks passed for x");

        assert_eq!(
            remote.find(&["https://a", "https://b"], true),
            Found::Source {
                source: "https://b".to_owned(),
                to: "good".to_owned()
            }
        );
    }

    #[test]
    fn refusals_win_over_a_source_without_the_branch() {
        let remote = Remote::default()
            .listed("https://a", 2)
            .listed("https://b", 0)
            .fetched("https://b", 1, "other")
            .signed("other", true, "deploy laptop: checks passed for x");

        assert_eq!(
            remote.find(&["https://a", "https://b"], true),
            Found::Refused(vec!["https://b: signed for another host".to_owned()])
        );
    }

    #[test]
    fn each_signature_failure_has_its_reason() {
        let signers = "git -C /repo -c gpg.format=ssh -c gpg.ssh.allowedSignersFile=/signers -c gpg.openpgp.program=false -c gpg.x509.program=false";
        let cases = [
            ("N", "tree t\n\nsubject\n", "unsigned"),
            (
                "N",
                "tree t\ngpgsig -----BEGIN PGP SIGNATURE-----\n\nsubject\n",
                "not signed with an SSH key",
            ),
            ("B", "", "signature invalid"),
            ("E", "", "signed by a key not in update.signers"),
        ];

        for (status, raw, reason) in cases {
            let remote = Remote::default()
                .answer(
                    &format!("{signers} log -1 --format=%G? c"),
                    Answer::ok(&format!("{status}\n")),
                )
                .answer("git -C /repo cat-file commit c", Answer::ok(raw));
            let git = Git::new(&remote, "/repo");
            let signers = Signers::new(&git, "machine", Path::new("/signers"));

            assert_eq!(signers.refusal("c"), Some(reason), "{status}");
        }
    }
}
