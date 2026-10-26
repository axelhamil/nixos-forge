mod config;
pub mod fido2;
pub mod tpm2;

pub use config::{CONFIG_VARIABLE, Config, ConfigError};

fn exit_code(code: Option<i32>) -> u8 {
    code.and_then(|code| u8::try_from(code).ok())
        .filter(|&code| code != 0)
        .unwrap_or(1)
}
