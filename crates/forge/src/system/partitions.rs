use serde::Deserialize;
use serde_json::Value;

use super::{Commands, Invocation};

pub const ESP_TYPE: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";

const COLUMNS: &str = "NAME,TYPE,PKNAME,PARTN,PARTTYPE,PARTLABEL";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Partition {
    pub name: String,
    pub disk: Option<String>,
    pub number: Option<u32>,
    pub part_type: Option<String>,
    pub label: Option<String>,
}

impl Partition {
    #[must_use]
    pub fn is_esp(&self) -> bool {
        self.part_type
            .as_deref()
            .is_some_and(|part_type| part_type.eq_ignore_ascii_case(ESP_TYPE))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PartitionsError {
    #[error("lsblk failed: {0}")]
    Lsblk(String),
    #[error("cannot read the output of lsblk: {0}")]
    Invalid(#[from] serde_json::Error),
}

#[derive(Deserialize)]
struct Listing {
    blockdevices: Vec<Row>,
}

#[derive(Deserialize)]
struct Row {
    name: String,
    #[serde(rename = "type")]
    kind: Option<String>,
    pkname: Option<String>,
    partn: Option<Value>,
    parttype: Option<String>,
    partlabel: Option<String>,
}

impl Row {
    fn number(&self) -> Option<u32> {
        match self.partn.as_ref()? {
            Value::Number(number) => number.as_u64().and_then(|n| u32::try_from(n).ok()),
            Value::String(text) => text.trim().parse().ok(),
            _ => None,
        }
    }
}

pub fn list(
    commands: &dyn Commands,
    device: Option<&str>,
) -> Result<Vec<Partition>, PartitionsError> {
    let mut args = vec!["-J", "-l", "-p", "-o", COLUMNS];
    args.extend(device);
    let answer = commands.run(&Invocation::new("lsblk", &args));
    if !answer.success {
        return Err(PartitionsError::Lsblk(answer.stderr.trim().to_owned()));
    }
    parse(&answer.stdout)
}

pub fn parse(json: &str) -> Result<Vec<Partition>, PartitionsError> {
    let listing: Listing = serde_json::from_str(json)?;
    Ok(listing
        .blockdevices
        .into_iter()
        .filter(|row| row.kind.as_deref() == Some("part"))
        .map(|row| Partition {
            number: row.number(),
            name: row.name,
            disk: row.pkname,
            part_type: row.parttype,
            label: row.partlabel,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::Answer;

    const SAMPLE: &str = r#"{
       "blockdevices": [
          {"name":"/dev/vda", "type":"disk", "pkname":null, "partn":null, "parttype":null, "partlabel":null},
          {"name":"/dev/vda1", "type":"part", "pkname":"/dev/vda", "partn":1, "parttype":"c12a7328-f81f-11d2-ba4b-00a0c93ec93b", "partlabel":"disk-main-ESP"},
          {"name":"/dev/vda2", "type":"part", "pkname":"/dev/vda", "partn":2, "parttype":"0fc63daf-8483-4772-8e79-3d69d8477de4", "partlabel":"disk-main-luks"},
          {"name":"/dev/mapper/crypted", "type":"crypt", "pkname":"/dev/vda2", "partn":null, "parttype":null, "partlabel":null},
          {"name":"/dev/vdb1", "type":"part", "pkname":"/dev/vdb", "partn":"1", "parttype":"C12A7328-F81F-11D2-BA4B-00A0C93EC93B", "partlabel":"EFI system partition"}
       ]
    }"#;

    #[test]
    fn parse_keeps_the_partitions_and_reads_an_esp_in_uppercase() {
        let partitions = parse(SAMPLE).unwrap();

        let names: Vec<&str> = partitions.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["/dev/vda1", "/dev/vda2", "/dev/vdb1"]);
        let esps: Vec<&str> = partitions
            .iter()
            .filter(|p| p.is_esp())
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(esps, ["/dev/vda1", "/dev/vdb1"]);
        assert_eq!(
            partitions[2],
            Partition {
                name: "/dev/vdb1".to_owned(),
                disk: Some("/dev/vdb".to_owned()),
                number: Some(1),
                part_type: Some("C12A7328-F81F-11D2-BA4B-00A0C93EC93B".to_owned()),
                label: Some("EFI system partition".to_owned()),
            }
        );
    }

    #[test]
    fn output_that_is_not_lsblk_json_is_an_error() {
        assert!(matches!(parse("vda 8G"), Err(PartitionsError::Invalid(_))));
    }

    struct Lsblk;

    impl Commands for Lsblk {
        fn run(&self, invocation: &Invocation) -> Answer {
            match invocation.command_line().as_str() {
                "lsblk -J -l -p -o NAME,TYPE,PKNAME,PARTN,PARTTYPE,PARTLABEL /dev/vdb" => {
                    Answer::ok(SAMPLE)
                }
                _ => Answer::failed("lsblk: not a block device"),
            }
        }
    }

    #[test]
    fn list_asks_lsblk_for_the_device_and_reports_its_failure() {
        assert_eq!(list(&Lsblk, Some("/dev/vdb")).unwrap().len(), 3);
        assert_eq!(
            list(&Lsblk, None).unwrap_err().to_string(),
            "lsblk failed: lsblk: not a block device"
        );
    }
}
