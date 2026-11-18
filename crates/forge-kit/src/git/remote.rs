use crate::pattern::{Pattern, PatternInvalid};

pub const UPSTREAM_REPOSITORY: &str = "axelhamil/nixos-forge";
pub const GITHUB: &str = "https://github.com/";

pub(crate) static GITHUB_URL: Pattern =
    Pattern::new(r"^(https://github\.com/|ssh://git@github\.com/|git@github\.com:)([^/]+/[^/]+)$");

pub(crate) static PLAIN_HTTPS: Pattern = Pattern::new(r"^https://[^?#]+$");

fn trimmed(url: &str) -> &str {
    let url = url.strip_suffix('/').unwrap_or(url);
    url.strip_suffix(".git").unwrap_or(url)
}

pub fn github_repository(url: &str) -> Result<Option<String>, PatternInvalid> {
    let repository = GITHUB_URL
        .captures(trimmed(url))?
        .and_then(|captures| captures.get(2))
        .map(|repository| repository.as_str().to_owned());
    Ok(repository)
}

pub fn repository_of(origin: &str) -> Result<String, PatternInvalid> {
    if let Some(repository) = github_repository(origin)? {
        return Ok(repository);
    }
    let trimmed = trimmed(origin);
    let after_last = |text: &str| -> String {
        text.rsplit_once(['/', ':'])
            .map_or(text, |(_, last)| last)
            .to_owned()
    };
    let name = after_last(trimmed);
    let rest = trimmed
        .rsplit_once(['/', ':'])
        .map_or(trimmed, |(rest, _)| rest);
    Ok(format!("{}/{name}", after_last(rest)))
}

pub fn fork_flake(origin: Option<&str>, upstream: &str) -> Result<String, PatternInvalid> {
    let Some(origin) = origin else {
        return Ok(String::new());
    };
    if let Some(repository) = github_repository(origin)? {
        if is_upstream(&repository, upstream) {
            return Ok(String::new());
        }
        return Ok(format!("github:{repository}"));
    }
    if PLAIN_HTTPS.is_match(origin)? {
        return Ok(format!("git+{origin}"));
    }
    Ok(String::new())
}

#[must_use]
pub fn is_upstream(repository: &str, upstream: &str) -> bool {
    repository.eq_ignore_ascii_case(upstream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_github_form_names_its_repository() {
        for url in [
            "https://github.com/bob/my-nixos",
            "https://github.com/bob/my-nixos.git",
            "https://github.com/bob/my-nixos/",
            "ssh://git@github.com/bob/my-nixos.git",
            "git@github.com:bob/my-nixos.git",
        ] {
            assert_eq!(
                github_repository(url).unwrap().as_deref(),
                Some("bob/my-nixos"),
                "{url}"
            );
        }
    }

    #[test]
    fn other_hosts_and_deeper_paths_are_not_github_repositories() {
        for url in [
            "https://git.example.org/bob/nixos.git",
            "https://github.com/bob/nixos/tree/main",
            "/srv/git/nixos",
        ] {
            assert_eq!(github_repository(url).unwrap(), None, "{url}");
        }
    }

    #[test]
    fn the_repository_of_origin_is_its_last_two_segments() {
        assert_eq!(
            repository_of("git@github.com:bob/nixos.git").unwrap(),
            "bob/nixos"
        );
        assert_eq!(
            repository_of("https://github.com/bob/nixos/").unwrap(),
            "bob/nixos"
        );
        assert_eq!(
            repository_of("file:///tmp/run/synced-origin.git").unwrap(),
            "run/synced-origin"
        );
        assert_eq!(
            repository_of("git@git.example.org:team/nixos.git").unwrap(),
            "team/nixos"
        );
    }

    #[test]
    fn upstream_is_matched_whatever_the_case() {
        assert!(is_upstream("AxelHamil/NixOS-Forge", UPSTREAM_REPOSITORY));
        assert!(!is_upstream("bob/nixos-forge", UPSTREAM_REPOSITORY));
    }
}
