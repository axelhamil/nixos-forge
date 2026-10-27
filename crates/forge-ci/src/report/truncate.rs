const MARGIN: usize = 1024;

#[derive(Debug, Default)]
struct Blocks<'a> {
    fence: Option<&'a str>,
    details: i64,
}

impl<'a> Blocks<'a> {
    fn follow(&mut self, line: &'a str) {
        if let Some(fence) = self.fence {
            if line == fence {
                self.fence = None;
            }
            return;
        }

        let backticks = line.len() - line.trim_start_matches('`').len();
        if backticks >= 3 {
            self.fence = Some(&line[..backticks]);
        } else if line.starts_with("<details") {
            self.details += 1;
        } else if line.starts_with("</details>") {
            self.details -= 1;
        }
    }

    fn close(&self, out: &mut String) {
        if let Some(fence) = self.fence {
            push_line(out, fence);
        }

        for _ in 0..self.details.max(0) {
            push_line(out, "</details>");
        }
    }
}

fn push_line(out: &mut String, line: &str) {
    out.push_str(line);
    out.push('\n');
}

#[must_use]
pub fn truncate(report: &str, limit: usize, footer: &str) -> String {
    let budget = limit.saturating_sub(MARGIN);
    let mut out = String::new();
    let mut used = 0;
    let mut blocks = Blocks::default();
    for line in report.split_inclusive('\n') {
        let line = line.strip_suffix('\n').unwrap_or(line);
        let size = line.len() + 1;
        if used + size > budget {
            blocks.close(&mut out);
            push_line(&mut out, "");
            push_line(&mut out, footer);
            return out;
        }

        used += size;
        push_line(&mut out, line);
        blocks.follow(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_under_its_limit_is_kept_whole() {
        assert_eq!(truncate("a\nb\n", 2000, "cut"), "a\nb\n");
        assert_eq!(truncate("", 2000, "cut"), "");
    }

    #[test]
    fn a_cut_closes_the_open_fence_then_every_open_details() {
        let report = format!(
            "<details>\n<details open>\n</details>\n<details>\n````text\n```\n<details>\n{}\nlast\n",
            "x".repeat(900)
        );
        assert_eq!(
            truncate(&report, 1996, "cut"),
            format!(
                "<details>\n<details open>\n</details>\n<details>\n````text\n```\n<details>\n{}\n````\n</details>\n</details>\n\ncut\n",
                "x".repeat(900)
            )
        );
    }

    #[test]
    fn a_cut_falls_between_lines_so_never_inside_a_character() {
        let line = "\u{e9}".repeat(300);
        let report = format!("{line}\n{line}\n");
        assert_eq!(
            truncate(&report, 1024 + 601, "cut"),
            format!("{line}\n\ncut\n")
        );
    }
}
