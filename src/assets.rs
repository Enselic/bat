use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::Path;

use crate::dep_analysis::*;
use syntect::dumps::{dump_binary, dump_to_file, from_binary, from_reader};
use syntect::highlighting::{Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet, SyntaxSetBuilder};

use path_abs::PathAbs;

use crate::assets_metadata::AssetsMetadata;
use crate::bat_warning;
use crate::error::*;
use crate::input::{InputReader, OpenedInput, OpenedInputKind};
use crate::syntax_mapping::{MappingTarget, SyntaxMapping};

use lazycell::LazyCell;

/// A SyntaxSet in a serialized form, i.e. bincoded and flate2 compressed.
/// We keep it in this format since we want to load it lazily.
#[derive(Debug)]
enum SerializedSyntaxSet {
    Owned(Vec<u8>),
    Referenced(&'static [u8]),
}

impl SerializedSyntaxSet {
    fn deserialize(&self) -> SyntaxSet {
        match self {
            SerializedSyntaxSet::Referenced(data) => asset_from_reader(*data, "lazy loaded syntax set").expect("data compiled to binary is not corrupt"),
            _ => panic!("not yet imlpemented"),
        }
    }
}


// #[derive(Debug)]
// enum SyntaxSetForm {
//     Serialized(SerializedSyntaxSet),
//     Deserialized(SyntaxSet),
// }

/// Old comments:
/// Serialized form of `syntax_set`. Not present if we already have a `syntax_set`
/// Lazy-loaded from `serialized_syntax_set`
#[derive(Debug)]
pub enum RawSyntaxes {
    Owned(Vec<u8>),
    Referenced(&'static [u8]),
}

#[derive(Debug)]
pub struct HighlightingAssets {
    /// Invariant: The serialized version of `syntax_set` if present
    serialized_syntax_set: Option<SerializedSyntaxSet>,
    syntax_set: LazyCell<SyntaxSet>,

    lookup: SyntaxesLookup,
    syntaxes: RawSyntaxes,
    loaded_syntax_sets: HashMap<OffsetAndSize, SyntaxSet>,

    pub(crate) theme_set: ThemeSet,

    fallback_theme: Option<&'static str>,
}

const IGNORED_SUFFIXES: [&str; 10] = [
    // Editor etc backups
    "~",
    ".bak",
    ".old",
    ".orig",
    // Debian and derivatives apt/dpkg backups
    ".dpkg-dist",
    ".dpkg-old",
    // Red Hat and derivatives rpm backups
    ".rpmnew",
    ".rpmorig",
    ".rpmsave",
    // Build system input/template files
    ".in",
];

impl HighlightingAssets {

    pub fn new(
        serialized_syntax_set: Option<SerializedSyntaxSet>,
        syntax_set: LazyCell<SyntaxSet>,
        lookup: SyntaxesLookup,
        syntaxes: RawSyntaxes,

    ) -> Self {
        HighlightingAssets {
            serialized_syntax_set,
            syntax_set,
            lookup,
            syntaxes,
            loaded_syntax_sets: HashMap::new(),
            theme_set: ThemeSet,
            fallback_theme: Option<&'static str>,
        }
    }

    pub fn default_theme() -> &'static str {
        "Monokai Extended"
    }

    pub fn from_files(source_dir: &Path, include_integrated_assets: bool) -> Result<Self> {
        let mut theme_set = if include_integrated_assets {
            Self::get_integrated_themeset()
        } else {
            ThemeSet {
                themes: BTreeMap::new(),
            }
        };

        let theme_dir = source_dir.join("themes");
        if theme_dir.exists() {
            let res = theme_set.add_from_folder(&theme_dir);
            if let Err(err) = res {
                println!(
                    "Failed to load one or more themes from '{}' (reason: '{}')",
                    theme_dir.to_string_lossy(),
                    err,
                );
            }
        } else {
            println!(
                "No themes were found in '{}', using the default set",
                theme_dir.to_string_lossy()
            );
        }

        let mut syntax_set_builder = if !include_integrated_assets {
            let mut builder = SyntaxSetBuilder::new();
            builder.add_plain_text_syntax();
            builder
        } else {
            Self::get_integrated_syntaxset().into_builder()
        };

        let syntax_dir = source_dir.join("syntaxes");
        if syntax_dir.exists() {
            syntax_set_builder.add_from_folder(syntax_dir, true)?;
        } else {
            println!(
                "No syntaxes were found in '{}', using the default set.",
                syntax_dir.to_string_lossy()
            );
        }

        let independent_syntaxes = super::dep_analysis::build_independent(&syntax_set_builder);

        eprintln!("");
        eprintln!("");
        eprintln!("");
        eprintln!("The following independent syntax sets were built:");

        let mut data: Vec<u8> = vec![];

        let mut offset = 0;

        let mut lookup = SyntaxesLookup {
            lookup_by_name: HashMap::new(),
            lookup_by_ext: HashMap::new(),
        };

        for syntax_set in independent_syntaxes {
            eprintln!("");

            let size = Self::handle_independent_syntax(&mut lookup, &syntax_set, offset, &mut data);

            // Update offset for next syntax set
            offset += size;
        }

        // Last, add the full fallback SyntaxSet that contains everything
        let full_syntax_set = syntax_set_builder.build();
        // TODO: Use None to mark "full syntax set"
        Self::handle_independent_syntax(&mut lookup, &full_syntax_set, offset, &mut data);

        let syntax_set = LazyCell::new();
        syntax_set.fill(syntax_set_builder.build());

        Ok(HighlightingAssets {
            syntax_set,
            serialized_syntax_set: None,
            lookup,
            syntaxes: RawSyntaxes::Owned(data),
            loaded_syntax_sets: HashMap::new(),
            theme_set,
            fallback_theme: None,
        })
    }

    // TODO: Better name on SyntaxesLookup
    fn handle_independent_syntax(
        lookup_table: &mut SyntaxesLookup,
        syntax_set: &SyntaxSet,
        offset: u64,
        data: &mut Vec<u8>,
    ) -> u64 {
        // bincode this syntax set
        let syntax_set_bin = dump_binary(&syntax_set);
        let size = syntax_set_bin.len() as u64;

        // Remember where in the binary blob we can find it when we need it again
        let offset_and_size = super::dep_analysis::OffsetAndSize { offset, size };

        // Append the binary blob with the data
        data.extend(syntax_set_bin);

        let mut names = vec![];
        let mut extensions = vec![];

        // Map all file extensions to the offset and size that we just stored
        for syntax in syntax_set.syntaxes() {
            names.push(syntax.name.clone());

            lookup_table
                .lookup_by_name
                .insert(syntax.name.clone(), offset_and_size);

            for ext in &syntax.file_extensions {
                extensions.push(ext.clone());

                lookup_table
                    .lookup_by_ext
                    .insert(ext.to_string(), offset_and_size);
            }
        }

        eprintln!(
            "Mapped
        {:?}
        {:?}
        to {:?}",
            names, extensions, offset_and_size
        );

        return size;
    }

    pub fn from_cache(cache_path: &Path) -> Result<Self> {
        let syntax_set = LazyCell::new();
        syntax_set.fill(asset_from_cache(
            &cache_path.join("syntaxes.bin"),
            "syntax set",
        )?);

        Ok(HighlightingAssets {
            // TODO: Load in serialized form
            syntax_set,
            serialized_syntax_set: None,
            lookup: asset_from_cache(&cache_path.join("lookup.bin"), "theme set")?
            theme_set: asset_from_cache(&cache_path.join("themes.bin"), "theme set")?,
            fallback_theme: None,
        })
    }

    fn get_serialized_integrated_syntaxset() -> &'static [u8] {
        include_bytes!("../assets/syntaxes.bin")
    }

    fn get_integrated_syntaxset() -> SyntaxSet {
        from_binary(Self::get_serialized_integrated_syntaxset())
    }

    fn get_integrated_themeset() -> ThemeSet {
        from_binary(include_bytes!("../assets/themes.bin"))
    }

    pub fn from_binary() -> Self {
        let serialized_syntax_set = Some(SerializedSyntaxSet::Referenced(Self::get_serialized_integrated_syntaxset()));
        let theme_set = Self::get_integrated_themeset();

        HighlightingAssets {
            syntax_set: LazyCell::new(),
            serialized_syntax_set,
            theme_set,
            fallback_theme: None,
        }
    }

    pub fn save_to_cache(&self, target_dir: &Path, current_version: &str) -> Result<()> {
        let _ = fs::create_dir_all(target_dir);
        asset_to_cache(&self.theme_set, &target_dir.join("themes.bin"), "theme set")?;
        asset_to_cache(
            self.get_syntax_set(),
            &target_dir.join("syntaxes.bin"),
            "syntax set",
        )?;

        print!(
            "Writing metadata to folder {} ... ",
            target_dir.to_string_lossy()
        );
        AssetsMetadata::new(current_version).save_to_folder(target_dir)?;
        println!("okay");

        Ok(())
    }

    pub fn set_fallback_theme(&mut self, theme: &'static str) {
        self.fallback_theme = Some(theme);
    }

    pub(crate) fn get_syntax_set(&self) -> &SyntaxSet {
        if !self.syntax_set.filled() {
            self.syntax_set.fill(self.serialized_syntax_set.as_ref().unwrap().deserialize());
        }
        self.syntax_set.borrow().unwrap()
        // if let SyntaxSetForm::Serialized(ref serialized_syntax_set) = *self.syntax_set.borrow() {
        //     self.syntax_set.replace(SyntaxSetForm::Deserialized(serialized_syntax_set.deserialize()));
        // }
        // match *self.syntax_set.borrow() {
        //     SyntaxSetForm::Deserialized(ref syntax_set) => syntax_set,
        //     SyntaxSetForm::Serialized(_) => panic!("impossible, we just deserialized"),
        // }
    }

    pub fn syntaxes(&self) -> &[SyntaxReference] {
        self.get_syntax_set().syntaxes()
    }

    pub fn themes(&self) -> impl Iterator<Item = &str> {
        self.theme_set.themes.keys().map(|s| s.as_ref())
    }

    pub fn syntax_for_file_name(
        &self,
        file_name: impl AsRef<Path>,
        mapping: &SyntaxMapping,
    ) -> Option<&SyntaxReference> {
        let file_name = file_name.as_ref();
        match mapping.get_syntax_for(file_name) {
            Some(MappingTarget::MapToUnknown) => None,
            Some(MappingTarget::MapTo(syntax_name)) => {
                self.get_syntax_set().find_syntax_by_name(syntax_name)
            }
            None => self.get_extension_syntax(file_name.as_os_str()),
        }
    }

    pub fn find_syntax_by_name(&self, name: &str) -> Option<&SyntaxReference> {
        let offset_and_size = self.lookup.lookup_by_name.get(name);
        if let Some(offset_and_size) = offset_and_size {
            let OffsetAndSize { offset, size } = *offset_and_size;
            let end = offset + size;
            let ref_to_data = match self.syntaxes {
                RawSyntaxes::Owned(owned) => &owned,
                RawSyntaxes::Referenced(referenced) => referenced,
            };
            let slice_of_syntax_set = &ref_to_data[offset as usize..end as usize];
            let syntax_set = from_binary(slice_of_syntax_set);
            self.loaded_syntax_sets.insert(*offset_and_size, syntax_set);
            return syntax_set.find_syntax_by_name(name);
        }
        // TODO: Make single return point and deduplicate
        return None
    }

    pub(crate) fn get_theme(&self, theme: &str) -> &Theme {
        match self.theme_set.themes.get(theme) {
            Some(theme) => theme,
            None => {
                if theme == "ansi-light" || theme == "ansi-dark" {
                    bat_warning!("Theme '{}' is deprecated, using 'ansi' instead.", theme);
                    return self.get_theme("ansi");
                }
                if !theme.is_empty() {
                    bat_warning!("Unknown theme '{}', using default.", theme)
                }
                &self.theme_set.themes[self.fallback_theme.unwrap_or_else(|| Self::default_theme())]
            }
        }
    }

    pub(crate) fn get_syntax(
        &self,
        language: Option<&str>,
        input: &mut OpenedInput,
        mapping: &SyntaxMapping,
    ) -> Result<&SyntaxReference> {
        if let Some(language) = language {
            self.get_syntax_set()
                .find_syntax_by_token(language)
                .ok_or_else(|| ErrorKind::UnknownSyntax(language.to_owned()).into())
        } else {
            let line_syntax = self.get_first_line_syntax(&mut input.reader);

            // Get the path of the file:
            // If this was set by the metadata, that will take priority.
            // If it wasn't, it will use the real file path (if available).
            let path_str =
                input
                    .metadata
                    .user_provided_name
                    .as_ref()
                    .or_else(|| match input.kind {
                        OpenedInputKind::OrdinaryFile(ref path) => Some(path),
                        _ => None,
                    });

            if let Some(path_str) = path_str {
                // If a path was provided, we try and detect the syntax based on extension mappings.
                let path = Path::new(path_str);
                let absolute_path = PathAbs::new(path)
                    .ok()
                    .map(|p| p.as_path().to_path_buf())
                    .unwrap_or_else(|| path.to_owned());

                match mapping.get_syntax_for(absolute_path) {
                    Some(MappingTarget::MapToUnknown) => line_syntax.ok_or_else(|| {
                        ErrorKind::UndetectedSyntax(path.to_string_lossy().into()).into()
                    }),

                    Some(MappingTarget::MapTo(syntax_name)) => self
                        .get_syntax_set()
                        .find_syntax_by_name(syntax_name)
                        .ok_or_else(|| ErrorKind::UnknownSyntax(syntax_name.to_owned()).into()),

                    None => {
                        let file_name = path.file_name().unwrap_or_default();
                        self.get_extension_syntax(file_name)
                            .or(line_syntax)
                            .ok_or_else(|| {
                                ErrorKind::UndetectedSyntax(path.to_string_lossy().into()).into()
                            })
                    }
                }
            } else {
                // If a path wasn't provided, we fall back to the detect first-line syntax.
                line_syntax.ok_or_else(|| ErrorKind::UndetectedSyntax("[unknown]".into()).into())
            }
        }
    }

    fn get_extension_syntax(&self, file_name: &OsStr) -> Option<&SyntaxReference> {
        self.get_syntax_set()
            .find_syntax_by_extension(file_name.to_str().unwrap_or_default())
            .or_else(|| {
                let file_path = Path::new(file_name);
                self.get_syntax_set()
                    .find_syntax_by_extension(
                        file_path
                            .extension()
                            .and_then(|x| x.to_str())
                            .unwrap_or_default(),
                    )
                    .or_else(|| {
                        if let Some(file_str) = file_path.to_str() {
                            for suffix in IGNORED_SUFFIXES.iter() {
                                if let Some(stripped_filename) = file_str.strip_suffix(suffix) {
                                    return self
                                        .get_extension_syntax(OsStr::new(stripped_filename));
                                }
                            }
                        }
                        None
                    })
            })
    }

    fn find_syntax_by_extension(&self, ext: &str) -> Option<&SyntaxReference> {
        let offset_and_size = self.lookup.lookup_by_ext.get(ext);
        if let Some(offset_and_size) = offset_and_size {
            let OffsetAndSize { offset, size } = *offset_and_size;
            let end = offset + size;
            let ref_to_data = match self.syntaxes {
                RawSyntaxes::Owned(owned) => &owned,
                RawSyntaxes::Referenced(referenced) => referenced,
            };
            let slice_of_syntax_set = &ref_to_data[offset as usize..end as usize];
            let syntax_set = from_binary(slice_of_syntax_set);
            self.loaded_syntax_sets.insert(*offset_and_size, syntax_set);
            return syntax_set.find_syntax_by_extension(ext);
        }
        // TODO: Make single return point and deduplicate
        return None

    }

    fn get_first_line_syntax(&self, reader: &mut InputReader) -> Option<&SyntaxReference> {
        String::from_utf8(reader.first_line.clone())
            .ok()
            .and_then(|l| self.get_syntax_set().find_syntax_by_first_line(&l))
    }
}

fn asset_to_cache<T: serde::Serialize>(asset: &T, path: &Path, description: &str) -> Result<()> {
    print!("Writing {} to {} ... ", description, path.to_string_lossy());
    dump_to_file(asset, &path).chain_err(|| {
        format!(
            "Could not save {} to {}",
            description,
            path.to_string_lossy()
        )
    })?;
    println!("okay");
    Ok(())
}

fn asset_from_cache<T: serde::de::DeserializeOwned>(path: &Path, description: &str) -> Result<T> {
    let asset_file = File::open(&path).chain_err(|| {
        format!(
            "Could not load cached {} '{}'",
            description,
            path.to_string_lossy()
        )
    })?;
    asset_from_reader(BufReader::new(asset_file), description)
}

fn asset_from_reader<T: serde::de::DeserializeOwned, R: std::io::BufRead>(
    input: R,
    description: &str,
) -> Result<T> {
    from_reader(input).chain_err(|| format!("Could not parse {}", description))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::ffi::OsStr;

    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    use crate::input::Input;

    struct SyntaxDetectionTest<'a> {
        assets: HighlightingAssets,
        pub syntax_mapping: SyntaxMapping<'a>,
        pub temp_dir: TempDir,
    }

    impl<'a> SyntaxDetectionTest<'a> {
        fn new() -> Self {
            SyntaxDetectionTest {
                assets: HighlightingAssets::from_binary(),
                syntax_mapping: SyntaxMapping::builtin(),
                temp_dir: TempDir::new().expect("creation of temporary directory"),
            }
        }

        fn syntax_for_real_file_with_content_os(
            &self,
            file_name: &OsStr,
            first_line: &str,
        ) -> String {
            let file_path = self.temp_dir.path().join(file_name);
            {
                let mut temp_file = File::create(&file_path).unwrap();
                writeln!(temp_file, "{}", first_line).unwrap();
            }

            let input = Input::ordinary_file(&file_path);
            let dummy_stdin: &[u8] = &[];
            let mut opened_input = input.open(dummy_stdin, None).unwrap();

            self.assets
                .get_syntax(None, &mut opened_input, &self.syntax_mapping)
                .unwrap_or_else(|_| self.assets.get_syntax_set().find_syntax_plain_text())
                .name
                .clone()
        }

        fn syntax_for_file_with_content_os(&self, file_name: &OsStr, first_line: &str) -> String {
            let file_path = self.temp_dir.path().join(file_name);
            let input = Input::from_reader(Box::new(BufReader::new(first_line.as_bytes())))
                .with_name(Some(&file_path));
            let dummy_stdin: &[u8] = &[];
            let mut opened_input = input.open(dummy_stdin, None).unwrap();

            self.assets
                .get_syntax(None, &mut opened_input, &self.syntax_mapping)
                .unwrap_or_else(|_| self.assets.get_syntax_set().find_syntax_plain_text())
                .name
                .clone()
        }

        #[cfg(unix)]
        fn syntax_for_file_os(&self, file_name: &OsStr) -> String {
            self.syntax_for_file_with_content_os(file_name, "")
        }

        fn syntax_for_file_with_content(&self, file_name: &str, first_line: &str) -> String {
            self.syntax_for_file_with_content_os(OsStr::new(file_name), first_line)
        }

        fn syntax_for_file(&self, file_name: &str) -> String {
            self.syntax_for_file_with_content(file_name, "")
        }

        fn syntax_for_stdin_with_content(&self, file_name: &str, content: &[u8]) -> String {
            let input = Input::stdin().with_name(Some(file_name));
            let mut opened_input = input.open(content, None).unwrap();

            self.assets
                .get_syntax(None, &mut opened_input, &self.syntax_mapping)
                .unwrap_or_else(|_| self.assets.get_syntax_set().find_syntax_plain_text())
                .name
                .clone()
        }

        fn syntax_is_same_for_inputkinds(&self, file_name: &str, content: &str) -> bool {
            let as_file = self.syntax_for_real_file_with_content_os(file_name.as_ref(), content);
            let as_reader = self.syntax_for_file_with_content_os(file_name.as_ref(), content);
            let consistent = as_file == as_reader;
            // TODO: Compare StdIn somehow?

            if !consistent {
                eprintln!(
                    "Inconsistent syntax detection:\nFor File: {}\nFor Reader: {}",
                    as_file, as_reader
                )
            }

            consistent
        }
    }

    #[test]
    fn syntax_detection_basic() {
        let test = SyntaxDetectionTest::new();

        assert_eq!(test.syntax_for_file("test.rs"), "Rust");
        assert_eq!(test.syntax_for_file("test.cpp"), "C++");
        assert_eq!(test.syntax_for_file("test.build"), "NAnt Build File");
        assert_eq!(
            test.syntax_for_file("PKGBUILD"),
            "Bourne Again Shell (bash)"
        );
        assert_eq!(test.syntax_for_file(".bashrc"), "Bourne Again Shell (bash)");
        assert_eq!(test.syntax_for_file("Makefile"), "Makefile");
    }

    #[cfg(unix)]
    #[test]
    fn syntax_detection_invalid_utf8() {
        use std::os::unix::ffi::OsStrExt;

        let test = SyntaxDetectionTest::new();

        assert_eq!(
            test.syntax_for_file_os(OsStr::from_bytes(b"invalid_\xFEutf8_filename.rs")),
            "Rust"
        );
    }

    #[test]
    fn syntax_detection_same_for_inputkinds() {
        let mut test = SyntaxDetectionTest::new();

        test.syntax_mapping
            .insert("*.myext", MappingTarget::MapTo("C"))
            .ok();
        test.syntax_mapping
            .insert("MY_FILE", MappingTarget::MapTo("Markdown"))
            .ok();

        assert!(test.syntax_is_same_for_inputkinds("Test.md", ""));
        assert!(test.syntax_is_same_for_inputkinds("Test.txt", "#!/bin/bash"));
        assert!(test.syntax_is_same_for_inputkinds(".bashrc", ""));
        assert!(test.syntax_is_same_for_inputkinds("test.h", ""));
        assert!(test.syntax_is_same_for_inputkinds("test.js", "#!/bin/bash"));
        assert!(test.syntax_is_same_for_inputkinds("test.myext", ""));
        assert!(test.syntax_is_same_for_inputkinds("MY_FILE", ""));
        assert!(test.syntax_is_same_for_inputkinds("MY_FILE", "<?php"));
    }

    #[test]
    fn syntax_detection_well_defined_mapping_for_duplicate_extensions() {
        let test = SyntaxDetectionTest::new();

        assert_eq!(test.syntax_for_file("test.h"), "C++");
        assert_eq!(test.syntax_for_file("test.sass"), "Sass");
        assert_eq!(test.syntax_for_file("test.js"), "JavaScript (Babel)");
        assert_eq!(test.syntax_for_file("test.fs"), "F#");
        assert_eq!(test.syntax_for_file("test.v"), "Verilog");
    }

    #[test]
    fn syntax_detection_first_line() {
        let test = SyntaxDetectionTest::new();

        assert_eq!(
            test.syntax_for_file_with_content("my_script", "#!/bin/bash"),
            "Bourne Again Shell (bash)"
        );
        assert_eq!(
            test.syntax_for_file_with_content("build", "#!/bin/bash"),
            "Bourne Again Shell (bash)"
        );
        assert_eq!(
            test.syntax_for_file_with_content("my_script", "<?php"),
            "PHP"
        );
    }

    #[test]
    fn syntax_detection_with_custom_mapping() {
        let mut test = SyntaxDetectionTest::new();

        assert_eq!(test.syntax_for_file("test.h"), "C++");
        test.syntax_mapping
            .insert("*.h", MappingTarget::MapTo("C"))
            .ok();
        assert_eq!(test.syntax_for_file("test.h"), "C");
    }

    #[test]
    fn syntax_detection_is_case_sensitive() {
        let mut test = SyntaxDetectionTest::new();

        assert_ne!(test.syntax_for_file("README.MD"), "Markdown");
        test.syntax_mapping
            .insert("*.MD", MappingTarget::MapTo("Markdown"))
            .ok();
        assert_eq!(test.syntax_for_file("README.MD"), "Markdown");
    }

    #[test]
    fn syntax_detection_stdin_filename() {
        let test = SyntaxDetectionTest::new();

        // from file extension
        assert_eq!(test.syntax_for_stdin_with_content("test.cpp", b"a"), "C++");
        // from first line (fallback)
        assert_eq!(
            test.syntax_for_stdin_with_content("my_script", b"#!/bin/bash"),
            "Bourne Again Shell (bash)"
        );
    }

    #[cfg(unix)]
    #[test]
    fn syntax_detection_for_symlinked_file() {
        use std::os::unix::fs::symlink;

        let test = SyntaxDetectionTest::new();
        let file_path = test.temp_dir.path().join("my_ssh_config_filename");
        {
            File::create(&file_path).unwrap();
        }
        let file_path_symlink = test.temp_dir.path().join(".ssh").join("config");

        std::fs::create_dir(test.temp_dir.path().join(".ssh"))
            .expect("creation of directory succeeds");
        symlink(&file_path, &file_path_symlink).expect("creation of symbolic link succeeds");

        let input = Input::ordinary_file(&file_path_symlink);
        let dummy_stdin: &[u8] = &[];
        let mut opened_input = input.open(dummy_stdin, None).unwrap();

        assert_eq!(
            test.assets
                .get_syntax(None, &mut opened_input, &test.syntax_mapping)
                .unwrap_or_else(|_| test.assets.get_syntax_set().find_syntax_plain_text())
                .name,
            "SSH Config"
        );
    }
}
