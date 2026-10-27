use std::io;
use std::path::Path;

use fancy_regex::Regex;

use super::options::Flags;
use super::{Environment, Ports, Stop};
use crate::system::interrupt::SIGINT;

pub const TEMPLATE: &str = "modules/hosts/example/default.nix";
pub const EXAMPLE_PERSON: &str = "alice";
pub const PERSON_TEMPLATE: &str = "modules/users/alice/default.nix";

const SYSTEMS: [&str; 2] = ["x86_64-linux", "aarch64-linux"];
const PROFILES: [&str; 4] = ["desktop", "laptop", "server", "pi"];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answers {
    pub host: String,
    pub system: String,
    pub profile: String,
    pub login: String,
    pub full_name: String,
    pub email: String,
    pub time_zone: String,
    pub keyboard: String,
    pub locale: String,
    pub features: Vec<String>,
    pub hardware: Vec<String>,
    pub apps: Vec<String>,
    pub keep_server: bool,
    pub keep_pi: bool,
}

#[must_use]
pub fn words(list: &str) -> Vec<String> {
    let mut words: Vec<String> = Vec::new();
    for word in list.split(|c: char| c == ',' || c.is_whitespace()) {
        if !word.is_empty() && !words.iter().any(|seen| seen == word) {
            words.push(word.to_owned());
        }
    }
    words
}

#[must_use]
pub fn list_block(template: &str, block: &str) -> Vec<(String, String)> {
    let (Ok(start), Ok(end), Ok(item)) = (
        Regex::new(&format!(r"^ *{block} = \[$")),
        Regex::new(r"^ *\];"),
        Regex::new(r#"^ *(# )?"([^"]+)"( # (.*))?$"#),
    ) else {
        return Vec::new();
    };

    let mut entries = Vec::new();
    let mut inside = false;
    for line in template.lines() {
        if !inside {
            inside = start.is_match(line).unwrap_or(false);
            continue;
        }
        if end.is_match(line).unwrap_or(false) {
            break;
        }
        if let Ok(Some(captures)) = item.captures(line) {
            let name = captures.get(2).map_or("", |m| m.as_str());
            let description = captures.get(4).map_or("", |m| m.as_str());
            entries.push((name.to_owned(), description.to_owned()));
        }
    }
    entries
}

#[must_use]
pub fn detected_time_zone(localtime: &Path) -> String {
    let zone = localtime.to_string_lossy();
    match zone.rfind("/zoneinfo/") {
        Some(at) => zone[at + "/zoneinfo/".len()..].to_owned(),
        None => "UTC".to_owned(),
    }
}

#[must_use]
pub fn detected_locale(language: &str) -> String {
    let language = language.replacen(".utf8", ".UTF-8", 1);
    let valid = Regex::new(r"^[a-z]{2,3}_[A-Z]{2}\.UTF-8$")
        .is_ok_and(|pattern| pattern.is_match(&language).unwrap_or(false));
    if valid {
        language
    } else {
        "en_US.UTF-8".to_owned()
    }
}

#[must_use]
pub fn default_features(profile: &str, system: &str, known: &[String]) -> String {
    let mut wanted: Vec<&str> = match profile {
        "desktop" | "laptop" => vec!["disk", "plymouth", "greetd", "nix-ld", "printing"],
        "server" => vec!["disk", "ssh"],
        "pi" => vec!["ssh"],
        _ => Vec::new(),
    };
    if system == "x86_64-linux" && profile != "pi" {
        wanted.push("secure-boot");
    }
    known
        .iter()
        .filter(|feature| wanted.contains(&feature.as_str()))
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
}

#[must_use]
pub fn default_hardware(profile: &str) -> &'static str {
    match profile {
        "desktop" | "laptop" => "firmware audio bluetooth",
        "pi" => "raspberry-pi-5",
        _ => "",
    }
}

#[must_use]
pub fn default_apps(profile: &str) -> &'static str {
    match profile {
        "desktop" | "laptop" => {
            "hyprland waybar fuzzel kitty mako lock upgrade-notify direnv firefox"
        }
        _ => "",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Check {
    Host,
    System,
    Profile,
    Login,
    Text,
    Email,
    Word,
    Features,
    Names,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Need {
    Required,
    Optional,
}

fn matches(pattern: &str, value: &str) -> bool {
    Regex::new(pattern).is_ok_and(|regex| regex.is_match(value).unwrap_or(false))
}

fn check_word(value: &str) -> Result<(), String> {
    if matches(r"^[A-Za-z0-9_+./-]+$", value) {
        return Ok(());
    }
    Err("letters, digits and _ + . / - only".to_owned())
}

pub(super) struct Asker<'s, 'p, 'a> {
    pub ports: &'s mut Ports<'a>,
    pub root: &'p Path,
    pub env: &'p Environment,
    pub template: &'p str,
    pub yes: bool,
    known_features: Vec<String>,
}

impl<'s, 'p, 'a> Asker<'s, 'p, 'a> {
    pub fn new(
        ports: &'s mut Ports<'a>,
        root: &'p Path,
        env: &'p Environment,
        template: &'p str,
        yes: bool,
    ) -> Self {
        let known_features = list_block(template, "features")
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        Self {
            ports,
            root,
            env,
            template,
            yes,
            known_features,
        }
    }

    fn exists(&self, relative: &str) -> bool {
        self.ports.storage.exists(&self.root.join(relative))
    }

    fn check(&self, check: Check, value: &str) -> Result<(), String> {
        match check {
            Check::Host => {
                if !matches(r"^[A-Za-z0-9]([A-Za-z0-9_-]{0,61}[A-Za-z0-9])?$", value) {
                    return Err("a host name has 1 to 63 letters, digits, - and _, and starts and ends with a letter or digit".to_owned());
                }
                if self.exists(&format!("modules/hosts/{value}")) {
                    return Err(format!(
                        "modules/hosts/{value} already exists: pick another host name, init never overwrites a host"
                    ));
                }
                Ok(())
            }
            Check::System if SYSTEMS.contains(&value) => Ok(()),
            Check::System => Err(format!("the system is one of: {}", SYSTEMS.join(" "))),
            Check::Profile if PROFILES.contains(&value) => Ok(()),
            Check::Profile => Err(format!("the profile is one of: {}", PROFILES.join(" "))),
            Check::Login => {
                if !matches(r"^[a-z_][a-z0-9_-]{0,31}$", value) {
                    return Err("a login has at most 32 lowercase letters, digits, - and _, and starts with a letter or _".to_owned());
                }
                if value != EXAMPLE_PERSON && self.exists(&format!("modules/users/{value}")) {
                    return Err(format!(
                        "modules/users/{value} already exists: pick another login, init never overwrites a person"
                    ));
                }
                Ok(())
            }
            Check::Text if !value.is_empty() && !value.chars().any(char::is_control) => Ok(()),
            Check::Text => Err("a non-empty value on one line".to_owned()),
            Check::Email
                if value.is_empty()
                    || matches(
                        r#"^[^[:space:][:cntrl:]"]+@[^[:space:][:cntrl:]"]+$"#,
                        value,
                    ) =>
            {
                Ok(())
            }
            Check::Email => Err("an email such as alice@example.org, or nothing".to_owned()),
            Check::Word => check_word(value),
            Check::Features => match words(value)
                .into_iter()
                .find(|feature| !self.known_features.contains(feature))
            {
                Some(feature) => Err(format!(
                    "unknown feature \"{feature}\"; known: {}",
                    self.known_features.join(" ")
                )),
                None => Ok(()),
            },
            Check::Names => match words(value)
                .into_iter()
                .find(|item| check_word(item).is_err())
            {
                Some(item) => Err(format!("\"{item}\" is not a module or package name")),
                None => Ok(()),
            },
        }
    }

    fn ask(
        &mut self,
        flag: &str,
        given: Option<&str>,
        prompt: &str,
        default: &str,
        check: Check,
        need: Need,
    ) -> Result<String, Stop> {
        if let Some(value) = given {
            self.check(check, value)
                .map_err(|error| Stop::Refused(format!("--{flag}: {error}")))?;
            return Ok(value.to_owned());
        }

        if self.yes {
            if default.is_empty() && need == Need::Required {
                return Err(Stop::Refused(format!("--{flag} is required with --yes")));
            }
            self.check(check, default)
                .map_err(|error| Stop::Refused(format!("--{flag}: {error}")))?;
            return Ok(default.to_owned());
        }

        let shown = if default.is_empty() {
            format!("{prompt}: ")
        } else {
            format!("{prompt} [{default}]: ")
        };
        loop {
            let reply = self.line(&shown, || {
                Stop::Refused(format!(
                    "no answer for {prompt}: pass --{flag}, or --yes for the defaults"
                ))
            })?;
            let reply = if reply.is_empty() {
                default.to_owned()
            } else {
                reply
            };
            if reply.is_empty() && need == Need::Required {
                self.ports.warn("  an answer is needed");
                continue;
            }
            match self.check(check, &reply) {
                Ok(()) => return Ok(reply),
                Err(error) => self.ports.warn(&format!("  {error}")),
            }
        }
    }

    fn line(&mut self, prompt: &str, no_answer: impl Fn() -> Stop) -> Result<String, Stop> {
        let answer = self.ports.terminal.line(prompt);
        self.ports.checkpoint()?;
        answer.map_err(|error| match error.kind() {
            io::ErrorKind::Interrupted => Stop::Interrupted(SIGINT),
            _ => no_answer(),
        })
    }

    fn ask_yes_no(&mut self, given: bool, prompt: &str) -> Result<bool, Stop> {
        if given {
            return Ok(true);
        }
        if self.yes {
            return Ok(false);
        }
        let reply = match self.line(
            &format!("{prompt} [y/N]: "),
            || Stop::Refused(String::new()),
        ) {
            Ok(reply) => reply,
            Err(Stop::Refused(_)) => String::new(),
            Err(stop) => return Err(stop),
        };
        Ok(reply.starts_with(['y', 'Y']))
    }

    fn ask_email(&mut self, given: Option<&str>) -> Result<String, Stop> {
        if let Some(email) = given {
            return self.ask("email", Some(email), "", "", Check::Email, Need::Optional);
        }
        if self.yes {
            return Err(Stop::Refused(
                "--email is required with --yes (--email '' for none)".to_owned(),
            ));
        }
        self.ask(
            "email",
            None,
            "Email, for git (blank for none)",
            "",
            Check::Email,
            Need::Optional,
        )
    }

    fn ask_list(
        &mut self,
        flag: &str,
        given: Option<&str>,
        suggested: &str,
        check: Check,
        prompt: &str,
        title: Option<&str>,
    ) -> Result<Vec<String>, Stop> {
        if let Some(value) = given {
            return self
                .ask(flag, Some(value), "", "", check, Need::Optional)
                .map(|value| words(&value));
        }
        if self.yes {
            return Ok(words(suggested));
        }
        if let Some(title) = title {
            self.show_list(flag, title, suggested);
        }
        self.ask(flag, None, prompt, suggested, check, Need::Optional)
            .map(|value| words(&value))
    }

    fn show_list(&mut self, block: &str, title: &str, suggested: &str) {
        let chosen = words(suggested);
        self.ports
            .warn(&format!("{title} (each one is described in {TEMPLATE}):"));
        for (name, description) in list_block(self.template, block) {
            let mark = if chosen.contains(&name) { "x" } else { " " };
            self.ports
                .warn(&format!("  [{mark}] {name:<14} {description}"));
        }
    }

    fn required(
        &mut self,
        flag: &str,
        given: &str,
        prompt: &str,
        default: &str,
        check: Check,
    ) -> Result<String, Stop> {
        let given = Some(given).filter(|value| !value.is_empty());
        self.ask(flag, given, prompt, default, check, Need::Required)
    }

    fn keep(&mut self, example: &str, given: bool) -> Result<bool, Stop> {
        let dir = self.root.join("modules/hosts").join(example);
        if !self.ports.files.is_dir(&dir) {
            return Ok(given);
        }
        self.ask_yes_no(given, &format!("Keep the {example} host"))
    }

    pub fn collect(&mut self, flags: &Flags) -> Result<Answers, Stop> {
        let host = self.required("host", &flags.host, "Host name", "", Check::Host)?;
        let systems = format!("System ({})", SYSTEMS.join(" "));
        let system = self.required(
            "system",
            &flags.system,
            &systems,
            "x86_64-linux",
            Check::System,
        )?;
        let default_profile = if system == "aarch64-linux" {
            "pi"
        } else {
            "desktop"
        };
        let profiles = format!("Profile ({})", PROFILES.join(" "));
        let profile = self.required(
            "profile",
            &flags.profile,
            &profiles,
            default_profile,
            Check::Profile,
        )?;
        let login = self.required("login", &flags.login, "Login", "", Check::Login)?;
        let full_name =
            self.required("full-name", &flags.full_name, "Full name", "", Check::Text)?;
        let email = self.ask_email(flags.email.as_deref())?;

        let localtime = self.ports.files.canonicalize(Path::new("/etc/localtime"));
        let zone = detected_time_zone(&localtime);
        let time_zone = self.required(
            "time-zone",
            &flags.time_zone,
            "Time zone",
            &zone,
            Check::Word,
        )?;
        let keyboard = self.required(
            "keyboard",
            &flags.keyboard,
            "Keyboard layout",
            "us",
            Check::Word,
        )?;
        let language = detected_locale(&self.env.language);
        let locale = self.required("locale", &flags.locale, "Locale", &language, Check::Word)?;

        let suggested_features = default_features(&profile, &system, &self.known_features);
        let features = self.ask_list(
            "features",
            flags.features.as_deref(),
            &suggested_features,
            Check::Features,
            "Features, space separated (blank keeps the ones marked)",
            Some("Features"),
        )?;
        let hardware = self.ask_list(
            "hardware",
            flags.hardware.as_deref(),
            default_hardware(&profile),
            Check::Names,
            "Hardware: starter modules or nixos-hardware profiles, space separated (blank for none)",
            None,
        )?;
        let apps = self.ask_list(
            "apps",
            flags.apps.as_deref(),
            default_apps(&profile),
            Check::Names,
            "Apps for every user: starter apps or nixpkgs packages, space separated (blank keeps the ones marked)",
            Some("Apps"),
        )?;

        Ok(Answers {
            keep_server: self.keep("example-server", flags.keep_server)?,
            keep_pi: self.keep("example-pi", flags.keep_pi)?,
            host,
            system,
            profile,
            login,
            full_name,
            email,
            time_zone,
            keyboard,
            locale,
            features,
            hardware,
            apps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init::fake::{Machine, environment, run_init};
    use crate::system::interrupt::Flag;
    use crate::system::prompt::Typed;

    #[test]
    fn lists_split_on_commas_and_spaces_and_keep_the_first_of_each_name() {
        assert_eq!(words(" disk,ssh  docker,,disk "), ["disk", "ssh", "docker"]);
    }

    #[test]
    fn default_features_follow_the_order_of_the_example_host() {
        let known: Vec<String> = list_block(crate::init::fake::HOST, "features")
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        assert_eq!(
            default_features("laptop", "x86_64-linux", &known),
            "disk secure-boot plymouth greetd nix-ld printing"
        );
        assert_eq!(
            default_features("server", "aarch64-linux", &known),
            "disk ssh"
        );
        assert_eq!(default_features("pi", "x86_64-linux", &known), "ssh");
    }

    #[test]
    fn the_time_zone_and_locale_are_read_from_the_machine() {
        assert_eq!(
            detected_time_zone(Path::new("/nix/store/x/share/zoneinfo/America/New_York")),
            "America/New_York"
        );
        assert_eq!(detected_time_zone(Path::new("/etc/localtime")), "UTC");
        assert_eq!(detected_locale("fr_FR.utf8"), "fr_FR.UTF-8");
        assert_eq!(detected_locale("C.UTF-8"), "en_US.UTF-8");
    }

    #[test]
    fn an_empty_required_answer_and_a_wrong_one_are_asked_again() {
        let machine = Machine::fork();
        let mut terminal = Typed::new(&[
            "", "box", "", "", "bob", "Bob", "", "", "", "", "", "", "", "", "",
        ]);

        let outcome = run_init(
            &machine,
            &mut terminal,
            &Flag::default(),
            &environment(),
            &[],
        );

        assert_eq!(outcome.code, 0, "{}", outcome.err);
        assert!(
            outcome.err.starts_with("  an answer is needed\n"),
            "{}",
            outcome.err
        );
        assert_eq!(
            &outcome.prompts[..10],
            [
                "Host name: ",
                "Host name: ",
                "System (x86_64-linux aarch64-linux) [x86_64-linux]: ",
                "Profile (desktop laptop server pi) [desktop]: ",
                "Login: ",
                "Full name: ",
                "Email, for git (blank for none): ",
                "Time zone [Europe/Berlin]: ",
                "Keyboard layout [us]: ",
                "Locale [de_DE.UTF-8]: ",
            ]
        );
        assert_eq!(
            &outcome.prompts[10..],
            [
                "Features, space separated (blank keeps the ones marked) [disk secure-boot plymouth greetd nix-ld printing]: ",
                "Hardware: starter modules or nixos-hardware profiles, space separated (blank for none) [firmware audio bluetooth]: ",
                "Apps for every user: starter apps or nixpkgs packages, space separated (blank keeps the ones marked) [hyprland waybar fuzzel kitty mako lock upgrade-notify direnv firefox]: ",
                "Keep the example-server host [y/N]: ",
                "Keep the example-pi host [y/N]: ",
            ]
        );
    }

    #[test]
    fn the_end_of_input_at_a_yes_no_question_keeps_nothing() {
        let machine = Machine::fork();
        let mut terminal = Typed::new(&["box", "", "", "bob", "Bob", "", "", "", "", "", "", ""]);

        let outcome = run_init(
            &machine,
            &mut terminal,
            &Flag::default(),
            &environment(),
            &[],
        );

        assert_eq!(outcome.code, 0, "{}", outcome.err);
        assert!(
            machine
                .ran()
                .iter()
                .any(|line| line.ends_with("rm -r -q -- modules/hosts/example-server"))
        );
        assert!(
            machine
                .ran()
                .iter()
                .any(|line| line.ends_with("rm -r -q -- modules/hosts/example-pi"))
        );
    }

    #[test]
    fn an_interrupt_at_a_prompt_exits_130_before_any_key_or_file() {
        let machine = Machine::fork();
        let mut terminal = Typed::new(&["box"]).then_fail(io::ErrorKind::Interrupted);

        let outcome = run_init(
            &machine,
            &mut terminal,
            &Flag::default(),
            &environment(),
            &[],
        );

        assert_eq!(outcome.code, 130);
        assert_eq!(outcome.err, "");
        assert_eq!(
            machine.ran(),
            ["git rev-parse --is-inside-work-tree --show-cdup"]
        );
    }

    #[test]
    fn a_given_value_that_fails_its_check_names_its_option() {
        let machine = Machine::fork();
        let mut terminal = Typed::new(&[]);

        let outcome = run_init(
            &machine,
            &mut terminal,
            &Flag::default(),
            &environment(),
            &[
                "--yes",
                "--host",
                "box",
                "--login",
                "bob",
                "--full-name",
                "Bob",
                "--email",
                "",
                "--apps",
                "fire$fox",
            ],
        );

        assert_eq!(outcome.code, 1);
        assert_eq!(
            outcome.err,
            "forge init: --apps: \"fire$fox\" is not a module or package name\n"
        );
    }
}
