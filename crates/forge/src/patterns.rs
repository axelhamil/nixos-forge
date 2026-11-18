use std::path::Path;

use forge_kit::{Pattern, statics_declared_under};

use crate::auto_upgrade::sources::{SCP_LIKE, SSH_URL};
use crate::init::answers::{EMAIL, HOST_NAME, LIST_ITEM, LOCALE, LOGIN, WORD};
use crate::init::render::{BARE_ATTRIBUTE, BLOCK_ITEM, BLOCK_START, FLAKE, SIGNERS};
use crate::upstream::notes::{BREAKING_LINE, GITHUB_SHORTHAND};

const ALL: &[&Pattern] = &[
    &SSH_URL,
    &SCP_LIKE,
    &EMAIL,
    &HOST_NAME,
    &LIST_ITEM,
    &LOCALE,
    &LOGIN,
    &WORD,
    &BARE_ATTRIBUTE,
    &BLOCK_ITEM,
    &BLOCK_START,
    &FLAKE,
    &SIGNERS,
    &BREAKING_LINE,
    &GITHUB_SHORTHAND,
];

#[test]
fn every_constant_pattern_compiles() {
    for pattern in ALL {
        let invalid = pattern.is_match("").err().map(|error| error.to_string());
        assert_eq!(invalid, None);
    }
}

#[test]
fn every_static_pattern_of_the_crate_is_listed() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");

    assert_eq!(statics_declared_under(&source).unwrap(), ALL.len());
}
