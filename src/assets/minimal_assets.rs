use std::collections::HashMap;

use lazycell::LazyCell;

use syntect::parsing::Regex;
use syntect::parsing::SyntaxSet;

use super::*;

#[derive(Debug)]
pub(crate) struct MinimalAssets {
    minimal_syntaxes: MinimalSyntaxes,

    /// Lazily load serialized [SyntaxSet]s from [Self.minimal_syntaxes]. The
    /// index in this vec matches the index in
    /// [Self.minimal_syntaxes.serialized_syntax_sets]
    deserialized_minimal_syntaxes: Vec<LazyCell<SyntaxSet>>,
}

/// Stores and allows lookup of minimal [SyntaxSet]s. The [SyntaxSet]s are
/// stored in serialized form, and are deserialized on-demand. This gives good
/// startup performance since only the necessary [SyntaxReference]s needs to be
/// deserialized.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub(crate) struct MinimalSyntaxes {
    /// Lookup the index into `serialized_syntax_sets` of a [SyntaxSet] by the
    /// name of any [SyntaxReference] inside the [SyntaxSet]
    pub(crate) by_name: HashMap<String, usize>,

    /// Same as [Self.by_name] but by file extension
    pub(crate) by_file_extension: HashMap<String, usize>,

    /// TODO
    pub(crate) by_first_line_match: Vec<Vec<String>>,

    /// Serialized [SyntaxSet]s. Whether or not this data is compressed is
    /// decided by [COMPRESS_SERIALIZED_MINIMAL_SYNTAXES]
    pub(crate) serialized_syntax_sets: Vec<Vec<u8>>,
}

impl MinimalAssets {
    pub(crate) fn new(minimal_syntaxes: MinimalSyntaxes) -> Self {
        // Prepare so we can lazily load minimal syntaxes without a mut reference
        let deserialized_minimal_syntaxes =
            vec![LazyCell::new(); minimal_syntaxes.serialized_syntax_sets.len()];

        Self {
            minimal_syntaxes,
            deserialized_minimal_syntaxes,
        }
    }

    pub fn get_syntax_set_by_name(&self, name: &str) -> Option<&SyntaxSet> {
        self.minimal_syntaxes
            .by_name
            .get(&name.to_ascii_lowercase())
            .and_then(|index| self.get_minimal_syntax_set_with_index(*index))
    }

    pub fn get_syntax_set_by_extension(&self, extension: &str) -> Option<&SyntaxSet> {
        self.minimal_syntaxes
            .by_file_extension
            .get(&extension.to_ascii_lowercase())
            .and_then(|index| self.get_minimal_syntax_set_with_index(*index))
    }

    pub fn find_syntax_by_token(&self, language: &str) -> Result<Option<SyntaxReferenceInSet>> {
        Ok(match self.get_syntax_set_by_token(language)? {
            Some(syntax_set) => syntax_set
                .find_syntax_by_token(language)
                .map(|syntax| SyntaxReferenceInSet { syntax, syntax_set }),
            None => None,
        })
    }
    pub fn find_syntax_by_name(&self, name: &str) -> Result<Option<SyntaxReferenceInSet>> {
        Ok(match self.get_syntax_set_by_name(name) {
            Some(syntax_set) => syntax_set
                .find_syntax_by_name(name)
                .map(|syntax| SyntaxReferenceInSet { syntax, syntax_set }),
            None => None,
        })
    }

    pub fn find_syntax_by_extension(
        &self,
        e: Option<&OsStr>,
    ) -> Result<Option<SyntaxReferenceInSet>> {
        let extension = e.and_then(|x| x.to_str()).unwrap_or_default();
        Ok(match self.get_syntax_set_by_extension(extension) {
            Some(syntax_set) => syntax_set
                .find_syntax_by_extension(extension)
                .map(|syntax| SyntaxReferenceInSet { syntax, syntax_set }),
            None => None,
        })
    }

    pub fn get_extension_syntax(&self, file_name: &OsStr) -> Result<Option<SyntaxReferenceInSet>> {
        let mut syntax = self.find_syntax_by_extension(Some(file_name))?;
        if syntax.is_none() {
            syntax = self.find_syntax_by_extension(Path::new(file_name).extension())?;
        }
        if syntax.is_none() {
            syntax = try_with_stripped_suffix(file_name, |stripped_file_name| {
                self.get_extension_syntax(stripped_file_name) // Note: recursion
            })?;
        }
        Ok(syntax)
    }

    pub fn get_syntax_set_by_token(&self, language: &str) -> Result<Option<&SyntaxSet>> {
        Ok(match self.get_syntax_set_by_extension(language) {
            None => self.get_syntax_set_by_name(language),
            syntax_set => syntax_set,
        })
    }

    pub fn find_syntax_by_first_line(
        &self,
        first_line: &str,
    ) -> Result<Option<SyntaxReferenceInSet>> {
        for (index, first_line_matches) in
            self.minimal_syntaxes.by_first_line_match.iter().enumerate()
        {
            for first_line_match in first_line_matches {
                // TODO: cache?
                let regex = Regex::new(first_line_match.into());
                if regex.search(first_line, 0, first_line.len(), None) {
                    let syntax_set = self.get_minimal_syntax_set_with_index(index);
                    return Ok(syntax_set
                        .and_then(|ss| ss.find_syntax_by_first_line(first_line))
                        .map(|syntax| SyntaxReferenceInSet {
                            syntax,
                            syntax_set: syntax_set.unwrap(),
                        }));
                }
            }
        }

        Ok(None)
    }

    fn load_minimal_syntax_set_with_index(&self, index: usize) -> Result<SyntaxSet> {
        let serialized_syntax_set = &self.minimal_syntaxes.serialized_syntax_sets[index];
        asset_from_contents(
            &serialized_syntax_set[..],
            &format!("minimal syntax set {}", index),
            COMPRESS_SERIALIZED_MINIMAL_SYNTAXES,
        )
        .map_err(|_| format!("Could not parse minimal syntax set {}", index).into())
    }

    fn get_minimal_syntax_set_with_index(&self, index: usize) -> Option<&SyntaxSet> {
        self.deserialized_minimal_syntaxes
            .get(index)
            .and_then(|cell| {
                cell.try_borrow_with(|| self.load_minimal_syntax_set_with_index(index))
                    .ok()
            })
    }

    pub fn syntaxes_iter(&self) -> impl Iterator<Item = &SyntaxReference> {
        SyntaxDefinitionIterator::new(self)
    }
}

struct SyntaxDefinitionIterator<'a> {
    minimal_assets: &'a MinimalAssets,
    current_outer_index: usize,
    current_inner_index: usize,
}

impl<'a> SyntaxDefinitionIterator<'a> {
    pub fn new(minimal_assets: &'a MinimalAssets) -> Self {
        SyntaxDefinitionIterator {
            minimal_assets,
            current_outer_index: 0,
            current_inner_index: 0,
        }
    }
}

impl<'a> Iterator for SyntaxDefinitionIterator<'a> {
    type Item = &'a SyntaxReference;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let syntax_set = self
            .minimal_assets
            .get_minimal_syntax_set_with_index(self.current_outer_index)?;

        let syntaxes = syntax_set.syntaxes();
        if syntaxes.len() < self.current_inner_index {
            Some(&syntaxes[self.current_inner_index])
        } else {
            self.current_outer_index += 1;
            self.current_inner_index = 0;
            self.next()
        }
    }
}
