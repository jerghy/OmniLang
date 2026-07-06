//! Code generation target enum and parsing.

use std::str::FromStr;

/// Code generation target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Python,
    JavaScript,
    TypeScript,
    Rust,
    Dart,
    React,
    Vue,
    Dioxus,
    Flutter,
    OmniView,
}

impl Target {
    /// Return the file extension for this target.
    pub fn extension(&self) -> &'static str {
        match self {
            Target::Python => ".py",
            Target::JavaScript => ".js",
            Target::TypeScript => ".ts",
            Target::Rust => ".rs",
            Target::Dart => ".dart",
            Target::React => ".tsx",
            Target::Vue => ".vue",
            Target::Dioxus => ".rs",
            Target::Flutter => ".dart",
            Target::OmniView => ".js",
        }
    }
}

impl FromStr for Target {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "python" | "py" => Ok(Target::Python),
            "javascript" | "js" => Ok(Target::JavaScript),
            "typescript" | "ts" => Ok(Target::TypeScript),
            "rust" | "rs" => Ok(Target::Rust),
            "dart" => Ok(Target::Dart),
            "react" | "reactjs" | "react.js" => Ok(Target::React),
            "vue" | "vuejs" | "vue.js" => Ok(Target::Vue),
            "dioxus" => Ok(Target::Dioxus),
            "flutter" => Ok(Target::Flutter),
            "omniview" => Ok(Target::OmniView),
            _ => Err(format!("unknown target: '{s}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_parse_all() {
        assert_eq!(Target::from_str("python").unwrap(), Target::Python);
        assert_eq!(Target::from_str("py").unwrap(), Target::Python);
        assert_eq!(Target::from_str("javascript").unwrap(), Target::JavaScript);
        assert_eq!(Target::from_str("js").unwrap(), Target::JavaScript);
        assert_eq!(Target::from_str("typescript").unwrap(), Target::TypeScript);
        assert_eq!(Target::from_str("ts").unwrap(), Target::TypeScript);
        assert_eq!(Target::from_str("rust").unwrap(), Target::Rust);
        assert_eq!(Target::from_str("rs").unwrap(), Target::Rust);
        assert_eq!(Target::from_str("dart").unwrap(), Target::Dart);
        assert_eq!(Target::from_str("react").unwrap(), Target::React);
        assert_eq!(Target::from_str("vue").unwrap(), Target::Vue);
        assert_eq!(Target::from_str("dioxus").unwrap(), Target::Dioxus);
        assert_eq!(Target::from_str("flutter").unwrap(), Target::Flutter);
        assert_eq!(Target::from_str("omniview").unwrap(), Target::OmniView);
        assert!(Target::from_str("unknown").is_err());
    }

    #[test]
    fn test_target_extension() {
        assert_eq!(Target::Python.extension(), ".py");
        assert_eq!(Target::JavaScript.extension(), ".js");
        assert_eq!(Target::TypeScript.extension(), ".ts");
        assert_eq!(Target::Rust.extension(), ".rs");
        assert_eq!(Target::Dart.extension(), ".dart");
        assert_eq!(Target::React.extension(), ".tsx");
        assert_eq!(Target::Vue.extension(), ".vue");
        assert_eq!(Target::Dioxus.extension(), ".rs");
        assert_eq!(Target::Flutter.extension(), ".dart");
        assert_eq!(Target::OmniView.extension(), ".js");
    }
}
