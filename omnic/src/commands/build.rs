//! `omnic build` — compile .omni source to target language(s).
//! Mirrors Python cmd_build (cli.py:144).

use std::path::Path;

use crate::api;
use crate::commands::common::{extract_error_messages, write_output_files};

pub fn cmd_build(
    files: Vec<String>,
    target: Option<String>,
    output_dir: &str,
    store_mode: &str,
) -> anyhow::Result<()> {
    let sources = api::collect_sources(&files)?;
    let target_str = api::detect_target(&sources, target.as_deref());
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&sources);

    let t: omnilang::codegen::Target = target_str
        .parse()
        .map_err(|_| anyhow::anyhow!("unknown target: {target_str}"))?;

    let store_mode_opt = if store_mode == "hooks" { None } else { Some(store_mode) };
    let result_json = omnilang::compile_multi_to_target(sources, t, path_mapping, false, false, store_mode_opt);
    let data: serde_json::Value = serde_json::from_str(&result_json)?;

    // Python checks result.success (cli.py:173)
    if !data["success"].as_bool().unwrap_or(false) {
        let msg = extract_error_messages(&data["errors"]);
        anyhow::bail!("{msg}");
    }

    // Write output files — mirrors Python cli.py:179-187
    let output_path = Path::new(output_dir);
    std::fs::create_dir_all(output_path)?;

    let count = write_output_files(output_path, data["files"].as_object())?;
    if count > 0 {
        println!("Compiled {count} file(s) to {target_str} successfully.");
    }

    Ok(())
}
