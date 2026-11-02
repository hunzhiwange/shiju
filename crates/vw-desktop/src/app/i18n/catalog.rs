//! Compile-time embedded, flat catalogs indexed by Chinese source text.
use std::{collections::BTreeMap, fmt, sync::LazyLock};

use serde::{
    Deserialize, Deserializer,
    de::{self, MapAccess, Visitor},
};
use vw_config_types::ui::DesktopLocale;

pub(super) fn parse_catalog(source: &str) -> Result<BTreeMap<String, String>, serde_json::Error> {
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

static EN: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    parse_catalog(include_str!("locales/en.json")).expect("invalid embedded English catalog")
});
static ZH_TW: LazyLock<BTreeMap<String, String>> = LazyLock::new(|| {
    parse_catalog(include_str!("locales/zh-TW.json"))
        .expect("invalid embedded Traditional Chinese catalog")
});

/// A locale belongs to the caller; changing languages never changes global state.
#[derive(Debug, Clone, Copy)]
pub struct I18n {
    locale: DesktopLocale,
}

impl I18n {
    pub fn new(locale: DesktopLocale) -> Self {
        Self { locale }
    }

    /// Missing translations retain the source text, including any placeholders.
    pub fn __<'a>(&self, source: &'a str) -> &'a str {
        let catalog = match self.locale {
            DesktopLocale::SimplifiedChinese => return source,
            DesktopLocale::TraditionalChinese => &*ZH_TW,
            DesktopLocale::English => &*EN,
        };
        catalog.get(source).map(String::as_str).unwrap_or(source)
    }
}
