use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "omnic")]
#[command(about = "OmniLang v0.2 compiler — cross-language transpiler")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile .omni source to a target language
    Build {
        /// Source file
        file: String,
        /// Target language: rust, dart, js, python, all
        #[arg(short, long, default_value = "all")]
        target: String,
        /// Output directory
        #[arg(short, long, default_value = "build")]
        output: String,
    },
    /// Run static analysis
    Check {
        /// Source file
        file: String,
        /// Output format: human, json, github
        #[arg(long, default_value = "human")]
        format: String,
    },
}

fn compile_file(source: &str, file: &str, target: &str) -> anyhow::Result<String> {
    omnilang_core::compile(source, file, target)
        .map_err(|e| anyhow::anyhow!("{}", e))
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Build { file, target, output } => {
            println!("Building {} -> {} (output: {})", file, target, output);
            let source = std::fs::read_to_string(&file)?;

            if target == "all" {
                for t in &["python", "js", "dart", "rust"] {
                    let code = compile_file(&source, &file, t)?;
                    let ext = match *t {
                        "python" => "py",
                        "js" => "js",
                        "dart" => "dart",
                        "rust" => "rs",
                        _ => "out",
                    };
                    let outpath = format!("{}/{}.{}", output, file.replace(".omni", ""), ext);
                    std::fs::create_dir_all(&output)?;
                    std::fs::write(&outpath, code)?;
                    println!("  -> {}", outpath);
                }
            } else {
                let code = compile_file(&source, &file, &target)?;
                let ext = match target.as_str() {
                    "python" | "py" => "py",
                    "js" | "javascript" => "js",
                    "dart" => "dart",
                    "rust" | "rs" => "rs",
                    _ => "out",
                };
                let outpath = format!("{}/{}.{}", output, file.replace(".omni", ""), ext);
                std::fs::create_dir_all(&output)?;
                std::fs::write(&outpath, code)?;
                println!("  -> {}", outpath);
            }
        }
        Command::Check { file, format: _ } => {
            println!("Checking {}...", file);
            let source = std::fs::read_to_string(&file)?;
            let program = omnilang_core::parser::parse(&source, &file)
                .map_err(|e| anyhow::anyhow!(e.report(None)))?;
            println!("{} parsed successfully ({} functions, {} structs)",
                file, program.functions.len(), program.structs.len());
        }
    }

    Ok(())
}
