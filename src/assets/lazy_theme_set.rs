use super::*;

use std::convert::TryFrom;

use serde::Deserialize;
use serde::Serialize;

use once_cell::unsync::OnceCell;

use syntect::highlighting::{Theme, ThemeSet};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LazyThemeSet {
    /// This is a [`BTreeMap`] because that's what [`syntect::highlighting::ThemeSet`] uses
    themes: std::collections::BTreeMap<String, LazyTheme>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LazyTheme {
    serialized: Vec<u8>,

    #[serde(skip, default = "OnceCell::new")]
    deserialized: OnceCell<syntect::highlighting::Theme>,
}

impl LazyThemeSet {
    pub fn get(&self, name: &str) -> Option<&Theme> {
        self.themes.get(name).and_then(|lazy_theme| {
            lazy_theme
                .deserialized
                .get_or_try_init(|| lazy_theme.deserialize())
                .ok()
        })
    }

    pub fn themes(&self) -> impl Iterator<Item = &str> {
        self.themes.keys().map(|s| s.as_ref())
    }
}

impl LazyTheme {
    fn deserialize(&self) -> Result<Theme> {
        asset_from_contents(
            &self.serialized[..],
            "lazy-loaded theme",
            COMPRESS_LAZY_THEMES,
        )
    }
}

impl TryFrom<LazyThemeSet> for ThemeSet {
    type Error = Error;

    fn try_from(lazy_theme_set: LazyThemeSet) -> Result<Self> {
        let mut theme_set = ThemeSet::default();
        for (k, v) in lazy_theme_set.themes {
            theme_set.themes.insert(k.clone(), v.deserialize()?);
        }
        Ok(theme_set)
    }
}

#[cfg(feature = "build-assets")]
impl TryFrom<ThemeSet> for LazyThemeSet {
    type Error = Error;

    fn try_from(theme_set: ThemeSet) -> Result<Self> {
        let mut lazy_theme_set = LazyThemeSet::default();
        for (theme_name, v) in theme_set.themes {
            let lazy_theme = LazyTheme {
                serialized: crate::assets::build_assets::asset_to_contents(
                    &v,
                    &format!("theme {}", theme_name),
                    COMPRESS_LAZY_THEMES,
                )?,
                deserialized: OnceCell::new(),
            };
            lazy_theme_set.themes.insert(theme_name, lazy_theme);
        }
        Ok(lazy_theme_set)
    }
}
