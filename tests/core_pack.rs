//! Checks on the pack that ships in the binary.

// Test helpers may panic: a failed expectation is the point.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

mod common;

use std::collections::BTreeSet;

use bellcart::pack::{Name, Step, Text};

fn locale_keys(lang: &str) -> BTreeSet<String> {
    let path = common::repo_root().join(format!("locales/{lang}/bellcart.ftl"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
        .lines()
        .filter(|line| !line.starts_with('#') && !line.starts_with(' '))
        .filter_map(|line| line.split_once('='))
        .map(|(key, _)| key.trim().to_owned())
        .collect()
}

#[test]
fn the_core_pack_loads() {
    let pack = common::core_pack();
    assert_eq!(pack.meta.id, "core");
    assert!(!pack.rules.is_empty());
    assert!(!pack.recipes.is_empty());
    assert!(pack.questions.is_some());
}

#[test]
fn every_rule_reason_is_translated_in_every_language() {
    let pack = common::core_pack();
    for lang in ["en", "fr"] {
        let keys = locale_keys(lang);
        for rule in &pack.rules {
            for used in std::iter::once(&rule.reason).chain(rule.parts.as_ref()) {
                let key = format!("reason-{used}");
                assert!(
                    keys.contains(&key),
                    "rule `{}` uses `{key}`, which locales/{lang}/bellcart.ftl does not define",
                    rule.id
                );
            }
        }
    }
}

#[test]
fn french_defines_exactly_the_english_keys() {
    assert_eq!(locale_keys("en"), locale_keys("fr"));
}

#[test]
fn recipe_texts_shipped_in_core_have_a_french_translation() {
    fn texts(step: &Step) -> Vec<&Text> {
        match step {
            Step::Manual { text } => vec![text],
            Step::Copy { note, .. } => note.iter().collect(),
            _ => Vec::new(),
        }
    }
    for recipe in &common::core_pack().recipes {
        for step in recipe.before.iter().chain(&recipe.after) {
            for text in texts(step) {
                assert!(
                    text.translations.contains_key("fr"),
                    "recipe `{}` has a text without French: {}",
                    recipe.id,
                    text.en
                );
            }
        }
    }
}

#[test]
fn translated_recipe_names_in_core_have_french() {
    for recipe in &common::core_pack().recipes {
        if let Name::Translated(text) = &recipe.name {
            assert!(
                text.translations.contains_key("fr"),
                "recipe `{}` has a name without French",
                recipe.id
            );
        }
    }
}

#[test]
fn a_recipe_name_may_be_one_string_or_translated() {
    let pack = common::core_pack();
    let name = |id: &str| &pack.recipes.iter().find(|r| r.id == id).unwrap().name;
    assert_eq!(name("git").get("fr"), "Git");
    assert_eq!(name("openssh").get("fr"), "Clés OpenSSH");
    assert_eq!(name("openssh").en(), "OpenSSH keys");
    // A language the name does not have falls back to English.
    assert_eq!(name("openssh").get("de"), "OpenSSH keys");
}

#[test]
fn the_destination_question_offers_the_three_carts() {
    use bellcart::arbiter::Question;
    let questions = common::core_pack().questions.expect("questions");
    let Question::Choice { criteria, .. } = &questions.questions["destination"] else {
        panic!("`destination` must be a choice");
    };
    for option in ["migrate", "archive", "leave", "inspect"] {
        assert!(criteria.contains_key(option), "missing option `{option}`");
    }
}

#[test]
fn a_rule_that_proposes_to_leave_says_where_the_data_is() {
    // A proposal is never applied without the user, but `leave` must still
    // come with a reason that says why nothing is lost.
    let allowed = [
        "app-cloud-account",
        "installed-program",
        "app-sync-client",
        "app-downloaded-tools",
        "application-cache",
        "system-managed",
    ];
    for rule in &common::core_pack().rules {
        if rule.propose == Some(bellcart::plan::Decision::Leave) {
            assert!(
                allowed.contains(&rule.reason.as_str()),
                "rule `{}` proposes leave for `{}`",
                rule.id,
                rule.reason
            );
        }
    }
}

#[test]
fn every_rule_that_proposes_names_what_it_recognised() {
    for rule in &common::core_pack().rules {
        if rule.propose.is_some() {
            assert!(rule.name.is_some(), "rule `{}` has no name", rule.id);
        }
    }
}
