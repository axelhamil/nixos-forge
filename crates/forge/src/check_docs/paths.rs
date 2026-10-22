use std::path::{Component, Path};

pub fn to_slash(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for component in Path::new(path).components() {
        match component {
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
            Component::ParentDir if parts.last().is_some_and(|last| *last != "..") => {
                parts.pop();
            }
            Component::ParentDir => parts.push(".."),
            Component::Normal(part) => parts.push(part.to_str().unwrap_or_default()),
        }
    }
    if parts.is_empty() {
        return ".".to_owned();
    }
    parts.join("/")
}

pub fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

pub fn join(dir: &str, path: &str) -> String {
    if dir.is_empty() {
        return path.to_owned();
    }
    format!("{dir}/{path}")
}

pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if let Some(byte) = escaped_byte(bytes, index) {
            decoded.push(byte);
            index += 3;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8(decoded).unwrap_or_else(|_| text.to_owned())
}

pub fn percent_encode(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for byte in path.bytes() {
        if byte.is_ascii_graphic() && !b"\"#<>?`{}".contains(&byte) {
            encoded.push(char::from(byte));
            continue;
        }
        encoded.push('%');
        encoded.push(hex_digit(byte >> 4));
        encoded.push(hex_digit(byte & 0x0f));
    }
    encoded
}

fn hex_digit(nibble: u8) -> char {
    char::from_digit(u32::from(nibble), 16).map_or('0', |digit| digit.to_ascii_uppercase())
}

fn escaped_byte(bytes: &[u8], index: usize) -> Option<u8> {
    if bytes.get(index) != Some(&b'%') {
        return None;
    }
    let high = char::from(*bytes.get(index + 1)?).to_digit(16)?;
    let low = char::from(*bytes.get(index + 2)?).to_digit(16)?;
    u8::try_from(high * 16 + low).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_resolves_dots() {
        assert_eq!(normalize("docs/../README.md"), "README.md");
        assert_eq!(normalize("docs/a/./b/"), "docs/a/b");
        assert_eq!(normalize("docs/../../x"), "../x");
    }

    #[test]
    fn percent_decode_keeps_invalid_escapes() {
        assert_eq!(percent_decode("a%20b"), "a b");
        assert_eq!(percent_decode("100%"), "100%");
        assert_eq!(percent_decode("%zz"), "%zz");
    }

    #[test]
    fn percent_encode_writes_a_url_path() {
        assert_eq!(percent_encode("docs/my file.md"), "docs/my%20file.md");
        assert_eq!(percent_encode("docs/café.md"), "docs/caf%C3%A9.md");
    }
}
