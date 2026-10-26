use std::collections::BTreeMap;

use serde::Deserialize;
use thiserror::Error;

use super::{Commands, Invocation};

#[derive(Debug, Error)]
pub enum LuksError {
    #[error("cannot read the LUKS header of {device}: {output}")]
    Dump { device: String, output: String },
    #[error("cannot parse the LUKS header of {device}: {source}")]
    Parse {
        device: String,
        source: serde_json::Error,
    },
    #[error("the LUKS header of {device} names keyslot {slot}, which is not a number")]
    Slot { device: String, slot: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    keyslots: Vec<u32>,
    tokens: Vec<Token>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Token {
    kind: String,
    keyslots: Vec<u32>,
}

#[derive(Deserialize)]
struct RawHeader {
    keyslots: BTreeMap<String, serde_json::Value>,
    tokens: BTreeMap<String, RawToken>,
}

#[derive(Deserialize)]
struct RawToken {
    #[serde(rename = "type")]
    kind: String,
    keyslots: Vec<String>,
}

impl Header {
    pub fn read(commands: &dyn Commands, device: &str) -> Result<Self, LuksError> {
        let answer = commands.run(&Invocation::new(
            "cryptsetup",
            &["luksDump", "--dump-json-metadata", device],
        ));
        if !answer.success {
            let output = format!("{}{}", answer.stdout, answer.stderr);
            return Err(LuksError::Dump {
                device: device.to_owned(),
                output: output.trim_end().to_owned(),
            });
        }

        let raw: RawHeader =
            serde_json::from_str(&answer.stdout).map_err(|source| LuksError::Parse {
                device: device.to_owned(),
                source,
            })?;
        let slots = |names: Vec<String>| -> Result<Vec<u32>, LuksError> {
            let mut slots = names
                .into_iter()
                .map(|slot| {
                    slot.parse().map_err(|_| LuksError::Slot {
                        device: device.to_owned(),
                        slot,
                    })
                })
                .collect::<Result<Vec<u32>, _>>()?;
            slots.sort_unstable();
            Ok(slots)
        };

        let tokens = raw
            .tokens
            .into_values()
            .map(|token| {
                Ok(Token {
                    kind: token.kind,
                    keyslots: slots(token.keyslots)?,
                })
            })
            .collect::<Result<_, LuksError>>()?;
        Ok(Self {
            keyslots: slots(raw.keyslots.into_keys().collect())?,
            tokens,
        })
    }

    #[must_use]
    pub fn keyslots(&self) -> &[u32] {
        &self.keyslots
    }

    #[must_use]
    pub fn has_token(&self, kind: &str) -> bool {
        self.tokens.iter().any(|token| token.kind == kind)
    }

    #[must_use]
    pub fn slot_has_token(&self, slot: u32) -> bool {
        self.tokens
            .iter()
            .any(|token| token.keyslots.contains(&slot))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::system::Answer;

    const DEVICE: &str = "/dev/disk/by-partlabel/disk-main-luks";

    const TPM2_ON_SLOT_1: &str = r#"{
  "keyslots": {
    "0": {
      "type": "luks2",
      "key_size": 64,
      "af": { "type": "luks1", "stripes": 4000, "hash": "sha256" },
      "area": { "type": "raw", "offset": "32768", "size": "258048", "encryption": "aes-xts-plain64", "key_size": 64 },
      "kdf": { "type": "argon2id", "time": 4, "memory": 1048576, "cpus": 4, "salt": "x5N8qZ2zv3fVb0V8F3bW5wq5l0m2kQ7oR3p6n9s1t4U=" }
    },
    "1": {
      "type": "luks2",
      "key_size": 64,
      "af": { "type": "luks1", "stripes": 4000, "hash": "sha512" },
      "area": { "type": "raw", "offset": "290816", "size": "258048", "encryption": "aes-xts-plain64", "key_size": 64 },
      "kdf": { "type": "pbkdf2", "hash": "sha512", "iterations": 1000, "salt": "kq0z7V3f9mB2xR6t1Y8wN4cL5sH0pJ3dG7aE2uK9oQ=" }
    }
  },
  "tokens": {
    "0": {
      "type": "systemd-tpm2",
      "keyslots": ["1"],
      "tpm2-blob": "AJ4AIOj9n2kXbQ==",
      "tpm2-pcrs": [7, 15],
      "tpm2-pcr-bank": "sha256",
      "tpm2-policy-hash": "4b1c5e0f2a9d7c3e6b8a1f0d2c4e6a8b9d1f3e5c7a9b0d2f4e6c8a0b2d4f6e8a",
      "tpm2-pin": false,
      "tpm2_pcrlock": false,
      "tpm2_srk": "gQAAAQAiAAs=",
      "tpm2-primary-alg": "ecc"
    }
  },
  "segments": {
    "0": { "type": "crypt", "offset": "16777216", "size": "dynamic", "iv_tweak": "0", "encryption": "aes-xts-plain64", "sector_size": 512 }
  },
  "digests": {
    "0": {
      "type": "pbkdf2",
      "keyslots": ["0", "1"],
      "segments": ["0"],
      "hash": "sha256",
      "iterations": 129774,
      "salt": "Zt0yB6o4kJ1xV8qH2pM5wR9sC3nL7dF0aG4eT6uY1iE=",
      "digest": "p9Q2wE5rT8yU1iO4pA7sD0fG3hJ6kL9zX2cV5bN8mQ="
    }
  },
  "config": { "json_size": "12288", "keyslots_size": "16744448" }
}"#;

    const STRAY_SLOT_31: &str = r#"{
  "keyslots": {
    "0": {
      "type": "luks2",
      "key_size": 64,
      "af": { "type": "luks1", "stripes": 4000, "hash": "sha256" },
      "area": { "type": "raw", "offset": "32768", "size": "258048", "encryption": "aes-xts-plain64", "key_size": 64 },
      "kdf": { "type": "argon2id", "time": 4, "memory": 1048576, "cpus": 4, "salt": "x5N8qZ2zv3fVb0V8F3bW5wq5l0m2kQ7oR3p6n9s1t4U=" }
    },
    "31": {
      "type": "luks2",
      "key_size": 64,
      "af": { "type": "luks1", "stripes": 4000, "hash": "sha256" },
      "area": { "type": "raw", "offset": "290816", "size": "258048", "encryption": "aes-xts-plain64", "key_size": 64 },
      "kdf": { "type": "pbkdf2", "hash": "sha256", "iterations": 1000, "salt": "Hc4tW8zN1qB6vR3yK0pL9sM2xF5dJ7gA0eT4uY8iO1c=" }
    }
  },
  "tokens": {},
  "segments": {
    "0": { "type": "crypt", "offset": "16777216", "size": "dynamic", "iv_tweak": "0", "encryption": "aes-xts-plain64", "sector_size": 512 }
  },
  "digests": {
    "0": {
      "type": "pbkdf2",
      "keyslots": ["0", "31"],
      "segments": ["0"],
      "hash": "sha256",
      "iterations": 129774,
      "salt": "Zt0yB6o4kJ1xV8qH2pM5wR9sC3nL7dF0aG4eT6uY1iE=",
      "digest": "p9Q2wE5rT8yU1iO4pA7sD0fG3hJ6kL9zX2cV5bN8mQ="
    }
  },
  "config": { "json_size": "12288", "keyslots_size": "16744448" }
}"#;

    struct Cryptsetup(Answer);

    impl Commands for Cryptsetup {
        fn run(&self, invocation: &Invocation) -> Answer {
            if invocation.command_line()
                == format!("cryptsetup luksDump --dump-json-metadata {DEVICE}")
            {
                return self.0.clone();
            }
            Answer::exited(127)
        }
    }

    fn header(json: &str) -> Header {
        Header::read(&Cryptsetup(Answer::ok(json)), DEVICE).unwrap()
    }

    #[test]
    fn a_tpm2_token_is_read_with_the_keyslot_it_opens() {
        let header = header(TPM2_ON_SLOT_1);

        assert_eq!(header.keyslots(), [0, 1]);
        assert!(header.has_token("systemd-tpm2"));
        assert!(!header.has_token("systemd-fido2"));
        assert!(header.slot_has_token(1));
        assert!(!header.slot_has_token(0));
    }

    #[test]
    fn a_stray_slot_31_has_no_token() {
        let header = header(STRAY_SLOT_31);

        assert_eq!(header.keyslots(), [0, 31]);
        assert!(!header.has_token("systemd-tpm2"));
        assert!(!header.slot_has_token(31));
    }

    #[test]
    fn keyslots_are_in_numeric_order() {
        let json = r#"{"keyslots": {"10": {}, "2": {}, "0": {}}, "tokens": {}}"#;

        assert_eq!(header(json).keyslots(), [0, 2, 10]);
    }

    #[test]
    fn a_failed_dump_names_the_device_and_the_output() {
        let failed = Answer::failed("Device /dev/sda2 is not a valid LUKS device.\n");

        let error = Header::read(&Cryptsetup(failed), DEVICE).unwrap_err();

        assert_eq!(
            error.to_string(),
            format!(
                "cannot read the LUKS header of {DEVICE}: Device /dev/sda2 is not a valid LUKS device."
            )
        );
    }

    #[test]
    fn a_keyslot_that_is_not_a_number_is_refused() {
        let json = r#"{"keyslots": {"x": {}}, "tokens": {}}"#;

        let error = Header::read(&Cryptsetup(Answer::ok(json)), DEVICE).unwrap_err();

        assert!(matches!(error, LuksError::Slot { slot, .. } if slot == "x"));
    }
}
