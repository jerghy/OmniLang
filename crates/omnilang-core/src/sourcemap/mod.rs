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

/// A reverse source map entry mapping a generated position back to its source position.
/// Unlike the forward SourceMapEntry (node_id → target_spans), the reverse index
/// inverts the mapping so each generated position points directly to its source location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReverseSourceMapEntry {
    /// Generated output file path (e.g., "index.html", "index.js")
    pub generated_file: String,
    /// Line in the generated file
    pub generated_line: u32,
    /// Column in the generated file
    pub generated_col: u32,
    /// Source .omni file path (populated when source file info is available)
    pub omni_file: String,
    /// Line in the .omni source file
    pub omni_line: u32,
    /// Column in the .omni source file
    pub omni_col: u32,
}

/// Type alias for a complete reverse source map — a flat list of entries.
pub type ReverseSourceMap = Vec<ReverseSourceMapEntry>;

/// Build a reverse source map from forward source map entries.
///
/// Iterates each forward `SourceMapEntry` and its target spans, producing one
/// `ReverseSourceMapEntry` per target span. The `omni_file` field is left empty
/// when the forward entry does not carry source file information.
#[must_use]
pub fn build_reverse_source_map(entries: &[SourceMapEntry]) -> ReverseSourceMap {
    entries
        .iter()
        .flat_map(|entry| {
            entry.target_spans.iter().map(|ts| ReverseSourceMapEntry {
                generated_file: ts.file.clone(),
                generated_line: ts.line,
                generated_col: ts.col,
                omni_file: String::new(),
                omni_line: entry.omni_span.line,
                omni_col: entry.omni_span.col,
            })
        })
        .collect()
}

/// Serialize source map entries as a JSON array.
/// Each entry maps a generated position back to its source position.
/// Since one source node can produce multiple generated outputs (e.g., DOM, JS, CSS),
/// each target span becomes a separate JSON object.
#[must_use]
pub fn source_map_json(entries: &[SourceMapEntry]) -> String {
    let mappings: Vec<serde_json::Value> = entries
        .iter()
        .flat_map(|entry| {
            entry.target_spans.iter().map(|ts| {
                serde_json::json!({
                    "nodeId": entry.node_id,
                    "sourceLine": entry.omni_span.line,
                    "sourceCol": entry.omni_span.col,
                    "generatedFile": ts.file,
                    "generatedLine": ts.line,
                    "generatedCol": ts.col,
                    "kind": ts.kind,
                })
            })
        })
        .collect();
    match serde_json::to_string(&mappings) {
        Ok(json) => json,
        Err(e) => {
            log::warn!("failed to serialize source map: {}", e);
            "[]".into()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_map_json() {
        let entries = vec![SourceMapEntry {
            node_id: "node_1".into(),
            omni_span: SourceSpan {
                line: 1,
                col: 0,
                len: 10,
            },
            target_spans: vec![
                TargetSpan {
                    file: "output.js".into(),
                    line: 5,
                    col: 0,
                    len: 20,
                    kind: TargetSpanKind::JsExpression,
                },
                TargetSpan {
                    file: "output.html".into(),
                    line: 3,
                    col: 2,
                    len: 15,
                    kind: TargetSpanKind::DomElement,
                },
            ],
        }];
        let json = source_map_json(&entries);
        assert!(json.contains("node_1"));
        assert!(json.contains("sourceLine"));
        assert!(json.contains("generatedFile"));
        assert!(json.contains("generatedLine"));
        assert!(json.contains("output.js"));
        assert!(json.contains("output.html"));
    }

    #[test]
    fn test_source_map_json_empty() {
        let json = source_map_json(&[]);
        assert_eq!(json, "[]");
    }

    #[test]
    fn test_source_map_json_roundtrip() {
        let entries = vec![SourceMapEntry {
            node_id: "btn".into(),
            omni_span: SourceSpan {
                line: 10,
                col: 5,
                len: 8,
            },
            target_spans: vec![TargetSpan {
                file: "out.css".into(),
                line: 42,
                col: 0,
                len: 30,
                kind: TargetSpanKind::CssRule,
            }],
        }];
        let json = source_map_json(&entries);
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&json).expect("valid JSON array");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["nodeId"], "btn");
        assert_eq!(parsed[0]["sourceLine"], 10);
        assert_eq!(parsed[0]["generatedFile"], "out.css");
        assert_eq!(parsed[0]["kind"], "cssrule");
    }

    #[test]
    fn test_build_reverse_source_map_creates_entries() {
        let entries = vec![SourceMapEntry {
            node_id: "node_1".into(),
            omni_span: SourceSpan {
                line: 1,
                col: 0,
                len: 10,
            },
            target_spans: vec![
                TargetSpan {
                    file: "output.js".into(),
                    line: 5,
                    col: 0,
                    len: 20,
                    kind: TargetSpanKind::JsExpression,
                },
                TargetSpan {
                    file: "output.html".into(),
                    line: 3,
                    col: 2,
                    len: 15,
                    kind: TargetSpanKind::DomElement,
                },
            ],
        }];
        let reverse = build_reverse_source_map(&entries);
        assert_eq!(reverse.len(), 2);
        // First reverse entry: output.js → source line 1, col 0
        assert_eq!(reverse[0].generated_file, "output.js");
        assert_eq!(reverse[0].generated_line, 5);
        assert_eq!(reverse[0].generated_col, 0);
        assert_eq!(reverse[0].omni_line, 1);
        assert_eq!(reverse[0].omni_col, 0);
        // Second reverse entry: output.html → source line 1, col 0
        assert_eq!(reverse[1].generated_file, "output.html");
        assert_eq!(reverse[1].generated_line, 3);
        assert_eq!(reverse[1].generated_col, 2);
        assert_eq!(reverse[1].omni_line, 1);
        assert_eq!(reverse[1].omni_col, 0);
    }

    #[test]
    fn test_build_reverse_source_map_empty() {
        let reverse = build_reverse_source_map(&[]);
        assert!(reverse.is_empty());
    }

    #[test]
    fn test_build_reverse_source_map_preserves_generated_col() {
        let entries = vec![SourceMapEntry {
            node_id: "btn".into(),
            omni_span: SourceSpan {
                line: 10,
                col: 5,
                len: 8,
            },
            target_spans: vec![TargetSpan {
                file: "out.css".into(),
                line: 42,
                col: 7,
                len: 30,
                kind: TargetSpanKind::CssRule,
            }],
        }];
        let reverse = build_reverse_source_map(&entries);
        assert_eq!(reverse.len(), 1);
        assert_eq!(reverse[0].generated_file, "out.css");
        assert_eq!(reverse[0].generated_line, 42);
        assert_eq!(reverse[0].generated_col, 7);
        assert_eq!(reverse[0].omni_file, ""); // Not populated from SourceMapEntry
        assert_eq!(reverse[0].omni_line, 10);
        assert_eq!(reverse[0].omni_col, 5);
    }
}
