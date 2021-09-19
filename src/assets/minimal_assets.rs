use std::collections::HashMap;

use lazycell::LazyCell;

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
            .and_then(|index| self.get_syntax_set_with_index(*index))
    }

    pub fn get_syntax_set_by_extension(&self, extension: &str) -> Option<&SyntaxSet> {
        self.minimal_syntaxes
            .by_file_extension
            .get(&extension.to_ascii_lowercase())
            .and_then(|index| self.get_syntax_set_with_index(*index))
    }

    fn find_syntax_by_name(&self, name: &str) -> Result<Option<SyntaxReferenceInSet>> {
        Ok(match self.get_syntax_set_by_name(name) {
            Some(syntax_set) => syntax_set.find_syntax_by_name(name).map(|syntax| SyntaxReferenceInSet { syntax, syntax_set }),
            None => None,
        })
    }

    fn find_syntax_by_extension(&self, e: Option<&OsStr>) -> Result<Option<SyntaxReferenceInSet>> {
           let extension = e.and_then(|x| x.to_str()).unwrap_or_default();

        Ok(match self.get_syntax_set_by_extension(name) {
            Some(syntax_set) => syntax_set.find_syntax_by_extension(name).map(|syntax| SyntaxReferenceInSet { syntax, syntax_set }),
            None => None,
        })
    }

    fn get_extension_syntax(&self, file_name: &OsStr) -> Result<Option<SyntaxReferenceInSet>> {
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
        match self.get_syntax_set_by_name(language)? {
            None => self.get_syntax_set_by_file_extension(language),
            syntax_set => Ok(syntax_set),
        }
    }
 
    pub fn get_extension_syntax(&self, file_name: &OsStr) -> Result<Option<SyntaxReferenceInSet>> {
        let mut syntax = self.find_syntax_by_extension(file_name.to_str().unwrap_or_default())?;
        if syntax.is_none() {
            syntax = self.find_syntax_by_extension(
                Path::new(file_name)
                    .extension()
                    .and_then(|x| x.to_str())
                    .unwrap_or_default(),
            )?;
        }
        if syntax.is_none() {
            syntax = try_with_stripped_suffix(file_name, |stripped_file_name| {
                self.get_extension_syntax(stripped_file_name) // Note: recursion
            })?;
        }
        Ok(syntax)
    }
 
    fn find_syntax_by_extension(&self, extension: &str) -> Result<Option<SyntaxReferenceInSet>> {
        match self.get_syntax_set_by_file_extension(extension)? {
            Some(syntax_set) => Ok(syntax_set
                .find_syntax_by_extension(extension)
                .map(|syntax| SyntaxReferenceInSet { syntax, syntax_set })),
            None => Ok(None),
        }
    }
    pub fn get_syntax_set_by_name(&self, name: &str) -> Result<Option<&SyntaxSet>> {
        self.index_to_syntax_set(
            self.minimal_syntaxes
                .by_name
                .get(&name.to_ascii_lowercase()),
        )
    }
 
    pub fn get_syntax_set_by_file_extension(&self, extension: &str) -> Result<Option<&SyntaxSet>> {
        self.index_to_syntax_set(
            self.minimal_syntaxes
                .by_file_extension
                .get(&extension.to_ascii_lowercase()),
        )

            /*
                let l = String::from_utf8(reader.first_line.clone()).map_err(|e| format!("{}", e))?;
            let s = &l;
 
            for (index, first_line_matches) in
                self.minimal_syntaxes.by_first_line_match.iter().enumerate()
            {
                for first_line_match in first_line_matches {
                    // TODO: cache?
                    let regex = Regex::new(first_line_match.into());
                    if regex.search(s, 0, s.len(), None) {
                        let syntax_set = self.get_minimal_syntax_set_with_index(index)?;
                        return Ok(syntax_set
                            .find_syntax_by_first_line(s)
                            .map(|syntax| SyntaxReferenceInSet { syntax, syntax_set }));
                    }
                }
            }
    */


    fn get_first_line_syntax(
        &self,
        reader: &mut InputReader,
    ) -> Result<Option<SyntaxReferenceInSet>> {
        match String::from_utf8(reader.first_line.clone()).ok() {
            Some(line) => self.find_syntax_by_first_line(&line),
            None => Ok(None),
        }
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
}
