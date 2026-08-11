//! `omnic build` — compile .omni source to target language(s).
//! Mirrors Python cmd_build (cli.py:144).

use std::path::Path;

use crate::api;
use crate::commands::common::{extract_error_messages, print_warnings, write_output_files};

pub fn cmd_build(
    files: Vec<String>,
    target: Option<String>,
    output_dir: &str,
) -> anyhow::Result<()> {
    let sources = api::collect_sources(&files)?;
    let target_str = api::detect_target(&sources, target.as_deref());
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);

    let t: omnilang::codegen::Target = target_str
        .parse()
        .map_err(|_| anyhow::anyhow!("unknown target: {target_str}"))?;

    let result_json =
        omnilang::compile_multi_to_target(sources, t, path_mapping, false, false, None);
    let data: serde_json::Value = serde_json::from_str(&result_json)?;

    // Python checks result.success (cli.py:173)
    if !data["success"].as_bool().unwrap_or(false) {
        let msg = extract_error_messages(&data["errors"]);
        anyhow::bail!("{msg}");
    }

    // Surface warnings (lint rules + codegen) on stderr without failing the
    // build — mirrors Python cli.py behavior.
    print_warnings(&data["warnings"]);

    // Write output files — mirrors Python cli.py:179-187
    let output_path = Path::new(output_dir);
    std::fs::create_dir_all(output_path)?;

    let count = write_output_files(output_path, data["files"].as_object())?;
    if count > 0 {
        println!("Compiled {count} file(s) to {target_str} successfully.");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A warning-producing project must build successfully and still emit
    /// output files (warnings go to stderr, they must not fail the build).
    #[test]
    fn cmd_build_surfaces_warnings_without_failing() {
        let tmp = std::env::temp_dir().join(format!("omnic-build-warnings-{}", std::process::id()));
        let src_dir = tmp.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let main = src_dir.join("main.omni");
        // `unused` signal is never referenced → dead-code lint warning.
        // `import omniview;` is required for UI components.
        std::fs::write(
            &main,
            "import omniview;\ncomponent App { signal unused: i32 = 0; view { <div /> } }",
        )
        .unwrap();

        let out_dir = tmp.join("out");
        let files = vec![main.to_string_lossy().to_string()];
        let result = cmd_build(
            files,
            Some("omniview".to_string()),
            &out_dir.to_string_lossy(),
        );
        assert!(
            result.is_ok(),
            "build with warnings must succeed, got: {result:?}"
        );
        let written = std::fs::read_dir(&out_dir).unwrap().count();
        assert!(written > 0, "expected generated output files");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// W-301 (unsupported selector) is a warning — build must succeed
    /// (honest degradation, ADR-054) and still emit output files. The
    /// warning's stderr text is asserted at the core level
    /// (validate_style_selectors tests); here we pin the build-succeeds
    /// contract end to end (批次 3, spec §2.3).
    #[test]
    fn cmd_build_surfaces_w301_without_failing() {
        let tmp = std::env::temp_dir().join(format!("omnic-build-w301-{}", std::process::id()));
        let src_dir = tmp.join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let main = src_dir.join("main.omni");
        // `input:checked + span` — sibling selector: parser truncates at `+`
        // and records UnsupportedSelector (W-301 at the semantic layer).
        std::fs::write(
            &main,
            "import omniview;\ncomponent App { style { input:checked + span { color: red; } } view { <div /> } }",
        )
        .unwrap();

        let out_dir = tmp.join("out");
        let files = vec![main.to_string_lossy().to_string()];
        let result = cmd_build(
            files,
            Some("omniview".to_string()),
            &out_dir.to_string_lossy(),
        );
        assert!(
            result.is_ok(),
            "build with W-301 must succeed (warning non-blocking), got: {result:?}"
        );
        let written = std::fs::read_dir(&out_dir).unwrap().count();
        assert!(written > 0, "expected generated output files");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
