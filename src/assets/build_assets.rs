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

    let acknowledgements = build_acknowledgements(source_dir, include_integrated_assets)?;

    print_unlinked_contexts(&syntax_set);

    write_assets(
        &theme_set,
        &syntax_set,
        acknowledgements.as_deref(),
        target_dir,
        current_version,
    )
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
) -> Result<Option<String>> {
    // TODO: Special flag --build-acknowledgements?
    if include_integrated_assets {
        return Ok(None);
    }

    // Sourced from License section in README.md
    let preamble = "Copyright (c) 2018-2021 bat-developers (https://github.com/sharkdp/bat).

bat is made available under the terms of either the MIT License or the Apache License 2.0, at your option.

See the LICENSE-APACHE and LICENSE-MIT files for license details.
";

    let mut acknowledgements = String::new();
    acknowledgements.push_str(preamble);

    // Sort entries so the order is stable over time
    let entries =
        walkdir::WalkDir::new(source_dir).sort_by(|a, b| a.file_name().cmp(b.file_name()));
    for entry in entries {
        let entry = entry.map_err(|e| Error::Msg(format!("{}", e)))?;
        if dir_entry_is_license(&entry) {
            let contents = std::fs::read_to_string(Path::new(entry.path()))?;
            acknowledgements.push_str("\n");
            acknowledgements.push_str(&contents);
        }
    }

    return Ok(Some(acknowledgements));
}

fn dir_entry_is_license(entry: &walkdir::DirEntry) -> bool {
    return if let Some(Some(stem)) = entry.path().file_stem().map(|s| s.to_str()) {
        stem.to_ascii_uppercase() == "LICENSE"
    } else {
        false
    };
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
    acknowledgements: Option<&str>,
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
    if let Some(acknowledgements) = acknowledgements {
        std::fs::write(&target_dir.join("acknowledgements.bin"), acknowledgements)?;
        // asset_to_cache(
        //     acknowledgements,
        //     ,
        //     "acknowledgements",
        //     true,
        // )?;
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_is_license() {
        //assert!(dir_entry_is_license(walkdir::DirEntry::from_path))
    }
}
