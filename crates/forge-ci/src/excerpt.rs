use std::collections::HashSet;
use std::io::{self, BufRead};

const KEPT_LINES: usize = 20;

const FAILURE_MARKS: [&str; 7] = [
    "AssertionError",
    "RequestedAssertionFailed",
    "Traceback",
    "timed out",
    "Timeout",
    "hash mismatch",
    "No space left",
];

const NIX_ERROR: &str = "error:";

pub fn excerpt(mut log: impl BufRead) -> io::Result<Vec<String>> {
    let mut excerpt = Excerpt::default();
    let mut bytes = Vec::new();
    let mut older = String::new();
    let mut previous = String::new();
    loop {
        bytes.clear();
        if log.read_until(b'\n', &mut bytes)? == 0 {
            break;
        }
        let raw = String::from_utf8_lossy(&bytes);
        let line = clean(raw.strip_suffix('\n').unwrap_or(&raw));

        if is_failure(&line) {
            excerpt.emit(&older);
            excerpt.emit(&previous);
            excerpt.emit(&line);
        }

        older = previous;
        previous = line;
    }
    Ok(excerpt.last())
}

#[derive(Default)]
struct Excerpt {
    seen: HashSet<String>,
    kept: Vec<String>,
}

impl Excerpt {
    fn emit(&mut self, line: &str) {
        if line.is_empty() || !self.seen.insert(line.to_owned()) {
            return;
        }
        self.kept.push(line.to_owned());
    }

    fn last(mut self) -> Vec<String> {
        let start = self.kept.len().saturating_sub(KEPT_LINES);
        self.kept.split_off(start)
    }
}

fn is_failure(line: &str) -> bool {
    line.starts_with(NIX_ERROR)
        || line.contains(&format!(": {NIX_ERROR}"))
        || FAILURE_MARKS.iter().any(|mark| line.contains(mark))
}

fn clean(line: &str) -> String {
    let line = without_escapes(line);
    let line = without_builder_prefix(&line);
    let line = without_quoted_prefix(line);
    let line = line.strip_prefix("!!! ").unwrap_or(line);
    line.trim_end_matches([' ', '\t', '\r']).to_owned()
}

fn without_escapes(line: &str) -> String {
    let mut clean = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find("\u{1b}[") {
        clean.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let parameters = after
            .find(|char: char| !(char.is_ascii_digit() || char == ';' || char == '?'))
            .unwrap_or(after.len());
        match after[parameters..].chars().next() {
            Some(end) if end.is_ascii_alphabetic() => {
                rest = &after[parameters + end.len_utf8()..];
            }
            _ => {
                clean.push('\u{1b}');
                rest = &rest[start + 1..];
            }
        }
    }
    clean.push_str(rest);
    clean
}

fn without_builder_prefix(line: &str) -> &str {
    let name = line
        .find(|char: char| !(char.is_ascii_alphanumeric() || "._+-".contains(char)))
        .unwrap_or(line.len());
    if name == 0 {
        return line;
    }
    line[name..].strip_prefix("> ").unwrap_or(line)
}

fn without_quoted_prefix(line: &str) -> &str {
    let spaces = line.len() - line.trim_start_matches(' ').len();
    if spaces == 0 {
        return line;
    }
    line[spaces..].strip_prefix("> ").unwrap_or(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_escape_without_its_final_letter_is_kept() {
        assert_eq!(without_escapes("a\u{1b}[31"), "a\u{1b}[31");
        assert_eq!(without_escapes("\u{1b}[?25lb\u{1b}[0m"), "b");
    }

    #[test]
    fn an_error_inside_a_word_is_no_failure() {
        assert!(!is_failure("noerror: fine"));
        assert!(is_failure("main.c:3:1: error: x"));
    }
}
