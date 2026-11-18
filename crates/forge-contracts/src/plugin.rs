use serde::Deserialize;

pub const CONTRACT: u64 = 1;
pub const CHECK_WIDTH: usize = 14;

#[derive(Debug, Deserialize)]
struct Version {
    contract: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Description {
    pub contract: u64,
    pub name: String,
    pub summary: String,
    #[serde(default)]
    pub doctor: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum DescriptionError {
    #[error(transparent)]
    Invalid(serde_json::Error),
    #[error("contract {found} instead of {CONTRACT}")]
    Contract { found: u64 },
}

impl Description {
    pub fn parse(stdout: &[u8]) -> Result<Self, DescriptionError> {
        let version: Version = serde_json::from_slice(stdout).map_err(DescriptionError::Invalid)?;
        if version.contract != CONTRACT {
            return Err(DescriptionError::Contract {
                found: version.contract,
            });
        }
        serde_json::from_slice(stdout).map_err(DescriptionError::Invalid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum DoctorStatus {
    #[serde(rename = "OK")]
    Ok,
    #[serde(rename = "WARN")]
    Warn,
    #[serde(rename = "FAIL")]
    Fail,
    #[serde(rename = "SKIP")]
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DoctorLine {
    pub status: DoctorStatus,
    pub check: String,
    pub detail: String,
    #[serde(default)]
    pub hint: String,
}

#[derive(Debug, thiserror::Error)]
pub enum LineError {
    #[error(transparent)]
    Json(serde_json::Error),
    #[error("check `{check}` must have 1 to {CHECK_WIDTH} characters")]
    CheckWidth { check: String },
}

impl DoctorLine {
    pub fn parse(text: &str) -> Result<Self, LineError> {
        let line: Self = serde_json::from_str(text).map_err(LineError::Json)?;
        if line.check.is_empty() || line.check.chars().count() > CHECK_WIDTH {
            return Err(LineError::CheckWidth { check: line.check });
        }
        Ok(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_description_with_doctor_round_trips() {
        let description = Description::parse(
            br#"{"contract":1,"name":"backup","summary":"Back up the host","doctor":true}"#,
        )
        .unwrap();

        assert_eq!(
            description,
            Description {
                contract: CONTRACT,
                name: "backup".to_owned(),
                summary: "Back up the host".to_owned(),
                doctor: true,
            }
        );
    }

    #[test]
    fn a_description_without_doctor_does_not_join_the_doctor() {
        let description =
            Description::parse(br#"{"contract":1,"name":"x","summary":"y"}"#).unwrap();

        assert!(!description.doctor);
    }

    #[test]
    fn another_contract_is_refused_with_the_version_found() {
        let error = Description::parse(br#"{"contract":2}"#).unwrap_err();

        assert!(matches!(error, DescriptionError::Contract { found: 2 }));
    }

    #[test]
    fn a_description_without_summary_is_invalid() {
        let error = Description::parse(br#"{"contract":1,"name":"x"}"#).unwrap_err();

        assert!(matches!(error, DescriptionError::Invalid(_)));
    }

    #[test]
    fn a_doctor_line_without_hint_parses() {
        let line = DoctorLine::parse(r#"{"status":"WARN","check":"backup","detail":"3 days old"}"#)
            .unwrap();

        assert_eq!(
            line,
            DoctorLine {
                status: DoctorStatus::Warn,
                check: "backup".to_owned(),
                detail: "3 days old".to_owned(),
                hint: String::new(),
            }
        );
    }

    #[test]
    fn a_doctor_line_check_longer_than_the_column_is_refused() {
        let error = DoctorLine::parse(r#"{"status":"OK","check":"fifteen-letters","detail":""}"#)
            .unwrap_err();

        assert_eq!(
            error.to_string(),
            "check `fifteen-letters` must have 1 to 14 characters"
        );
    }

    #[test]
    fn a_doctor_line_with_an_unknown_status_keeps_the_json_error() {
        let error = DoctorLine::parse(r#"{"status":"MAYBE","check":"x","detail":""}"#).unwrap_err();

        assert!(matches!(error, LineError::Json(_)));
    }
}
