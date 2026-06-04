use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

/// LSP server communicating over stdio using JSON-RPC 2.0.
pub struct LspServer {
    documents: HashMap<String, DocumentState>,
    next_id: u64,
}

struct DocumentState {
    uri: String,
    language_id: String,
    text: String,
}

#[derive(Debug, Deserialize)]
struct LspMessage {
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
}

impl LspServer {
    pub fn new() -> Self {
        Self { documents: HashMap::new(), next_id: 0 }
    }

    /// Run the LSP server on stdio. Blocks until stdin closes.
    pub fn run(&mut self) {
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin);
        let mut stdout = std::io::stdout();

        for line in reader.lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if line.is_empty() { continue; }

            // Parse Content-Length header
            let content_length = if line.starts_with("Content-Length: ") {
                line["Content-Length: ".len()..].parse::<usize>().unwrap_or(0)
            } else {
                continue;
            };

            // Skip headers until empty line
            let mut headers_done = false;
            let mut content = String::new();
            for header_line in reader.by_ref().lines() {
                let hl = header_line.unwrap_or_default();
                if hl.is_empty() {
                    headers_done = true;
                    continue;
                }
                if headers_done {
                    // Read content
                    let mut buf = vec![0u8; content_length];
                    // Already consumed the first content line as a header check
                    content.push_str(&hl);
                }
            }

            // Try to read the full content
            if content.is_empty() {
                continue;
            }

            let msg: LspMessage = match serde_json::from_str(&content) {
                Ok(m) => m,
                Err(_) => continue,
            };

            let response = self.handle_message(&msg);
            if let Some(resp) = response {
                let resp_str = serde_json::to_string(&resp).unwrap_or_default();
                let header = format!("Content-Length: {}\r\n\r\n", resp_str.len());
                let _ = stdout.write_all(header.as_bytes());
                let _ = stdout.write_all(resp_str.as_bytes());
                let _ = stdout.flush();
            }
        }
    }

    fn handle_message(&mut self, msg: &LspMessage) -> Option<Value> {
        match msg.method.as_deref() {
            Some("initialize") => {
                let result = json!({
                    "capabilities": {
                        "textDocumentSync": { "openClose": true, "change": 1 },
                        "hoverProvider": true,
                        "definitionProvider": true,
                        "completionProvider": {
                            "triggerCharacters": [".", " "]
                        },
                        "codeActionProvider": true,
                    },
                    "serverInfo": {
                        "name": "omnilang-lsp",
                        "version": "0.2.0"
                    }
                });
                Some(self.make_response(msg.id.as_ref()?, &result))
            }
            Some("initialized") => None,
            Some("textDocument/didOpen") => {
                if let Some(params) = &msg.params {
                    let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
                    let text = params["textDocument"]["text"].as_str().unwrap_or("");
                    let lang = params["textDocument"]["languageId"].as_str().unwrap_or("omnilang");
                    self.documents.insert(uri.to_string(), DocumentState {
                        uri: uri.to_string(), language_id: lang.to_string(), text: text.to_string(),
                    });
                    // Send diagnostics
                    self.publish_diagnostics(uri);
                }
                None
            }
            Some("textDocument/didChange") => {
                if let Some(params) = &msg.params {
                    let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
                    if let Some(changes) = params["contentChanges"].as_array() {
                        if let Some(change) = changes.last() {
                            let text = change["text"].as_str().unwrap_or("");
                            if let Some(doc) = self.documents.get_mut(uri) {
                                doc.text = text.to_string();
                            }
                        }
                    }
                    self.publish_diagnostics(uri);
                }
                None
            }
            Some("textDocument/hover") => {
                self.handle_hover(msg)
            }
            Some("textDocument/definition") => {
                self.handle_definition(msg)
            }
            Some("textDocument/completion") => {
                self.handle_completion(msg)
            }
            Some("textDocument/codeAction") => {
                self.handle_code_action(msg)
            }
            Some("shutdown") => {
                Some(self.make_response(msg.id.as_ref()?, &json!(null)))
            }
            _ => None,
        }
    }

    fn make_response(&self, id: &Value, result: &Value) -> Value {
        json!({ "jsonrpc": "2.0", "id": id, "result": result })
    }

    fn publish_diagnostics(&mut self, uri: &str) {
        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return,
        };
        let mut diags_json = vec![];

        // Run omnic check on the document
        if let Ok(diags) = self.run_check(&doc.text, uri) {
            for d in &diags {
                let severity = match d.severity {
                    crate::analyze::diagnostic::Severity::Error => 1,
                    crate::analyze::diagnostic::Severity::Warning => 2,
                    _ => 3,
                };
                diags_json.push(json!({
                    "range": {
                        "start": { "line": d.location.as_ref().map_or(0, |l| l.line.saturating_sub(1)), "character": d.location.as_ref().map_or(0, |l| l.column.saturating_sub(1)) },
                        "end": { "line": d.location.as_ref().map_or(0, |l| l.span.2.saturating_sub(1)), "character": d.location.as_ref().map_or(0, |l| l.span.3.saturating_sub(1)) }
                    },
                    "severity": severity,
                    "source": "omnilang",
                    "message": format!("[{}] {}", d.code, d.message),
                }));
            }
        }

        let notification = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": uri,
                "diagnostics": diags_json
            }
        });
        let msg = serde_json::to_string(&notification).unwrap_or_default();
        let header = format!("Content-Length: {}\r\n\r\n", msg.len());
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(header.as_bytes());
        let _ = stdout.write_all(msg.as_bytes());
        let _ = stdout.flush();
    }

    fn run_check(&self, source: &str, uri: &str) -> Result<Vec<crate::analyze::diagnostic::Diagnostic>, String> {
        let program = crate::parser::parse(source, uri).map_err(|e| e.report(None))?;
        let lines: HashMap<String, Vec<String>> = HashMap::from([
            (uri.to_string(), source.lines().map(|l| l.to_string()).collect()),
        ]);
        let (type_map, rename_map) = crate::semantic::analyze(&program, Some(&lines))
            .map_err(|e| e.report(Some(&lines)))?;
        let diags = crate::analyze::run_checks(&program, &type_map, &rename_map, &["all".into()], lines);
        Ok(diags)
    }

    fn handle_hover(&mut self, msg: &LspMessage) -> Option<Value> {
        let params = msg.params.as_ref()?;
        let uri = params["textDocument"]["uri"].as_str()?;
        let line = params["position"]["line"].as_u64()? as u32 + 1;
        let col = params["position"]["character"].as_u64()? as u32 + 1;

        let doc = self.documents.get(uri)?;
        // Simple hover: show filename and position
        let result = json!({
            "contents": {
                "kind": "markdown",
                "value": format!("OmniLang source\nFile: {}\nLine: {}, Col: {}", uri, line, col)
            }
        });
        Some(self.make_response(msg.id.as_ref()?, &result))
    }

    fn handle_definition(&mut self, msg: &LspMessage) -> Option<Value> {
        // Basic go-to-definition: returns the same position (placeholder)
        let params = msg.params.as_ref()?;
        let _uri = params["textDocument"]["uri"].as_str()?;
        Some(self.make_response(msg.id.as_ref()?, &json!(null)))
    }

    fn handle_completion(&mut self, msg: &LspMessage) -> Option<Value> {
        let items = vec![
            json!({ "label": "fn", "kind": 14, "detail": "Function definition" }),
            json!({ "label": "let", "kind": 14, "detail": "Variable declaration" }),
            json!({ "label": "let mut", "kind": 14, "detail": "Mutable variable" }),
            json!({ "label": "if", "kind": 14, "detail": "If statement" }),
            json!({ "label": "while", "kind": 14, "detail": "While loop" }),
            json!({ "label": "for", "kind": 14, "detail": "For-in loop" }),
            json!({ "label": "return", "kind": 14, "detail": "Return statement" }),
            json!({ "label": "println", "kind": 3, "detail": "Print with newline" }),
            json!({ "label": "print", "kind": 3, "detail": "Print without newline" }),
            json!({ "label": "struct", "kind": 14, "detail": "Struct definition" }),
            json!({ "label": "match", "kind": 14, "detail": "Match expression" }),
            json!({ "label": "i32", "kind": 6, "detail": "32-bit signed integer" }),
            json!({ "label": "f64", "kind": 6, "detail": "64-bit float" }),
            json!({ "label": "String", "kind": 6, "detail": "String type" }),
            json!({ "label": "bool", "kind": 6, "detail": "Boolean type" }),
            json!({ "label": "Array", "kind": 6, "detail": "Array type" }),
            json!({ "label": "Map", "kind": 6, "detail": "Map type" }),
        ];
        let result = json!({ "isIncomplete": false, "items": items });
        Some(self.make_response(msg.id.as_ref()?, &result))
    }

    fn handle_code_action(&mut self, msg: &LspMessage) -> Option<Value> {
        // Return empty code actions for now
        Some(self.make_response(msg.id.as_ref()?, &json!([])))
    }
}
