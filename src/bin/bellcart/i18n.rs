//! Interface texts. English is the reference and the fallback.
//!
//! Adding a language: add `locales/<code>/bellcart.ftl` with the same keys
//! as English, then list it in [`LOCALES`].

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::LanguageIdentifier;

const LOCALES: &[(&str, &str)] = &[
    ("en", include_str!("../../../locales/en/bellcart.ftl")),
    ("fr", include_str!("../../../locales/fr/bellcart.ftl")),
];

pub struct Localizer {
    lang: &'static str,
    bundle: FluentBundle<FluentResource>,
    fallback: FluentBundle<FluentResource>,
}

impl Localizer {
    /// `requested` wins over the system language. Unknown languages fall
    /// back to English.
    pub fn new(requested: Option<&str>) -> Self {
        let wanted = requested
            .map(str::to_owned)
            .or_else(sys_locale::get_locale)
            .unwrap_or_default();
        let lang = pick(&wanted);
        Self {
            lang,
            bundle: bundle(lang),
            fallback: bundle("en"),
        }
    }

    /// The language code in use, for texts that live in packs.
    pub fn lang(&self) -> &str {
        self.lang
    }

    pub fn get(&self, key: &str) -> String {
        self.format(key, None)
    }

    pub fn with(&self, key: &str, args: &[(&str, FluentValue<'_>)]) -> String {
        let mut fluent_args = FluentArgs::new();
        for (name, value) in args {
            fluent_args.set(*name, value.clone());
        }
        self.format(key, Some(&fluent_args))
    }

    fn format(&self, key: &str, args: Option<&FluentArgs<'_>>) -> String {
        for bundle in [&self.bundle, &self.fallback] {
            if let Some(pattern) = bundle.get_message(key).and_then(|m| m.value()) {
                let mut errors = Vec::new();
                return bundle
                    .format_pattern(pattern, args, &mut errors)
                    .into_owned();
            }
        }
        // A missing key shows up as itself: visible, never a crash.
        key.to_owned()
    }
}

/// Maps a locale such as `fr-FR` or `fr_FR.UTF-8` to a language we ship.
fn pick(wanted: &str) -> &'static str {
    let primary = wanted
        .split(['-', '_', '.'])
        .next()
        .unwrap_or_default()
        .to_lowercase();
    LOCALES
        .iter()
        .map(|(code, _)| *code)
        .find(|code| *code == primary)
        .unwrap_or("en")
}

fn bundle(lang: &'static str) -> FluentBundle<FluentResource> {
    let id: LanguageIdentifier = lang.parse().unwrap_or_default();
    let mut bundle = FluentBundle::new(vec![id]);
    // Plain terminal output: no invisible direction marks around values.
    bundle.set_use_isolating(false);
    let source = LOCALES
        .iter()
        .find(|(code, _)| *code == lang)
        .map_or("", |(_, source)| *source);
    // A partly valid file still yields its valid messages.
    let resource = match FluentResource::try_new(source.to_owned()) {
        Ok(resource) | Err((resource, _)) => resource,
    };
    let _ = bundle.add_resource(resource);
    bundle
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regional_locales_map_to_their_language() {
        assert_eq!(pick("fr-FR"), "fr");
        assert_eq!(pick("fr_FR.UTF-8"), "fr");
        assert_eq!(pick("en-GB"), "en");
        assert_eq!(pick("de-DE"), "en");
        assert_eq!(pick(""), "en");
    }

    #[test]
    fn a_missing_key_is_shown_as_itself() {
        assert_eq!(Localizer::new(Some("fr")).get("no-such-key"), "no-such-key");
    }

    #[test]
    fn french_is_used_when_asked() {
        assert_eq!(
            Localizer::new(Some("fr")).get("decision-leave"),
            "À laisser"
        );
        assert_eq!(
            Localizer::new(Some("en")).get("decision-leave"),
            "Leave behind"
        );
    }
}
