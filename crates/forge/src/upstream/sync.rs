use std::io;
use std::path::Path;
use std::time::Duration;

use super::notes::{
    breaking_lines, changelog_lines, latest_tag, matches, merged_release, notes, pull_request_body,
    repository_of, source_url, starter,
};
use super::{Context, Options, Ports};
use crate::system::{Answer, Git, Invocation};

const BASE: &str = "origin/main";
const NETWORK_TIMEOUT: Duration = Duration::from_secs(600);
const YES: &str = r"^[Yy]([Ee][Ss])?$";
const BOT_NAME: &str = "github-actions[bot]";
const BOT_EMAIL: &str = "41898282+github-actions[bot]@users.noreply.github.com";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct Failed(String);

fn failed(message: impl Into<String>) -> Failed {
    Failed(message.into())
}

struct Release {
    url: String,
    tag: String,
    branch: String,
    title: String,
    notes: String,
}

pub fn sync(options: &Options, context: &Context, ports: &mut Ports) -> Result<(), Failed> {
    let git = Git::new(ports.commands, ".");
    if !git.quiet(&["rev-parse", "--git-dir"]).success {
        return Err(failed("run it inside your fork's checkout"));
    }

    let source = options.source.clone().unwrap_or_else(|| {
        let upstream = git.quiet(&["remote", "get-url", "upstream"]);
        if upstream.success {
            upstream.line().to_owned()
        } else {
            starter()
        }
    });
    let url = source_url(&source);

    let tags = network(&git, &["ls-remote", "--tags", &url, "refs/tags/v[0-9]*"]);
    if !tags.success {
        return Err(failed(format!("cannot read the tags of {url}")));
    }
    let Some(tag) = latest_tag(&tags.stdout) else {
        say(ports.out, &format!("{url} has no release yet."));
        return Ok(());
    };

    let tag_ref = format!("+refs/tags/{tag}:refs/tags/{tag}");
    if !git
        .attached(&["fetch", "-q", "--no-tags", &url, &tag_ref])
        .success
    {
        return Err(failed(format!("cannot fetch {tag} from {url}")));
    }
    let main_ref = "+refs/heads/main:refs/remotes/origin/main";
    if !git
        .attached(&["fetch", "-q", "--no-tags", "origin", main_ref])
        .success
    {
        return Err(failed("cannot fetch the main branch of origin"));
    }

    if git
        .quiet(&["merge-base", "--is-ancestor", &tag, BASE])
        .success
    {
        say(ports.out, &format!("Already up to date with {tag}."));
        return Ok(());
    }

    let release = Release {
        notes: release_notes(&git, &tag),
        branch: format!("chore/sync-upstream-{tag}"),
        title: format!("chore: sync with upstream {tag}"),
        url,
        tag,
    };
    say(
        ports.out,
        &format!(
            "{} {} is not merged in the main branch of this fork yet.\n\n{}\n",
            release.url, release.tag, release.notes
        ),
    );

    if !ready(&git, &release, options.yes, ports)? {
        return Ok(());
    }
    prepare(&git, &release, context, ports)
}

fn release_notes(git: &Git, tag: &str) -> String {
    let log = git.quiet(&["log", "--format=%B", &format!("{BASE}..{tag}")]);
    let diff = git.quiet(&["diff", &format!("{BASE}...{tag}"), "--", "CHANGELOG.md"]);
    notes(&breaking_lines(&log.stdout), &changelog_lines(&diff.stdout))
}

fn ready(git: &Git, release: &Release, yes: bool, ports: &mut Ports) -> Result<bool, Failed> {
    let branch = &release.branch;
    let on_origin = git.run(
        &git.invocation(&["ls-remote", "--exit-code", "--heads", "origin", branch])
            .timeout(NETWORK_TIMEOUT)
            .loud(),
    );
    if on_origin.success {
        say(
            ports.out,
            &format!("{branch} is already on origin: its pull request is waiting for your review."),
        );
        return Ok(false);
    }

    let local = format!("refs/heads/{branch}");
    if git.quiet(&["rev-parse", "--verify", "-q", &local]).success {
        return Err(failed(format!(
            "{branch} already exists here: push it, or delete it with git branch -D {branch} to prepare it again"
        )));
    }

    if !git.quiet(&["status", "--porcelain"]).stdout.is_empty() {
        return Err(failed("commit or stash your changes first"));
    }

    if yes || accepted(branch, ports) {
        return Ok(true);
    }
    say(ports.out, "Nothing changed.");
    Ok(false)
}

fn accepted(branch: &str, ports: &mut Ports) -> bool {
    let _ = write!(ports.err, "Prepare branch {branch}? [y/N] ");
    let _ = ports.err.flush();
    let answer = ports.terminal.line("").unwrap_or_default();
    matches(YES, &answer)
}

fn prepare(
    git: &Git,
    release: &Release,
    context: &Context,
    ports: &mut Ports,
) -> Result<(), Failed> {
    let Release {
        tag, branch, title, ..
    } = release;

    if context.ci {
        let named = git.loud(&["config", "user.name", BOT_NAME]).success
            && git.loud(&["config", "user.email", BOT_EMAIL]).success;
        if !named {
            return Err(failed(format!("cannot set the git identity of {BOT_NAME}")));
        }
    }

    let start = current_position(git)?;
    if !git.loud(&["switch", "-q", "-c", branch, BASE]).success {
        return Err(failed(format!("cannot create {branch} from {BASE}")));
    }
    let give_up = |message: String| abandon(git, &start, branch, message);

    let merged = git.quiet(&["merge", "--no-ff", "--no-edit", "-m", title, tag]);
    if !merged.success {
        let conflicts = git.quiet(&["diff", "--name-only", "--diff-filter=U"]);
        let conflicts = conflicts.stdout.lines().collect::<Vec<_>>().join(" ");
        if conflicts.is_empty() {
            return Err(give_up(format!(
                "cannot merge {tag} into {BASE}: {}",
                combined(&merged)
            )));
        }
        return Err(give_up(format!(
            "{tag} conflicts with the main branch of this fork in: {conflicts}. Merge it by hand: git switch -c {branch} {BASE} && git merge --no-ff {tag}, resolve the conflicts and commit, then git push -u origin {branch} and open its pull request against main"
        )));
    }

    let origin = git.quiet(&["remote", "get-url", "origin"]);
    if !origin.success {
        return Err(give_up("cannot read the URL of origin".to_owned()));
    }
    let repository = repository_of(origin.line());

    if !git
        .attached(&["push", "-q", "-u", "origin", branch])
        .success
    {
        return Err(give_up(format!("cannot push {branch} to origin")));
    }

    let body_file = write_body(git, release, ports)?;
    open_pull_request(context, &repository, release, &body_file, ports)
}

fn current_position(git: &Git) -> Result<String, Failed> {
    let branch = git.quiet(&["symbolic-ref", "-q", "--short", "HEAD"]);
    if branch.success {
        return Ok(branch.line().to_owned());
    }
    let commit = git.quiet(&["rev-parse", "HEAD"]);
    if commit.success {
        return Ok(commit.line().to_owned());
    }
    Err(failed("cannot tell which commit HEAD is on"))
}

fn abandon(git: &Git, start: &str, branch: &str, message: String) -> Failed {
    if git
        .quiet(&["rev-parse", "-q", "--verify", "MERGE_HEAD"])
        .success
    {
        let _ = git.loud(&["merge", "--abort"]);
    }
    let start_branch = format!("refs/heads/{start}");
    if git
        .quiet(&["show-ref", "-q", "--verify", &start_branch])
        .success
    {
        let _ = git.loud(&["switch", "-q", start]);
    } else {
        let _ = git.loud(&["switch", "-q", "--detach", start]);
    }
    let _ = git.loud(&["branch", "-q", "-D", branch]);
    Failed(message)
}

fn combined(answer: &Answer) -> String {
    [answer.stdout.trim_end(), answer.stderr.trim_end()]
        .into_iter()
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn write_body(git: &Git, release: &Release, ports: &Ports) -> Result<String, Failed> {
    let Release {
        url, tag, notes, ..
    } = release;
    let cannot_write = || failed(format!("cannot write the pull request body of {tag}"));

    let git_dir = git.quiet(&["rev-parse", "--absolute-git-dir"]);
    if !git_dir.success {
        return Err(cannot_write());
    }
    let body_file = format!("{}/upstream-{tag}.md", git_dir.line());
    let commits = git.quiet(&[
        "log",
        "--oneline",
        "--no-decorate",
        &format!("{BASE}..{tag}"),
    ]);
    let body = pull_request_body(&merged_release(url, tag), &commits.stdout, notes);
    ports
        .storage
        .write(Path::new(&body_file), body.as_bytes())
        .map_err(|_| cannot_write())?;
    Ok(body_file)
}

fn open_pull_request(
    context: &Context,
    repository: &str,
    release: &Release,
    body_file: &str,
    ports: &mut Ports,
) -> Result<(), Failed> {
    let Release { branch, title, .. } = release;
    let logged_in = context.gh.as_deref().filter(|gh| {
        ports
            .commands
            .run(&Invocation::new(gh, &["auth", "status"]).discard_output())
            .success
    });

    let Some(gh) = logged_in else {
        say(
            ports.out,
            &format!(
                "{branch} is on origin. Open its pull request with:\n\n  gh pr create --repo {repository} --base main --head {branch} --title \"{title}\" --body-file {body_file}"
            ),
        );
        return Ok(());
    };

    let created = ports.commands.run(
        &Invocation::new(
            gh,
            &[
                "pr",
                "create",
                "--repo",
                repository,
                "--base",
                "main",
                "--head",
                branch,
                "--title",
                title,
                "--body-file",
                body_file,
            ],
        )
        .attached(),
    );
    if created.success {
        return Ok(());
    }
    Err(failed(format!(
        "gh pr create failed: {branch} is on origin, open its pull request by hand"
    )))
}

fn network(git: &Git, args: &[&str]) -> Answer {
    git.run(&git.invocation(args).timeout(NETWORK_TIMEOUT).loud())
}

fn say(out: &mut dyn io::Write, text: &str) {
    let _ = writeln!(out, "{text}");
}
