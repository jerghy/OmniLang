use crate::analyze::diagnostic::{Diagnostic, Severity};
use crate::ast::*;
use std::collections::HashMap;

/// Context passed to each analyze rule.
pub struct AnalyzeContext<'a> {
    pub program: &'a Program,
    pub source_lines: HashMap<String, Vec<String>>,
    pub targets: Vec<String>,
    pub type_map: &'a HashMap<NodeId, TypeRepr>,
    pub rename_map: &'a [HashMap<String, String>],
    pub capture_map: HashMap<NodeId, Vec<super::super::semantic::capture::CaptureInfo>>,
}

/// Trait for all analyze rules.
pub trait AnalyzeRule: Send + Sync {
    fn code(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn severity(&self) -> Severity;
    fn category(&self) -> &'static str;
    fn message(&self) -> &'static str;
    fn help(&self) -> &'static str { "" }
    fn check(&self, ctx: &AnalyzeContext) -> Vec<Diagnostic>;
}

/// Registry of all analyze rules.
pub struct RuleRegistry {
    rules: Vec<Box<dyn AnalyzeRule>>,
    by_code: HashMap<String, usize>,
}

impl RuleRegistry {
    pub fn new() -> Self {
        Self { rules: vec![], by_code: HashMap::new() }
    }

    pub fn register<R: AnalyzeRule + 'static>(&mut self, rule: R) {
        let code = rule.code().to_string();
        self.by_code.insert(code, self.rules.len());
        self.rules.push(Box::new(rule));
    }

    pub fn get(&self, code: &str) -> Option<&dyn AnalyzeRule> {
        self.by_code.get(code).map(|&i| self.rules[i].as_ref())
    }

    pub fn list_all(&self) -> Vec<&dyn AnalyzeRule> {
        self.rules.iter().map(|r| r.as_ref()).collect()
    }

    pub fn list_enabled(&self) -> Vec<&dyn AnalyzeRule> {
        self.rules.iter().map(|r| r.as_ref()).collect()
    }

    pub fn count(&self) -> usize {
        self.rules.len()
    }
}

impl Default for RuleRegistry {
    fn default() -> Self { Self::new() }
}
