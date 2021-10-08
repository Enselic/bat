use super::*;

use serde::Serialize;
use serde::Deserialize;

use lazycell::LazyCell;

use syntect::highlighting::Theme;

#[derive(Debug, Default, Serialize, Deserialize)]
struct LazyThemeSet {
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
    pub fn get(&self, name: &str) -> &Theme {
        self.themes.get(name).unwrap().borrow().unwrap()
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
