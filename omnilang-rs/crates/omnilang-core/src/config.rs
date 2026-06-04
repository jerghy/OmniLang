use serde::Deserialize;
use std::path::PathBuf;

/// OmniLang project configuration (.omnilang.toml).
#[derive(Debug, Clone, Deserialize)]
pub struct OmniConfig {
    #[serde(default)]
    pub project: ProjectConfig,
    #[serde(default)]
    pub target: TargetConfig,
    #[serde(default)]
    pub analyze: AnalyzeConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProjectConfig {
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TargetConfig {
    #[serde(default)]
    pub js: TargetJsConfig,
    #[serde(default)]
    pub python: TargetPythonConfig,
    #[serde(default)]
    pub rust: TargetRustConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TargetJsConfig {
    #[serde(default = "default_js_mode")]
    pub mode: String,
    #[serde(default = "default_js_module")]
    pub module: String,
}

impl Default for TargetJsConfig {
    fn default() -> Self {
        Self { mode: "plain".into(), module: "esm".into() }
    }
}

fn default_js_mode() -> String { "plain".into() }
fn default_js_module() -> String { "esm".into() }

#[derive(Debug, Clone, Deserialize)]
pub struct TargetPythonConfig {
    #[serde(default = "default_py_version")]
    pub min_version: String,
}

impl Default for TargetPythonConfig {
    fn default() -> Self { Self { min_version: "3.10".into() } }
}

fn default_py_version() -> String { "3.10".into() }

#[derive(Debug, Clone, Deserialize)]
pub struct TargetRustConfig {
    #[serde(default = "default_async_runtime")]
    pub async_runtime: String,
}

impl Default for TargetRustConfig {
    fn default() -> Self { Self { async_runtime: "tokio".into() } }
}

fn default_async_runtime() -> String { "tokio".into() }

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnalyzeConfig {
    #[serde(default = "default_analyze_level")]
    pub level: String,
    #[serde(default)]
    pub rules: std::collections::HashMap<String, String>,
}

fn default_analyze_level() -> String { "port".into() }

impl OmniConfig {
    /// Load config from .omnilang.toml in the given directory.
    pub fn load(dir: &PathBuf) -> Result<Self, String> {
        let path = dir.join(".omnilang.toml");
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| format!("Cannot read config: {}", e))?;
            toml::from_str(&content)
                .map_err(|e| format!("Invalid config: {}", e))
        } else {
            Ok(Self::default())
        }
    }
}

impl Default for OmniConfig {
    fn default() -> Self {
        Self {
            project: ProjectConfig::default(),
            target: TargetConfig::default(),
            analyze: AnalyzeConfig::default(),
        }
    }
}
