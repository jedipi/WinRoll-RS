use std::sync::atomic::{AtomicU32, Ordering};
use windows_sys::Win32::Globalization::GetUserDefaultUILanguage;

mod en;
mod zh_hans;
mod zh_hant;

pub struct Language {
    pub id: u32,
    pub name: &'static str,
    windows_ids: &'static [u16],
    messages: &'static [(&'static str, &'static str)],
}

// IDs are persisted in the registry: never reuse or change an existing ID.
pub const LANGUAGES: &[Language] = &[en::LANGUAGE, zh_hans::LANGUAGE, zh_hant::LANGUAGE];
static PREFERENCE: AtomicU32 = AtomicU32::new(0);
static LANGUAGE: AtomicU32 = AtomicU32::new(1);

pub fn preference() -> u32 {
    PREFERENCE.load(Ordering::Relaxed)
}

pub fn normalize_preference(value: u32) -> u32 {
    if value == 0 || LANGUAGES.iter().any(|language| language.id == value) {
        value
    } else {
        1
    }
}

pub fn choices() -> impl Iterator<Item = (u32, &'static str)> {
    std::iter::once((0, text("System default"))).chain(
        LANGUAGES
            .iter()
            .map(|language| (language.id, language.name)),
    )
}

pub fn set_preference(value: u32) {
    let value = normalize_preference(value);
    let resolved = if value == 0 {
        // SAFETY: this query has no pointer arguments or side effects.
        system_language(unsafe { GetUserDefaultUILanguage() })
    } else {
        value
    };
    PREFERENCE.store(value, Ordering::Relaxed);
    LANGUAGE.store(resolved, Ordering::Relaxed);
}

fn system_language(id: u16) -> u32 {
    LANGUAGES
        .iter()
        .find(|language| language.windows_ids.contains(&id))
        .map_or(1, |language| language.id)
}

pub fn text(english: &str) -> &str {
    translate(english, LANGUAGE.load(Ordering::Relaxed))
}

fn translate(english: &str, id: u32) -> &str {
    LANGUAGES
        .iter()
        .find(|language| language.id == id)
        .and_then(|language| language.messages.iter().find(|(key, _)| *key == english))
        .map_or(english, |(_, translation)| *translation)
}

#[cfg(test)]
#[test]
fn language_mapping_and_catalog() {
    let mut ids = std::collections::HashSet::new();
    let mut windows_ids = std::collections::HashSet::new();
    let english: std::collections::HashSet<_> =
        en::LANGUAGE.messages.iter().map(|(key, _)| *key).collect();
    for language in LANGUAGES {
        assert!(
            language.id != 0 && ids.insert(language.id),
            "duplicate or reserved language ID"
        );
        assert!(!language.name.is_empty());
        assert_eq!(normalize_preference(language.id), language.id);
        for id in language.windows_ids {
            assert!(windows_ids.insert(id), "duplicate Windows language ID");
            assert_eq!(system_language(*id), language.id);
        }
        let mut keys = std::collections::HashSet::new();
        for &(key, translation) in language.messages {
            assert!(keys.insert(key), "duplicate translation key: {key}");
            assert!(english.contains(key), "unknown translation key: {key}");
            assert!(!translation.is_empty());
            assert_eq!(translate(key, language.id), translation);
            let placeholders = |value: &str| {
                let mut tokens = value
                    .split('{')
                    .skip(1)
                    .map(|part| part.split('}').next().unwrap().to_owned())
                    .collect::<Vec<_>>();
                tokens.sort();
                tokens
            };
            assert_eq!(
                placeholders(key),
                placeholders(translation),
                "placeholders in {key}"
            );
        }
        assert_eq!(keys, english, "missing translations in {}", language.name);
        assert_eq!(
            translate("Windows error detail", language.id),
            "Windows error detail"
        );
    }
    assert_eq!(normalize_preference(0), 0);
    assert_eq!(normalize_preference(u32::MAX), 1);
    assert_eq!(system_language(0xffff), 1);
    assert_eq!(translate("Close", u32::MAX), "Close");
    assert_eq!(
        (en::LANGUAGE.id, zh_hans::LANGUAGE.id, zh_hant::LANGUAGE.id),
        (1, 2, 3)
    );
}
