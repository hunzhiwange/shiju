//! Compile-time embedded, flat catalogs indexed by Chinese source text.
use std::{collections::BTreeMap, fmt, sync::LazyLock};

use config_ui::DesktopLocale;
use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, Visitor},
};

pub fn parse_catalog(source: &str) -> Result<BTreeMap<String, String>, serde_json::Error> {
    struct FlatCatalog;
    impl<'de> Visitor<'de> for FlatCatalog {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a flat object of Chinese source keys and string translations")
        }
        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            let mut entries = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, String>()? {
                if !key.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c))
                    || key.split('.').next().is_some_and(|prefix| {
                        !prefix.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c))
                    })
                {
                    return Err(de::Error::custom(
                        "catalog keys must be Chinese source text without domains",
                    ));
                }
                if entries.insert(key, value).is_some() {
                    return Err(de::Error::custom("duplicate catalog key"));
                }
            }
            Ok(entries)
        }
    }
    struct Catalog(BTreeMap<String, String>);
    impl<'de> Deserialize<'de> for Catalog {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_map(FlatCatalog).map(Self)
        }
    }
    serde_json::from_str::<Catalog>(source).map(|catalog| catalog.0)
}

/// Cache strict parsing failures so a damaged locale keeps returning source text.
pub(crate) struct RuntimeCatalog(Result<BTreeMap<String, String>, serde_json::Error>);

impl RuntimeCatalog {
    pub(crate) fn load(source: &str) -> Self {
        Self(parse_catalog(source))
    }

    fn translate<'a>(&'a self, source: &'a str) -> &'a str {
        self.0
            .as_ref()
            .ok()
            .and_then(|catalog| catalog.get(source))
            .filter(|translation| !translation.trim().is_empty())
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
