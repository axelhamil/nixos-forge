use std::collections::{BTreeSet, HashMap};

use fancy_regex::Regex;
use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

use super::{CheckDocsError, compile};

pub struct Link {
    pub target: String,
    pub line: usize,
}

pub struct Markdown {
    html_url: Regex,
    html_anchor: Regex,
}

impl Markdown {
    pub fn new() -> Result<Self, CheckDocsError> {
        Ok(Self {
            html_url: compile(r#"(?i)(?:^|\s)(href|src|srcset)\s*=\s*(?:"([^"]*)"|'([^']*)')"#)?,
            html_anchor: compile(r#"(?i)(?:^|\s)(?:id|name)\s*=\s*(?:"([^"]*)"|'([^']*)')"#)?,
        })
    }

    pub fn links(&self, text: &str) -> Vec<Link> {
        let mut links = Vec::new();
        for (event, range) in Parser::new_ext(text, options()).into_offset_iter() {
            match event {
                Event::Start(
                    Tag::Link {
                        link_type,
                        dest_url,
                        ..
                    }
                    | Tag::Image {
                        link_type,
                        dest_url,
                        ..
                    },
                ) if link_type != LinkType::Email => links.push(Link {
                    target: dest_url.to_string(),
                    line: line_at(text, range.start),
                }),
                Event::Html(html) | Event::InlineHtml(html) => {
                    let start = line_at(text, range.start);
                    links.extend(self.html_links(&html, start));
                }
                _ => {}
            }
        }
        links
    }

    fn html_links(&self, html: &str, first_line: usize) -> Vec<Link> {
        let mut links = Vec::new();
        for captures in self.html_url.captures_iter(html).flatten() {
            let Some(value) = captures.get(2).or_else(|| captures.get(3)) else {
                continue;
            };
            let line = first_line + html[..value.start()].matches('\n').count();
            let srcset = captures
                .get(1)
                .is_some_and(|attribute| attribute.as_str().eq_ignore_ascii_case("srcset"));
            let targets: Vec<&str> = if srcset {
                value
                    .as_str()
                    .split(',')
                    .filter_map(|candidate| candidate.split_whitespace().next())
                    .collect()
            } else {
                vec![value.as_str()]
            };
            links.extend(targets.into_iter().map(|target| Link {
                target: target.to_owned(),
                line,
            }));
        }
        links
    }

    pub fn anchors(&self, text: &str) -> BTreeSet<String> {
        let mut anchors = BTreeSet::new();
        let mut seen: HashMap<String, usize> = HashMap::new();
        let mut heading: Option<String> = None;

        for event in Parser::new_ext(text, options()) {
            match event {
                Event::Start(Tag::Heading { .. }) => heading = Some(String::new()),
                Event::Text(part) | Event::Code(part) => {
                    if let Some(buffer) = heading.as_mut() {
                        buffer.push_str(&part);
                    }
                }
                Event::End(TagEnd::Heading(_)) => {
                    let slug = slugify(&heading.take().unwrap_or_default());
                    let count = seen.entry(slug.clone()).or_insert(0);
                    anchors.insert(if *count == 0 {
                        slug
                    } else {
                        format!("{slug}-{count}")
                    });
                    *count += 1;
                }
                Event::Html(html) | Event::InlineHtml(html) => {
                    anchors.extend(self.html_anchors(&html));
                }
                _ => {}
            }
        }
        anchors
    }

    fn html_anchors(&self, html: &str) -> Vec<String> {
        self.html_anchor
            .captures_iter(html)
            .flatten()
            .filter_map(|captures| captures.get(1).or_else(|| captures.get(2)))
            .map(|value| value.as_str().to_owned())
            .collect()
    }
}

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
}

fn slugify(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .filter_map(|character| match character {
            ' ' => Some('-'),
            '-' | '_' => Some(character),
            _ if character.is_alphanumeric() => Some(character),
            _ => None,
        })
        .collect()
}

fn line_at(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].matches('\n').count() + 1
}
