//! `omnic watch` — watch .omni files and auto-rebuild on changes.
//! Mirrors Python cmd_watch (cli.py:477).

use std::path::Path;

use crate::api;
use crate::commands::common::validate_output_filename;

pub fn cmd_watch(file: &str, target: &str, output_dir: &str) -> anyhow::Result<()> {
    let filepath = Path::new(file);
    if !filepath.exists() {
        anyhow::bail!("File not found: {file}");
    }

    // Collect ALL project sources for multi-file import support
    let base_sources = api::collect_sources(&[])?;
    let path_mapping = omnilang::path_mapping::derive_path_mapping(&base_sources);

    let t: omnilang::codegen::Target = target
        .parse()
        .map_err(|_| anyhow::anyhow!("unknown target: {target}"))?;

    println!("Watching {file} (target={target}, output={output_dir})");
    println!("Press Ctrl+C to stop.\n");

    let mut last_mtime = std::fs::metadata(filepath)
        .ok()
        .and_then(|m| m.modified().ok());

    let mut build_count: u64 = 0;
    let mut cache = omnilang::analyse::IncrementalCache::new();

    loop {
        std::thread::sleep(std::time::Duration::from_millis(500));

        let current = std::fs::metadata(filepath)
            .ok()
            .and_then(|m| m.modified().ok());

        if current == last_mtime {
            continue;
        }
        last_mtime = current;
        build_count += 1;

        let timestamp = {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default();
            let secs = now.as_secs() % 86400;
            format!(
                "{:02}:{:02}:{:02}",
                secs / 3600,
                (secs % 3600) / 60,
                secs % 60
            )
        };

        // Read the watched file's current content
        let source = match std::fs::read_to_string(filepath) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[{timestamp}] #{build_count} Read error: {e}");
                continue;
            }
        };

        // Update sources with the changed file's content
        let mut sources = base_sources.clone();
        sources.insert(file.to_string(), source);

        // Use the cached compilation path — skips re-parsing unchanged files
        let result = omnilang::compile_multi_internal_cached(
            sources,
            t,
            path_mapping.clone(),
            false,
            false,
            None,
            &mut cache,
        );

        if result.success {
            // Write output files
            std::fs::create_dir_all(output_dir)?;
            let file_count = result.files.len();
            for (fname, content) in &result.files {
                if let Err(e) = validate_output_filename(fname) {
                    eprintln!("[{timestamp}] #{build_count} Invalid filename: {e}");
                    continue;
                }
                let out_path = Path::new(output_dir).join(fname);
                if let Some(parent) = out_path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        eprintln!("[{timestamp}] #{build_count} Create dir error: {e}");
                        continue;
                    }
                }
                if let Err(e) = std::fs::write(&out_path, content) {
                    eprintln!("[{timestamp}] #{build_count} Write error: {e}");
                    continue;
                }
            }
            println!(
                "[{timestamp}] #{build_count} Build succeeded — {file_count} file(s) -> {output_dir}/"
            );
        } else {
            let msg = result
                .errors
                .iter()
                .map(|d| d.message.as_str())
                .collect::<Vec<_>>()
                .join("; ");
            println!("[{timestamp}] #{build_count} {msg}");
        }
    }
}
