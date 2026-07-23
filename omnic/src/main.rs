//! OmniLang CLI — `omnic` binary.
//!
//! Exact Rust port of the Python `omnilang/cli.py` reference implementation.
//!
//! Commands:
//!   build    — compile .omni source to target language(s)
//!   analyse  — static analysis on .omni source
//!   run      — compile and run an OmniLang file
//!   create   — scaffold a new OmniLang project
//!   watch    — watch .omni files and auto-rebuild on changes

mod api;
mod commands;

use clap::{Parser, Subcommand};

// Mirrors Python ALL_TARGETS in cli.py:27
const ALL_TARGETS: &[&str] = &[
    "python", "js", "ts", "rust", "dart", "react", "vue", "dioxus", "flutter", "omniview",
];

#[derive(Parser)]
#[command(name = "omnic")]
#[command(version = "0.2.0")]
#[command(
    about = "OmniLang compiler — transpile .omni to Python, JS, TS, Rust, Dart, React, Vue, and more"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile .omni source file(s) to target language(s)
    Build {
        /// Source file(s) (.omni) or directory (default: auto-discover src/)
        files: Vec<String>,
        /// Target language/framework
        #[arg(short, long, value_parser = clap::builder::PossibleValuesParser::new(ALL_TARGETS))]
        target: Option<String>,
        /// Output directory (default: ./build)
        #[arg(short, long, default_value = "./build")]
        output: String,
        /// Store state management mode (react target only)
        #[arg(long, default_value = "hooks", value_parser = clap::builder::PossibleValuesParser::new(["hooks", "zustand"]))]
        store_mode: String,
    },

    /// Run static analysis on .omni source
    Analyse {
        /// Source file(s) (.omni) or directory (default: auto-discover src/)
        files: Vec<String>,
        /// Output format
        #[arg(long, default_value = "terminal", value_parser = clap::builder::PossibleValuesParser::new(["terminal", "json"]))]
        format: String,
    },

    /// Compile and run an OmniLang file
    Run {
        /// Source file (.omni) — defaults to src/main.omni when in project root
        file: Option<String>,
        /// Target (auto-detected if omitted)
        #[arg(short, long, value_parser = clap::builder::PossibleValuesParser::new(ALL_TARGETS))]
        target: Option<String>,
    },

    /// Scaffold a new OmniLang project
    Create {
        /// Project name
        name: String,
        /// Project type (required): logic or view
        #[arg(short, long, required = true, value_parser = clap::builder::PossibleValuesParser::new(["logic", "view"]))]
        r#type: String,
    },

    /// Watch .omni files and auto-rebuild on changes
    Watch {
        /// Source file (.omni)
        file: String,
        /// Target (default: react)
        #[arg(short, long, default_value = "react", value_parser = clap::builder::PossibleValuesParser::new(ALL_TARGETS))]
        target: String,
        /// Output directory (default: ./build)
        #[arg(short, long, default_value = "./build")]
        output: String,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Analyse { files, format } => commands::cmd_analyse(files, &format),
        Command::Build {
            files,
            target,
            output,
            store_mode,
        } => commands::cmd_build(files, target, &output, &store_mode),
        Command::Run { file, target } => commands::cmd_run(file, target),
        Command::Create { name, r#type } => commands::cmd_create(&name, &r#type),
        Command::Watch {
            file,
            target,
            output,
        } => commands::cmd_watch(&file, &target, &output),
    };

    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
