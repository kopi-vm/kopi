use regex::Regex;
use std::path::Path;
use walkdir::WalkDir;

#[test]
fn kopi_home_paths_use_registry_helpers() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src_root = repo_root.join("src");
    let paths_root = src_root.join("paths");

    let forbidden_segments = [
        ("jdks", "paths::home::jdks_dir(...)"),
        ("cache", "paths::home::cache_dir(...)"),
        ("shims", "paths::home::shims_dir(...)"),
        ("bin", "paths::home::bin_dir(...)"),
        ("locks", "paths::home::locks_dir(...)"),
    ];

    let patterns: Vec<(Regex, &str, &str)> = forbidden_segments
        .iter()
        .map(|(segment, guidance)| {
            let pattern = format!(r#"\.(join|push)\(\s*"{}""#, regex::escape(segment));
            (
                Regex::new(&pattern).expect("valid regex"),
                *segment,
                *guidance,
            )
        })
        .collect();

    let mut violations = Vec::new();

    for entry in WalkDir::new(&src_root).into_iter() {
        let entry = entry.expect("walkdir traversal");
        let path = entry.path();

        if path.starts_with(&paths_root) {
            continue;
        }

        if entry.file_type().is_dir() {
            continue;
        }

        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }

        let contents = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("Failed to read {}: {err}", path.display()));

        for (regex, segment, guidance) in &patterns {
            if let Some(mat) = regex.find(&contents) {
                let relative = path
                    .strip_prefix(repo_root)
                    .unwrap_or(path)
                    .display()
                    .to_string();

                let method = if mat.as_str().contains(".join") {
                    "join"
                } else {
                    "push"
                };

                violations.push(format!(
                    "{relative} uses a hard-coded .{method}(\"{segment}\"). Import {guidance} instead."
                ));
            }
        }
    }

    if !violations.is_empty() {
        panic!(
            "Kopi home path helpers must be used instead of hard-coded joins:\n{}",
            violations.join("\n")
        );
    }
}
