use fancy_regex::Regex;

pub const UPSTREAM_REPOSITORY: &str = "axelhamil/nixos-forge";

const GITHUB_URL: &str =
    r"^(https://github\.com/|ssh://git@github\.com/|git@github\.com:)([^/]+/[^/]+)$";

#[must_use]
pub fn github_repository(url: &str) -> Option<String> {
    let trimmed = url.strip_suffix('/').unwrap_or(url);
    let trimmed = trimmed.strip_suffix(".git").unwrap_or(trimmed);
    let captures = Regex::new(GITHUB_URL).ok()?.captures(trimmed).ok()??;
    Some(captures.get(2)?.as_str().to_owned())
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
    fn upstream_is_matched_whatever_the_case() {
        assert!(is_upstream("AxelHamil/NixOS-Forge", UPSTREAM_REPOSITORY));
        assert!(!is_upstream("bob/nixos-forge", UPSTREAM_REPOSITORY));
    }
}
