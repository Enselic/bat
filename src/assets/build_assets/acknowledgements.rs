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
            let license_text = std::fs::read_to_string(Path::new(entry.path()))?;
            if license_requires_attribution(&license_text) {
                // Most license texts wrap at 80 chars so our horizontal divider is 80 chars
                acknowledgements.push_str("――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――――\n");
                acknowledgements.push_str(&format!("{:?}:\n", &entry.path()));
                acknowledgements.push_str(&license_text);
                if acknowledgements
                    .chars()
                    .last()
                    .expect("string is not empty")
                    != '\n'
                {
                    acknowledgements.push('\n');
                }
            } else {
            }
        }
    }

    return Ok(Some(acknowledgements));
}

fn dir_entry_is_license(entry: &walkdir::DirEntry) -> bool {
    return if let Some(Some(stem)) = entry.path().file_stem().map(|s| s.to_str()) {
        let uppercase_stem = stem.to_ascii_uppercase();
        uppercase_stem == "LICENSE" || uppercase_stem == "NOTICE"
    } else {
        false
    };
}

fn license_requires_attribution(license_text: &str) -> bool {
    // TODO: Replace newline with ' '
    let markers = vec![
        "Redistributions in binary form must reproduce the above",
        // MIT
        "The above copyright notice and this permission notice shall be included in
all copies or substantial portions of the Software.",
    ];
    for marker in markers {
        if license_text.contains(marker) {
            return true;
        }
    }
    return false;
}

/// Replaces newlines with a space character, and replaces multiple spaces with one space.
/// This makes the text easier to analyze.
fn normalize_license_text(license_text: &str) -> Result<String> {
    let multiple_spaces = regex::Regex::new(" +").map_err(|e| format!("{}", e))?;
    multiple_spaces.replace(license_text.replace("\n", " "), " ")?
}

// TODO: Tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_license_text() {
        let license_text = "This is a license text with these terms:
 * Complicated multi-line
   term with indentation";

        assert_eq!(
            "This is a license text with these terms: * Complicated multi-line term with indentation".to_owned(),
            normalize_license_text(license_text),
        );
    }
}
