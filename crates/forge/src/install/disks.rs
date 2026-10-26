use serde::Deserialize;
use serde_json::Value;

use super::{Session, Stop};
use crate::system::Invocation;

const LAYOUT_PREFIX: &str = "disk-main-";
const EXCLUDED_PREFIXES: [&str; 2] = ["/dev/fd", "/dev/zram"];

#[derive(Deserialize)]
struct Listing {
    #[serde(default)]
    blockdevices: Vec<Value>,
}

fn devices(lsblk_json: &str) -> Vec<Value> {
    serde_json::from_str::<Listing>(lsblk_json)
        .map(|listing| listing.blockdevices)
        .unwrap_or_default()
}

fn text<'v>(device: &'v Value, field: &str) -> Option<&'v str> {
    device.get(field).and_then(Value::as_str)
}

fn positive(size: Option<&Value>) -> bool {
    match size {
        Some(Value::Number(number)) => number.as_f64().is_some_and(|size| size > 0.0),
        Some(Value::String(size)) => size.trim().parse::<f64>().is_ok_and(|size| size > 0.0),
        _ => false,
    }
}

fn excluded_name(name: &str) -> bool {
    EXCLUDED_PREFIXES.iter().any(|prefix| {
        name.strip_prefix(prefix)
            .and_then(|rest| rest.chars().next())
            .is_some_and(|next| next.is_ascii_digit())
    })
}

#[must_use]
pub fn candidates(lsblk_json: &str, live: &[String]) -> Vec<String> {
    devices(lsblk_json)
        .iter()
        .filter(|device| text(device, "type") == Some("disk"))
        .filter(|device| device.get("ro") == Some(&Value::Bool(false)))
        .filter(|device| positive(device.get("size")))
        .filter_map(|device| text(device, "name"))
        .filter(|name| !name.is_empty() && !excluded_name(name))
        .filter(|name| !live.iter().any(|live| live == name))
        .map(str::to_owned)
        .collect()
}

#[must_use]
pub fn layout_holders(lsblk_json: &str, target: &str) -> Vec<String> {
    devices(lsblk_json)
        .iter()
        .filter(|device| {
            text(device, "partlabel").is_some_and(|label| label.starts_with(LAYOUT_PREFIX))
        })
        .filter(|device| text(device, "pkname") != Some(target))
        .filter_map(|device| text(device, "name"))
        .map(str::to_owned)
        .collect()
}

fn live_media(session: &Session) -> Result<Vec<String>, Stop> {
    let mounted = session.run(&Invocation::new("findmnt", &["-no", "SOURCE", "/iso"]))?;
    if !mounted.success {
        return Ok(Vec::new());
    }
    let mut source = mounted.line().to_owned();

    let backing = session.run(&Invocation::new("losetup", &["-nO", "BACK-FILE", &source]))?;
    if backing.success && !backing.line().is_empty() {
        let holder = session.run(&Invocation::new(
            "findmnt",
            &["-no", "SOURCE", "-T", backing.line()],
        ))?;
        if !holder.success {
            return Ok(Vec::new());
        }
        holder.line().clone_into(&mut source);
    }

    if !session.ports.storage.is_block_device(source.as_ref()) {
        return Ok(Vec::new());
    }
    let chain = session.run(&Invocation::new("lsblk", &["-nprs", "-o", "NAME", &source]))?;
    Ok(chain.stdout.lines().map(str::to_owned).collect())
}

pub fn list(session: &Session) -> Result<Vec<String>, Stop> {
    let live = live_media(session)?;
    let listing = session.run(&Invocation::new(
        "lsblk",
        &["-J", "-d", "-p", "-b", "-o", "NAME,TYPE,SIZE,RO"],
    ))?;
    Ok(candidates(&listing.stdout, &live))
}

pub fn others_with_layout(session: &Session, target: &str) -> Result<Vec<String>, Stop> {
    let listing = session.run(&Invocation::new(
        "lsblk",
        &["-J", "-l", "-p", "-o", "NAME,PKNAME,PARTLABEL"],
    ))?;
    if !listing.success {
        return Err(Stop::Refused(format!(
            "cannot list the partition labels: {}",
            listing.stderr.trim()
        )));
    }
    Ok(layout_holders(&listing.stdout, target))
}

pub fn describe(session: &Session, disk: &str) -> Result<String, Stop> {
    let size = session.run(&Invocation::new("lsblk", &["-dno", "SIZE", disk]))?;
    let model = session.run(&Invocation::new("lsblk", &["-dno", "MODEL", disk]))?;
    let size = size.line().replace(' ', "");
    let model = model
        .stdout
        .lines()
        .map(|line| line.trim_end_matches(' '))
        .collect::<Vec<_>>()
        .join("\n");
    let model = model.trim_end_matches('\n');
    let model = if model.is_empty() {
        "unknown model"
    } else {
        model
    };
    Ok(format!("{size}, {model}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const LSBLK: &str = r#"{
        "blockdevices": [
            {"name": "/dev/sda", "type": "disk", "size": 8000000000, "ro": false},
            {"name": "/dev/sdb", "type": "disk", "size": 16000000000, "ro": true},
            {"name": "/dev/sdc", "type": "disk", "size": 0, "ro": false},
            {"name": "/dev/sr0", "type": "rom", "size": 1000000, "ro": false},
            {"name": "/dev/fd0", "type": "disk", "size": 1474560, "ro": false},
            {"name": "/dev/zram0", "type": "disk", "size": 4000000000, "ro": false},
            {"name": "/dev/fdisk", "type": "disk", "size": 4000000000, "ro": false},
            {"name": "/dev/nvme0n1", "type": "disk", "size": 512000000000, "ro": false},
            {"name": "/dev/sdd", "type": "disk", "size": 32000000000, "ro": false}
        ]
    }"#;

    #[test]
    fn only_whole_writable_non_empty_disks_off_the_live_media_are_candidates() {
        let live = vec!["/dev/sdd1".to_owned(), "/dev/sdd".to_owned()];

        assert_eq!(
            candidates(LSBLK, &live),
            ["/dev/sda", "/dev/fdisk", "/dev/nvme0n1"]
        );
    }

    #[test]
    fn a_device_with_null_fields_is_left_out_without_a_crash() {
        let lsblk = r#"{"blockdevices": [
            {"name": "/dev/sda", "type": "disk", "size": null, "ro": false, "tran": null},
            {"name": "/dev/sdb", "type": "disk", "size": 8000, "ro": null, "tran": null},
            {"name": null, "type": "disk", "size": 8000, "ro": false},
            {"name": "/dev/sdc", "type": null, "size": 8000, "ro": false},
            {"name": "/dev/vda", "type": "disk", "size": 8000, "ro": false, "tran": null}
        ]}"#;

        assert_eq!(candidates(lsblk, &[]), ["/dev/vda"]);
    }

    #[test]
    fn output_that_is_not_lsblk_json_gives_no_candidate() {
        assert!(candidates("", &[]).is_empty());
        assert!(candidates("{\"blockdevices\": null}", &[]).is_empty());
    }

    #[test]
    fn another_disk_carrying_the_layout_labels_is_named() {
        let lsblk = r#"{"blockdevices": [
            {"name": "/dev/vda", "pkname": null, "partlabel": null},
            {"name": "/dev/vda1", "pkname": "/dev/vda", "partlabel": "disk-main-ESP"},
            {"name": "/dev/vdb", "pkname": null, "partlabel": null},
            {"name": "/dev/vdb1", "pkname": "/dev/vdb", "partlabel": "disk-main-ESP"},
            {"name": "/dev/vdb2", "pkname": "/dev/vdb", "partlabel": "disk-main-luks"},
            {"name": "/dev/vdc1", "pkname": "/dev/vdc", "partlabel": "Basic data partition"}
        ]}"#;

        assert_eq!(
            layout_holders(lsblk, "/dev/vda"),
            ["/dev/vdb1", "/dev/vdb2"]
        );
        assert_eq!(layout_holders(lsblk, "/dev/vdb"), ["/dev/vda1"]);
    }
}
