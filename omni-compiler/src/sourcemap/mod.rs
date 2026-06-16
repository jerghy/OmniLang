//! SourceMap — maps generated code locations back to .omni source.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMapEntry {
    pub node_id: String,
    pub omni_span: SourceSpan,
    pub target_spans: Vec<TargetSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSpan {
    pub line: u32,
    pub col: u32,
    pub len: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetSpan {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub len: u32,
    pub kind: TargetSpanKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetSpanKind {
    DomElement,
    JsExpression,
    CssRule,
}
