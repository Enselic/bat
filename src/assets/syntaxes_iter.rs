/*
use super::*;

pub struct SyntaxesIter<'a> {
    assets: &'a HighlightingAssets,
    inner: Option<std::slice::Iter<'a, SyntaxReference>>,
}

impl<'a> SyntaxesIter<'a> {
    pub fn new(assets: &'a HighlightingAssets) -> Self {
        SyntaxesIter {
            assets,
            inner: None,
        }
    }
}

impl<'a> Iterator for SyntaxesIter<'a> {
    type Item = &'a SyntaxReference;

    fn next(&mut self) -> Option<Self::Item> {
        if self.inner.is_none() {
            self.inner = self
                .assets
                .get_syntax_set()
                .ok()
                .map(|i| i.syntaxes().iter());
        }
        match &mut self.inner {
            Some(inner) => inner.next(),
            None => None,
        }
    }
}
*/
