use super::*;

use serde::Serialize;
use serde::Deserialize;

use lazycell::LazyCell;

use syntect::highlighting::{Theme, ThemeSet};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct LazyThemeSet {
    /// This is a [`BTreeMap`] because that's what [`syntect::highlighting::Theme`] uses
    themes: std::collections::BTreeMap<String, LazyTheme>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LazyTheme {
    serialized: Vec<u8>,

    #[serde(skip, default = "LazyCell::new")]
    deserialized: lazycell::LazyCell<syntect::highlighting::Theme>,
}

impl LazyThemeSet {
    pub fn get(&self, name: &str) -> Option<&Theme> {
        self.themes.get(name).and_then(|t| t.borrow().ok())
    }
}

impl LazyTheme {
    fn deserialize(&self) -> Result<Theme> {
        Ok(from_binary::<Theme>(&self.serialized[..], true))
    }

    fn borrow(&self) -> Result<&Theme> {
        self.deserialized.try_borrow_with(|| {
            self.deserialize()
        })
    }
}

impl From<LazyThemeSet> for ThemeSet {
    fn from(lazy_theme_set: LazyThemeSet) -> Self {
        let mut theme_set = ThemeSet::default();
        for (k, v) in lazy_theme_set.themes {
            theme_set.themes.insert(k.clone(), v.deserialize().unwrap());
        }
        theme_set
    }
}

#[cfg(feature = "build-assets")]
impl From<ThemeSet> for LazyThemeSet {
    fn from(theme_set: ThemeSet) -> Self {
        let mut lazy_theme_set = LazyThemeSet::default();
        for (k, v) in theme_set.themes {
            let lazy_theme = LazyTheme {
                deserialized: LazyCell::new(),
                serialized: crate::assets::build_assets::asset_to_contents(&v, "foo", true).unwrap(),
            };
            lazy_theme_set.themes.insert(k.clone(), lazy_theme);
        }
        lazy_theme_set
    }
}
