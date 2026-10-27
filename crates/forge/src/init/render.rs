use std::collections::HashSet;

use fancy_regex::Regex;

use super::answers::{Answers, EXAMPLE_PERSON};

const KEYWORDS: [&str; 10] = [
    "assert", "else", "if", "in", "inherit", "let", "or", "rec", "then", "with",
];

#[must_use]
pub fn nix_string(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace("${", "\\${")
}

#[must_use]
pub fn nix_attribute(name: &str) -> String {
    let bare = Regex::new(r"^[A-Za-z_][A-Za-z0-9_-]*$")
        .is_ok_and(|pattern| pattern.is_match(name).unwrap_or(false));
    if bare && !KEYWORDS.contains(&name) {
        name.to_owned()
    } else {
        format!("\"{name}\"")
    }
}

fn set_string(line: &str, value: &str) -> String {
    let Some(start) = line.find('"') else {
        return line.to_owned();
    };
    let (head, rest) = line.split_at(start + 1);
    let tail = rest.find('"').map_or(rest, |end| &rest[end..]);
    format!("{head}{value}{tail}")
}

fn lines(text: &str) -> impl Iterator<Item = &str> {
    text.split_inclusive('\n')
        .map(|line| line.strip_suffix('\n').unwrap_or(line))
}

fn joined(lines: impl IntoIterator<Item = String>) -> String {
    lines.into_iter().fold(String::new(), |mut text, line| {
        text.push_str(&line);
        text.push('\n');
        text
    })
}

fn regex(pattern: &str) -> Option<Regex> {
    Regex::new(pattern).ok()
}

fn captures<'t>(pattern: Option<&Regex>, line: &'t str) -> Option<Vec<&'t str>> {
    let captures = pattern?.captures(line).ok()??;
    Some(
        captures
            .iter()
            .map(|group| group.map_or("", |group| group.as_str()))
            .collect(),
    )
}

fn trust_signer(lines: Vec<String>, signer: &str) -> Vec<String> {
    let signers = regex(r#"^    # signers = \[ "[^"]*" \];(.*)$"#);
    let mut trusted = Vec::with_capacity(lines.len() + 2);
    for line in lines {
        let Some(groups) = captures(signers.as_ref(), &line) else {
            trusted.push(line);
            continue;
        };
        trusted.push("    signers = [".to_owned());
        trusted.push(format!("      \"{signer}\""));
        trusted.push(format!("    ];{}", groups[1]));
    }
    trusted
}

fn chosen<'a>(answers: &'a Answers, block: &str) -> &'a [String] {
    match block {
        "features" => &answers.features,
        "hardware" => &answers.hardware,
        _ => &answers.apps,
    }
}

#[must_use]
pub fn render_host(template: &str, answers: &Answers, signer: &str) -> String {
    let block_start = regex(r"^( *)(features|hardware|apps) = \[$");
    let item = regex(r#"^( *)(# )?"([^"]+)"(.*)$"#);
    let block_end = regex(r"^ *\];");
    let host_attribute = nix_attribute(&answers.host);
    let login_attribute = nix_attribute(&answers.login);

    let mut rendered = Vec::new();
    let mut block: Option<&[String]> = None;
    let mut seen: HashSet<&str> = HashSet::new();
    let mut indent = "";
    for line in lines(template) {
        if let Some(groups) = captures(block_start.as_ref(), line) {
            block = Some(chosen(answers, groups[2]));
            seen.clear();
            rendered.push(line.to_owned());
            continue;
        }

        if let Some(wanted) = block {
            if let Some(groups) = captures(item.as_ref(), line) {
                indent = groups[1];
                let name = groups[3];
                seen.insert(name);
                let mark = if wanted.iter().any(|want| want == name) {
                    ""
                } else {
                    "# "
                };
                rendered.push(format!("{indent}{mark}\"{name}\"{}", groups[4]));
                continue;
            }
            if captures(block_end.as_ref(), line).is_some() {
                for name in wanted.iter().filter(|name| !seen.contains(name.as_str())) {
                    rendered.push(format!("{indent}\"{name}\""));
                }
                block = None;
                rendered.push(line.to_owned());
                continue;
            }
        }

        rendered.push(host_line(line, answers, &host_attribute, &login_attribute));
    }
    joined(trust_signer(rendered, signer))
}

fn host_line(line: &str, answers: &Answers, host_attribute: &str, login_attribute: &str) -> String {
    let login = &answers.login;
    if line == "  configurations.nixos.example = {" {
        return line.replacen("example", host_attribute, 1);
    }
    if line == format!("    users.{EXAMPLE_PERSON} = {{") {
        return format!("    users.{login_attribute} = {{");
    }
    if line.starts_with("    # mirrors = ") {
        return line.replace(EXAMPLE_PERSON, login);
    }
    if line.starts_with(&format!("    flake = \"/home/{EXAMPLE_PERSON}/")) {
        return set_string(line, &format!("/home/{login}/nixos"));
    }
    let fields = [
        ("    system = \"", &answers.system),
        ("    profile = \"", &answers.profile),
        ("    timeZone = \"", &answers.time_zone),
        ("    locale = \"", &answers.locale),
        ("    keyboard = \"", &answers.keyboard),
    ];
    match fields.iter().find(|(prefix, _)| line.starts_with(prefix)) {
        Some((_, value)) => set_string(line, value),
        None => line.to_owned(),
    }
}

fn rename_person(line: &str, login: &str, login_attribute: &str) -> String {
    let example_attribute = format!("users.{EXAMPLE_PERSON}");
    line.split(&example_attribute)
        .map(|part| part.replace(EXAMPLE_PERSON, login))
        .collect::<Vec<_>>()
        .join(&format!("users.{login_attribute}"))
}

#[must_use]
pub fn render_person(template: &str, answers: &Answers) -> String {
    let login = &answers.login;
    let login_attribute = nix_attribute(login);
    let rendered = lines(template).map(|line| {
        if line == format!("  users.{EXAMPLE_PERSON} = {{") {
            return format!("  users.{login_attribute} = {{");
        }
        if line.starts_with("    fullName = \"") {
            return set_string(line, &nix_string(&answers.full_name));
        }
        if line.starts_with("    email = \"") {
            if answers.email.is_empty() {
                return line
                    .replacen("email", "# email", 1)
                    .replace(EXAMPLE_PERSON, login);
            }
            return set_string(line, &nix_string(&answers.email));
        }
        rename_person(line, login, &login_attribute)
    });
    joined(rendered)
}

#[must_use]
pub fn update_kept_example(text: &str, login: &str, fork_flake: &str, signer: &str) -> String {
    let flake = regex(r#"^    (# )?flake = "github:"#);
    let renamed = format!("    users.{} = {{", nix_attribute(login));
    let updated = lines(text)
        .map(|line| {
            if login != EXAMPLE_PERSON && line == format!("    users.{EXAMPLE_PERSON} = {{") {
                return renamed.clone();
            }
            if captures(flake.as_ref(), line).is_some() {
                if !fork_flake.is_empty() {
                    return set_string(line, fork_flake);
                }
                let field = line.trim_start_matches(' ');
                let field = field.strip_prefix("# ").unwrap_or(field);
                return format!("    # {field}");
            }
            if fork_flake.is_empty() && line.starts_with("      \"auto-upgrade\" #") {
                return line.replacen("\"auto-upgrade\"", "# \"auto-upgrade\"", 1);
            }
            line.to_owned()
        })
        .collect();
    joined(trust_signer(updated, signer))
}

#[must_use]
pub fn point_readme(text: &str, host: &str, login: &str) -> String {
    let target = format!("modules/hosts/{host}/default.nix");
    let person = format!("modules/users/{login}/default.nix");
    let pointed = lines(text).map(|line| {
        if line.starts_with("The commented [`example`](modules/hosts/example/default.nix) host ") {
            return format!("Your [`{host}`]({target}) host lists every profile, feature, hardware module and app upstream shipped when you ran `nix run .#init`, one line each, commented out when you did not pick it. Read it top to bottom and keep what you like.");
        }
        if line.starts_with("   - writes `modules/hosts/<host>/default.nix` from the [example host]") {
            return format!("   - writes your host, [`{target}`]({target}), from the example host and your person, [`{person}`]({person}), from the example person;");
        }
        line.to_owned()
    });
    joined(pointed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init::fake::{HOST, PERSON, PI, README};

    const SIGNER: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIFake deploy";

    fn names(list: &str) -> Vec<String> {
        crate::init::answers::words(list)
    }

    fn laptop() -> Answers {
        Answers {
            host: "my-box".to_owned(),
            system: "x86_64-linux".to_owned(),
            profile: "laptop".to_owned(),
            login: "bob".to_owned(),
            full_name: "Bob \"the\" Builder ${x}".to_owned(),
            email: "bob@example.net".to_owned(),
            time_zone: "Europe/Paris".to_owned(),
            keyboard: "fr".to_owned(),
            locale: "fr_FR.UTF-8".to_owned(),
            features: names("disk docker"),
            hardware: names("nvidia framework-13-7040-amd"),
            apps: names("firefox vlc"),
            keep_server: false,
            keep_pi: true,
        }
    }

    #[test]
    fn a_laptop_host_turns_on_what_was_picked_and_appends_what_the_example_lacks() {
        insta::assert_snapshot!(render_host(HOST, &laptop(), SIGNER));
    }

    #[test]
    fn a_person_gets_an_escaped_full_name_and_loses_every_alice() {
        insta::assert_snapshot!(render_person(PERSON, &laptop()));
    }

    #[test]
    fn a_pi_host_without_email_comments_out_the_email() {
        let answers = Answers {
            host: "pi-box".to_owned(),
            system: "aarch64-linux".to_owned(),
            profile: "pi".to_owned(),
            email: String::new(),
            features: names("ssh"),
            hardware: names("raspberry-pi-5"),
            apps: Vec::new(),
            ..laptop()
        };

        insta::assert_snapshot!(format!(
            "{}\n{}",
            render_host(HOST, &answers, SIGNER),
            render_person(PERSON, &answers)
        ));
    }

    #[test]
    fn nix_keywords_are_quoted_as_attributes() {
        let answers = Answers {
            host: "rec".to_owned(),
            login: "in".to_owned(),
            ..laptop()
        };

        insta::assert_snapshot!(format!(
            "{}\n{}\n{}",
            render_host(HOST, &answers, SIGNER),
            render_person(PERSON, &answers),
            point_readme(README, "rec", "in")
        ));
    }

    #[test]
    fn a_kept_example_follows_the_fork_or_drops_auto_upgrade_without_one() {
        insta::assert_snapshot!(format!(
            "{}\n{}",
            update_kept_example(PI, "bob", "github:bob/my-nixos", SIGNER),
            update_kept_example(
                include_str!("../../tests/fixtures/init/modules/hosts/example-server/default.nix"),
                "alice",
                "",
                SIGNER
            )
        ));
    }

    #[test]
    fn a_login_holding_alice_is_not_renamed_twice() {
        let answers = Answers {
            login: "alicex".to_owned(),
            ..laptop()
        };

        let person = render_person(PERSON, &answers);

        assert!(
            person.contains("an account with `users.alicex`"),
            "{person}"
        );
        assert!(!person.contains("alicexx"), "{person}");
    }

    #[test]
    fn nix_strings_escape_backslashes_quotes_and_interpolations() {
        assert_eq!(nix_string(r#"a\b "c" ${d} $e"#), r#"a\\b \"c\" \${d} $e"#);
    }

    #[test]
    fn attributes_are_bare_unless_a_keyword_or_not_an_identifier() {
        assert_eq!(nix_attribute("my-box"), "my-box");
        assert_eq!(nix_attribute("_x"), "_x");
        assert_eq!(nix_attribute("in"), "\"in\"");
        assert_eq!(nix_attribute("1box"), "\"1box\"");
    }
}
