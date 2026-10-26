mod config;
pub mod fido2;
pub mod tpm2;

use std::io;
use std::path::Path;

pub use config::{CONFIG_VARIABLE, Config};

use crate::system::{ConfigError, Machine, config as settings};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method<'a> {
    Tpm2,
    Fido2 { device: Option<&'a str> },
}

impl Method<'_> {
    fn name(self) -> &'static str {
        match self {
            Self::Tpm2 => "tpm2",
            Self::Fido2 { .. } => "fido2",
        }
    }
}

#[must_use]
pub fn run(method: Method, config_path: Option<&Path>) -> u8 {
    let config = match config_for(method, settings::load(config_path)) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("forge enroll {}: {error}", method.name());
            return 1;
        }
    };

    match method {
        Method::Tpm2 => tpm2::start(&config),
        Method::Fido2 { device } => fido2::run(
            device,
            config.luks_device.as_deref(),
            &Machine,
            &mut io::stdout().lock(),
            &mut io::stderr().lock(),
        ),
    }
}

fn config_for(method: Method, loaded: Result<Config, ConfigError>) -> Result<Config, ConfigError> {
    match (loaded, method) {
        (Err(ConfigError::Unset { .. }), Method::Fido2 { .. }) => Ok(Config::default()),
        (loaded, _) => loaded,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fido2_without_a_config_has_no_disk_to_default_to() {
        let config = config_for(Method::Fido2 { device: None }, settings::load(None)).unwrap();

        assert_eq!(config, Config::default());
    }

    #[test]
    fn tpm2_without_a_config_is_refused() {
        let error = config_for(Method::Tpm2, settings::load(None)).unwrap_err();

        assert_eq!(
            error.to_string(),
            "FORGE_ENROLL_CONFIG is not set: run the forge this system installs"
        );
    }

    #[test]
    fn fido2_with_an_unreadable_config_is_refused() {
        let path = Path::new("/nonexistent/enroll.json");

        let error =
            config_for(Method::Fido2 { device: None }, settings::load(Some(path))).unwrap_err();

        assert!(matches!(error, ConfigError::Read { .. }));
    }
}
