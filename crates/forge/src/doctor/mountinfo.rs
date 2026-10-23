use std::path::PathBuf;

use super::Mount;

#[must_use]
pub fn parse(mountinfo: &str) -> Vec<Mount> {
    mountinfo.lines().filter_map(parse_line).collect()
}

fn parse_line(line: &str) -> Option<Mount> {
    let (mount, filesystem) = line.split_once(" - ")?;
    let target = mount.split(' ').nth(4)?;
    let fstype = filesystem.split(' ').next()?;
    Some(Mount {
        target: PathBuf::from(unescape(target)),
        fstype: unescape(fstype),
    })
}

fn unescape(field: &str) -> String {
    let bytes = field.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let octal = bytes
            .get(index + 1..index + 4)
            .filter(|digits| {
                bytes[index] == b'\\' && digits.iter().all(|digit| (b'0'..=b'7').contains(digit))
            })
            .and_then(|digits| u8::from_str_radix(std::str::from_utf8(digits).ok()?, 8).ok());
        if let Some(byte) = octal {
            decoded.push(byte);
            index += 4;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_and_types_are_read_with_their_escapes_decoded() {
        let mountinfo = "\
22 1 0:21 / / rw,relatime shared:1 - btrfs /dev/mapper/root rw,subvol=/root
23 22 0:22 / /proc rw,nosuid shared:5 - proc proc rw
40 22 0:21 /persist/srv/my\\040app /srv/my\\040app rw shared:1 master:2 - btrfs /dev/mapper/root rw
41 22 0:30 / /mnt/tab\\011and\\134slash rw - tmpfs tmpfs rw
";

        let mounts = parse(mountinfo);

        let targets: Vec<_> = mounts
            .iter()
            .map(|mount| mount.target.display().to_string())
            .collect();
        assert_eq!(
            targets,
            ["/", "/proc", "/srv/my app", "/mnt/tab\tand\\slash"]
        );
        assert_eq!(mounts[0].fstype, "btrfs");
        assert_eq!(mounts[3].fstype, "tmpfs");
    }
}
