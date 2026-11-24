//! Compile-time embedded, flat catalogs indexed by Chinese source text.
use std::{
    collections::{BTreeMap, btree_map::Entry},
    fmt,
    sync::LazyLock,
};

use config_ui::DesktopLocale;
use serde::{
    Deserializer,
    de::{self, DeserializeSeed, MapAccess, Visitor},
};

pub fn parse_catalog(source: &str) -> Result<BTreeMap<String, String>, serde_json::Error> {
    load_catalog(source, true)
}

fn load_catalog(source: &str, strict: bool) -> Result<BTreeMap<String, String>, serde_json::Error> {
    struct FlatCatalog {
        strict: bool,
    }

    impl<'de> DeserializeSeed<'de> for FlatCatalog {
        type Value = BTreeMap<String, String>;

        fn deserialize<D: Deserializer<'de>>(
            self,
            deserializer: D,
        ) -> Result<Self::Value, D::Error> {
            deserializer.deserialize_map(self)
        }
    }

    impl<'de> Visitor<'de> for FlatCatalog {
        type Value = BTreeMap<String, String>;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a flat object of Chinese source keys and string translations")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            let mut entries = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, serde_json::Value>()? {
                let prefix = key.split_once('.').map_or(key.as_str(), |(prefix, _)| prefix);
                if !prefix.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c)) {
                    if self.strict {
                        return Err(de::Error::custom(
                            "catalog keys must be Chinese source text without domains",
                        ));
                    }
                    continue;
                }

                let serde_json::Value::String(value) = value else {
                    if self.strict {
                        return Err(de::Error::custom("catalog translations must be strings"));
                    }
                    continue;
                };

                match entries.entry(key) {
                    Entry::Vacant(entry) => {
                        if self.strict || !value.trim().is_empty() {
                            entry.insert(value);
                        }
                    }
                    Entry::Occupied(_) if self.strict => {
                        return Err(de::Error::custom("duplicate catalog key"));
                    }
                    Entry::Occupied(_) => {}
                }
            }
            Ok(entries)
        }
    }

    let mut deserializer = serde_json::Deserializer::from_str(source);
    let entries = FlatCatalog { strict }.deserialize(&mut deserializer)?;
    deserializer.end()?;
    Ok(entries)
}

/// Skip unusable entries and keep the first usable translation for each key.
/// If the JSON cannot be parsed, the locale falls back to source text.
pub(crate) struct RuntimeCatalog(Result<BTreeMap<String, String>, serde_json::Error>);

impl RuntimeCatalog {
    pub(crate) fn load(source: &str) -> Self {
        Self(load_catalog(source, false))
    }

    fn translate<'a>(&'a self, source: &'a str) -> &'a str {
        self.0
            .as_ref()
            .ok()
            .and_then(|catalog| catalog.get(source))
            .map(String::as_str)
            .unwrap_or(source)
    }
}

static EN: LazyLock<RuntimeCatalog> =
    LazyLock::new(|| RuntimeCatalog::load(include_str!("locales/en.json")));
static ZH_TW: LazyLock<RuntimeCatalog> =
    LazyLock::new(|| RuntimeCatalog::load(include_str!("locales/zh-TW.json")));

/// A locale belongs to the caller; changing languages never changes global state.
#[derive(Debug, Clone, Copy)]
pub struct I18n {
    locale: DesktopLocale,
}

impl I18n {
    pub fn new(locale: DesktopLocale) -> Self {
        Self { locale }
    }

    /// Missing, blank or damaged translations retain the source text and placeholders.
    pub fn __<'a>(&self, source: &'a str) -> &'a str {
        self.translate_with_catalogs(source, || &EN, || &ZH_TW)
    }

    pub(crate) fn translate_with_catalogs<'a>(
        &self,
        source: &'a str,
        en: impl FnOnce() -> &'a RuntimeCatalog,
        zh_tw: impl FnOnce() -> &'a RuntimeCatalog,
    ) -> &'a str {
        let catalog = match self.locale {
            DesktopLocale::SimplifiedChinese => return source,
            DesktopLocale::TraditionalChinese => zh_tw(),
            DesktopLocale::English => en(),
        };
        catalog.translate(source)
    }
}
