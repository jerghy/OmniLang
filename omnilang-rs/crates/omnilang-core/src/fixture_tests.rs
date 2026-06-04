/// Auto-discover and test all .omni fixture files.
/// Mirrors tests/test_fixtures.py
use crate::{parser, semantic, analyze};
use std::collections::HashMap;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    for c in &["../../../tests/fixtures", "../../tests/fixtures", "../tests/fixtures"] {
        let p = PathBuf::from(c);
        if p.exists() { return p; }
    }
    panic!("Fixtures dir not found");
}

fn discover_fixtures(include_errors: bool) -> Vec<(PathBuf, String, String)> {
    let mut fixtures = vec![];
    let base = fixtures_dir();
    walk_fixtures(&base, &base, &mut fixtures, include_errors);
    fixtures
}

fn walk_fixtures(base: &PathBuf, dir: &PathBuf, out: &mut Vec<(PathBuf, String, String)>,
                 include_errors: bool) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut files: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        files.sort_by_key(|e| e.file_name());
        for entry in files {
            let path = entry.path();
            if path.is_dir() {
                let cat = path.file_name().unwrap().to_string_lossy().to_string();
                if !include_errors && cat == "errors" { continue; }
                walk_fixtures(base, &path, out, include_errors);
            } else if path.extension().map_or(false, |e| e == "omni") {
                let rel = path.strip_prefix(base).unwrap_or(&path);
                let cat = rel.parent().and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "root".into());
                let name = rel.to_string_lossy().replace('\\', "_").replace('/', "_").replace(".omni", "");
                out.push((path, name, cat));
            }
        }
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;

    #[test]
    fn test_all_fixtures_parse() {
        let fixtures = discover_fixtures(false);
        let count = fixtures.len();
        assert!(count >= 80, "Expected at least 80 fixtures, got {}", count);

        let mut pass = 0;
        let mut fails = vec![];
        for (path, name, _cat) in &fixtures {
            crate::ast::reset_node_ids();
            let src = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => { fails.push((name.clone(), format!("read: {}", e))); continue; }
            };
            match parser::parse(&src, &path.to_string_lossy()) {
                Ok(p) => {
                    assert!(p.functions.iter().all(|f| !f.name.is_empty()),
                        "function with empty name in {}", name);
                    pass += 1;
                }
                Err(e) => {
                    fails.push((name.clone(), e.report(None)));
                }
            }
        }

        if !fails.is_empty() {
            eprintln!("\n=== {} fixtures failed to parse ===", fails.len());
            for (name, err) in fails.iter().take(30) {
                eprintln!("  {}: {}", name, err.lines().next().unwrap_or("?"));
            }
            panic!("{}/{} fixtures failed to parse", fails.len(), count);
        }
        eprintln!("All {}/{} fixtures parsed successfully (100%)", pass, count);
    }

    #[test]
    fn test_analyze_fixtures_dont_crash_rules() {
        let fixtures: Vec<_> = discover_fixtures(false).into_iter()
            .filter(|(_, _, cat)| matches!(cat.as_str(), "analyze" | "portability" | "interface"))
            .collect();
        if fixtures.is_empty() { return; }

        let registry = analyze::rules::build_default_registry();
        let mut crashes = vec![];

        for (path, name, _cat) in &fixtures {
            crate::ast::reset_node_ids();
            let src = match std::fs::read_to_string(path) { Ok(s) => s, Err(_) => continue };
            let prog = match parser::parse(&src, &path.to_string_lossy()) {
                Ok(p) => p,
                Err(_) => continue, // parse failures are tested elsewhere
            };
            let sl: HashMap<String, Vec<String>> = HashMap::from([
                (path.to_string_lossy().to_string(),
                 src.lines().map(|l| l.to_string()).collect())
            ]);
            let (tm, rm) = match semantic::analyze(&prog, Some(&sl)) {
                Ok(r) => r,
                Err(_) => continue,
            };
            let ctx = analyze::registry::AnalyzeContext {
                program: &prog,
                source_lines: sl,
                targets: vec!["python".into(), "js".into(), "dart".into(), "rust".into()],
                type_map: &tm,
                rename_map: &rm,
                capture_map: HashMap::new(),
            };
            for rule in registry.list_all() {
                let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _diags = rule.check(&ctx);
                })).map_err(|_| {
                    crashes.push((name.clone(), rule.code().to_string()));
                });
            }
        }

        if !crashes.is_empty() {
            eprintln!("\n=== {} rule crashes on fixtures ===", crashes.len());
            for (name, code) in crashes.iter().take(20) {
                eprintln!("  {}: rule {} crashed", name, code);
            }
            panic!("{} rule(s) crashed on fixtures", crashes.len());
        }
        eprintln!("All analyze fixtures pass without rule crashes ({} fixtures, {} rules)",
            fixtures.len(), registry.count());
    }

    #[test]
    fn test_fixture_count() {
        let fixtures = discover_fixtures(false);
        let count = fixtures.len();
        eprintln!("Total fixtures discovered: {}", count);
        assert!(count >= 80, "Expected at least 80 fixtures, got {}", count);
    }
}
