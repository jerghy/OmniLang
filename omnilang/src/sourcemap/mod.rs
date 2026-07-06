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
            eprintln!("Warning: failed to serialize source map: {}", e);
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
}
