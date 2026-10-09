//! Keeps `schemas/*.json` in step with the Rust types.
//!
//! To regenerate after changing a type:
//! `BELLCART_UPDATE_SCHEMAS=1 cargo test --test schemas`

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

#[test]
fn checked_in_schemas_match_the_types() {
    let dir = common::repo_root().join("schemas");
    let update = std::env::var_os("BELLCART_UPDATE_SCHEMAS").is_some();

    for (name, expected) in bellcart::schema::all() {
        let path = dir.join(name);
        if update {
            std::fs::write(&path, &expected).expect("schema written");
            continue;
        }
        let actual = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .replace("\r\n", "\n");
        assert!(
            actual == expected,
            "{name} is out of date. Run:\n  \
             BELLCART_UPDATE_SCHEMAS=1 cargo test --test schemas"
        );
    }
}
