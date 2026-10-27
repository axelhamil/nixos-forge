use fancy_regex::Regex;

pub const UPSTREAM_REPOSITORY: &str = "axelhamil/nixos-forge";
pub const GITHUB: &str = "https://github.com/";

const GITHUB_URL: &str =
    r"^(https://github\.com/|ssh://git@github\.com/|git@github\.com:)([^/]+/[^/]+)$";

const PLAIN_HTTPS: &str = r"^https://[^?#]+$";

fn trimmed(url: &str) -> &str {
    let url = url.strip_suffix('/').unwrap_or(url);
    url.strip_suffix(".git").unwrap_or(url)
}

#[must_use]
pub fn github_repository(url: &str) -> Option<String> {
    let captures = Regex::new(GITHUB_URL).ok()?.captures(trimmed(url)).ok()??;
    Some(captures.get(2)?.as_str().to_owned())
}

#[must_use]
pub fn repository_of(origin: &str) -> String {
    if let Some(repository) = github_repository(origin) {
        return repository;
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
    format!("{}/{name}", after_last(rest))
}

#[must_use]
pub fn fork_flake(origin: Option<&str>, upstream: &str) -> String {
    let Some(origin) = origin else {
        return String::new();
    };
    if let Some(repository) = github_repository(origin) {
        if is_upstream(&repository, upstream) {
            return String::new();
        }
        return format!("github:{repository}");
    }
    let plain_https =
        Regex::new(PLAIN_HTTPS).is_ok_and(|pattern| pattern.is_match(origin).unwrap_or(false));
    if plain_https {
        return format!("git+{origin}");
    }
    String::new()
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
                github_repository(url).as_deref(),
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
            assert_eq!(github_repository(url), None, "{url}");
        }
    }

    #[test]
    fn the_repository_of_origin_is_its_last_two_segments() {
        assert_eq!(repository_of("git@github.com:bob/nixos.git"), "bob/nixos");
        assert_eq!(repository_of("https://github.com/bob/nixos/"), "bob/nixos");
        assert_eq!(
            repository_of("file:///tmp/run/synced-origin.git"),
            "run/synced-origin"
        );
        assert_eq!(
            repository_of("git@git.example.org:team/nixos.git"),
            "team/nixos"
        );
    }

    #[test]
    fn upstream_is_matched_whatever_the_case() {
        assert!(is_upstream("AxelHamil/NixOS-Forge", UPSTREAM_REPOSITORY));
        assert!(!is_upstream("bob/nixos-forge", UPSTREAM_REPOSITORY));
    }
}
