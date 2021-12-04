use std::path::Path;

use crate::error::*;

pub fn build(source_dir: &Path, include_integrated_assets: bool) -> Result<Option<String>> {
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
            // Most license texts wrap at 80 chars so our horizontal divider is 80 chars
            acknowledgements.push_str("――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――\n");
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
