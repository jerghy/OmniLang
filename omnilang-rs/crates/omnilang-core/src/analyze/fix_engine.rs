use crate::analyze::diagnostic::Diagnostic;
use std::collections::HashMap;

/// Auto-fix engine that can apply suggested fixes to source code.
pub struct FixEngine {
    fixers: HashMap<String, Box<dyn Fn(&Diagnostic, &str) -> Option<String>>>,
}

impl FixEngine {
    pub fn new() -> Self {
        Self { fixers: HashMap::new() }
    }

    pub fn has_fix(&self, code: &str) -> bool {
        self.fixers.contains_key(code)
    }

    pub fn apply(&self, diagnostic: &Diagnostic, source: &str) -> Option<String> {
        if let Some(fixer) = self.fixers.get(&diagnostic.code) {
            fixer(diagnostic, source)
        } else {
            None
        }
    }

    pub fn apply_all(&self, diagnostics: &[Diagnostic], source: &str) -> String {
        let mut result = source.to_string();
        for d in diagnostics.iter().rev() {
            if let Some(fixed) = self.apply(d, &result) {
                result = fixed;
            }
        }
        result
    }
}

impl Default for FixEngine {
    fn default() -> Self { Self::new() }
}

/// Simple fix: replace text in source at a given line/col span.
pub fn replace_in_source(source: &str, old: &str, new: &str) -> String {
    source.replace(old, new)
}
