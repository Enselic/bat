use std::path::Path;
use syntect::highlighting::ThemeSet;
use syntect::parsing::{SyntaxSet, SyntaxSetBuilder};

use crate::assets::*;

mod acknowledgements;

pub fn build(
    source_dir: &Path,
    include_integrated_assets: bool,
    target_dir: &Path,
    current_version: &str,
) -> Result<()> {
    let theme_set = build_theme_set(source_dir, include_integrated_assets);

    let syntax_set_builder = build_syntax_set_builder(source_dir, include_integrated_assets)?;

    let syntax_set = syntax_set_builder.build();

    print_unlinked_contexts(&syntax_set);

    write_assets(&theme_set, &syntax_set, target_dir, current_version)
}

fn build_theme_set(source_dir: &Path, include_integrated_assets: bool) -> ThemeSet {
    let mut theme_set = if include_integrated_assets {
        crate::assets::get_integrated_themeset()
    } else {
        ThemeSet::new()
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

    theme_set
}

fn build_syntax_set_builder(
    source_dir: &Path,
    include_integrated_assets: bool,
) -> Result<SyntaxSetBuilder> {
    let mut syntax_set_builder = if !include_integrated_assets {
        let mut builder = syntect::parsing::SyntaxSetBuilder::new();
        builder.add_plain_text_syntax();
        builder
    } else {
        from_binary::<SyntaxSet>(get_serialized_integrated_syntaxset(), COMPRESS_SYNTAXES)
            .into_builder()
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

    Ok(syntax_set_builder)
}

pub fn build_acknowledgements(
    source_dir: &Path,
    include_integrated_assets: bool,
) -> Result<String> {
    // Sourced from License section in README.md
    let preamble = "Copyright (c) 2018-2021 bat-developers (https://github.com/sharkdp/bat).

bat is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.
";

    let acknowledgements = String::new();
    acknowledgements.push_str(preamble);

    // Sort entries so the order is stable over time
    let dir_entries =
        walkdir::WalkDir::new(source_dir).sort_by(|a, b| a.file_name().cmp(b.file_name()));
    for dir_entry in dir_entries {
        let dir_entry = dir_entry.map_err(|| Error::Msg("TODO"))?;
        if dir_entry_is_license(&dir_entry) {
            let syntax = load_syntax_file(dir_entry.path(), lines_include_newline)?;
            if let Some(path_str) = dir_entry.path().to_str() {
                // Split the path up and rejoin with slashes so that syntaxes loaded on Windows
                // can still be loaded the same way.
                let path = Path::new(path_str);
                let path_parts: Vec<_> = path.iter().map(|c| c.to_str().unwrap()).collect();
                self.path_syntaxes
                    .push((path_parts.join("/").to_string(), self.syntaxes.len()));
            }
            self.syntaxes.push(syntax);
        }

        #[cfg(feature = "metadata")]
        {
            if entry.path().extension() == Some("tmPreferences".as_ref()) {
                match RawMetadataEntry::load(entry.path()) {
                    Ok(meta) => self.raw_metadata.add_raw(meta),
                    Err(_err) => (),
                }
            }
        }
    }

    return acknowledgements;
}

fn dir_entry_is_license(dir_entry: &walkdir::DirEntry) -> bool {
    dir_entry.path().file_prefix()
}

fn print_unlinked_contexts(syntax_set: &SyntaxSet) {
    let missing_contexts = syntax_set.find_unlinked_contexts();
    if !missing_contexts.is_empty() {
        println!("Some referenced contexts could not be found!");
        for context in missing_contexts {
            println!("- {}", context);
        }
    }
}

fn write_assets(
    theme_set: &ThemeSet,
    syntax_set: &SyntaxSet,
    target_dir: &Path,
    current_version: &str,
) -> Result<()> {
    let _ = std::fs::create_dir_all(target_dir);
    asset_to_cache(
        theme_set,
        &target_dir.join("themes.bin"),
        "theme set",
        COMPRESS_THEMES,
    )?;
    asset_to_cache(
        syntax_set,
        &target_dir.join("syntaxes.bin"),
        "syntax set",
        COMPRESS_SYNTAXES,
    )?;

    print!(
        "Writing metadata to folder {} ... ",
        target_dir.to_string_lossy()
    );
    crate::assets_metadata::AssetsMetadata::new(current_version).save_to_folder(target_dir)?;
    println!("okay");

    Ok(())
}

fn asset_to_contents<T: serde::Serialize>(
    asset: &T,
    description: &str,
    compressed: bool,
) -> Result<Vec<u8>> {
    let mut contents = vec![];
    if compressed {
        bincode::serialize_into(
            flate2::write::ZlibEncoder::new(&mut contents, flate2::Compression::best()),
            asset,
        )
    } else {
        bincode::serialize_into(&mut contents, asset)
    }
    .map_err(|_| format!("Could not serialize {}", description))?;
    Ok(contents)
}

fn asset_to_cache<T: serde::Serialize>(
    asset: &T,
    path: &Path,
    description: &str,
    compressed: bool,
) -> Result<()> {
    print!("Writing {} to {} ... ", description, path.to_string_lossy());
    let contents = asset_to_contents(asset, description, compressed)?;
    std::fs::write(path, &contents[..]).map_err(|_| {
        format!(
            "Could not save {} to {}",
            description,
            path.to_string_lossy()
        )
    })?;
    println!("okay");
    Ok(())
}
