"""OmniLang Language Server Protocol server.

Provides IDE features via stdio LSP protocol using the Rust analyze() API.
Supports 11 LSP features: diagnostics, completion, hover, definition,
references, rename, semantic tokens, signature help, document symbols,
folding ranges, and formatting.
"""

import json
import os
import sys
import time
from pathlib import Path
from typing import Any


# ── Keyword list for completion ──────────────────────────

KEYWORDS = [
    "fn", "let", "mut", "if", "else", "while", "for", "match",
    "return", "struct", "class", "async", "await", "import",
    "export", "type", "interface", "true", "false", "println",
    "print", "break", "continue", "impl", "nominal", "static",
    "private", "override",
]

# Map from keyword to snippet / detail
KEYWORD_DETAILS = {
    "fn":       "fn name(params) -> Type { ... }",
    "let":      "let name: Type = value;",
    "mut":      "mut — mutable binding modifier",
    "if":       "if condition { ... }",
    "else":     "else { ... }",
    "while":    "while condition { ... }",
    "for":      "for var in iterable { ... }",
    "match":    "match value { pattern => body, ... }",
    "return":   "return value;",
    "struct":   "struct Name { field: Type, ... }",
    "class":    "class Name extends Base implements IFace { ... }",
    "async":    "async fn name(...) -> Type { ... }",
    "await":    "await expr",
    "import":   'import { Name } from "./path";',
    "export":   "export fn name(...) -> Type { ... }",
    "type":     "type Name<T> = Variant1 | Variant2",
    "interface":"interface Name { field: Type; fn method(...) -> Type; }",
    "println":  'println(value);',
    "print":    'print(value);',
    "break":    "break;",
    "continue": "continue;",
    "true":     "true — boolean literal",
    "false":    "false — boolean literal",
    "impl":     "impl StructName { fn method(...) { ... } }",
    "nominal":  "nominal struct — nominal typing modifier",
    "static":   "static — class member modifier",
    "private":  "private — visibility modifier",
    "override": "@override — method override annotation",
}


# ── Built-in type documentation ─────────────────────────

BUILTIN_TYPE_DOCS = {
    "i32":       "32-bit signed integer (-2^31 to 2^31-1)",
    "f64":       "64-bit floating point (IEEE 754 double)",
    "bool":      "Boolean type (true / false)",
    "String":    "UTF-8 encoded string (owned, heap-allocated)",
    "Array":     "Generic array/vector: Array<T>",
    "Map":       "Generic hash map: Map<K, V>",
    "void":      "No value / unit type",
    "null":      "Null / absence of value",
}

# ── OmniView-specific completion data ────────────────────

OMNIVIEW_KEYWORDS = [
    ("component", "component Name { ... }", 14),
    ("store", "store Name { ... }", 14),
    ("theme", "theme { ... }", 14),
    ("signal", "signal name: Type = value;", 14),
    ("prop", "prop name: Type;", 14),
    ("computed", "computed name: Type = expr;", 14),
    ("event", "event name: Type;", 14),
    ("ref", "ref name: Type;", 14),
    ("getter", "getter name: Type = expr;", 14),
    ("action", "action name(params) { ... }", 14),
    ("view", "view { ... }", 14),
    ("style", "style { ... }", 14),
    ("let", "let name: Type = value;", 14),
    ("mut", "mut — mutable binding modifier", 14),
    ("fn", "fn name(params) -> Type { ... }", 14),
    ("import", 'import omniview; | import { X } from "./path";', 14),
    ("async", "async fn/action name(...) { ... }", 14),
]

OMNIVIEW_DIRECTIVES = [
    ("@store", "@store StoreName; — inject a store into this component", 14),
    ("@if", "@if condition { ... } — conditional rendering", 14),
    ("@else", "@else { ... } | @else if condition { ... }", 14),
    ("@for", "@for item in iterable { ... } — list rendering", 14),
    ("@empty", "@empty { ... } — fallback when @for iterable is empty", 14),
    ("@mount", "@mount { ... } — lifecycle: runs after component mounts", 14),
    ("@unmount", "@unmount { ... } — lifecycle: runs before component unmounts", 14),
    ("@effect", "@effect(name) { ... } — runs when dependencies change", 14),
    ("@slot", "@slot [name=\"name\"] — slot definition", 14),
    ("@native", "@native(target) { code } — target-specific raw code", 14),
    ("@fallback", "@fallback { ... } — fallback content", 14),
    ("@error", "@error(name) { ... } — error boundary content", 14),
]

OMNIVIEW_ELEMENT_TAGS = [
    ("Container", "div — abstract container element", 7),
    ("Flex", "div with display:flex — flexbox container", 7),
    ("Grid", "div with display:grid — grid container", 7),
    ("Stack", "div — stacked layout container", 7),
    ("Spacer", "div — flexible spacer", 7),
    ("Text", "span — inline text element", 7),
    ("Heading", "h1-h6 — heading text element (use level prop)", 7),
    ("Image", "img — image element", 7),
    ("Link", "a — hyperlink element", 7),
    ("Divider", "hr — horizontal divider", 7),
    ("Button", "button — clickable button", 7),
    ("Input", "input — text input field", 7),
    ("TextArea", "textarea — multi-line text input", 7),
    ("Checkbox", "input[type=checkbox] — checkbox input", 7),
    ("Select", "select — dropdown select", 7),
    ("List", "ul — unordered list", 7),
    ("ListItem", "li — list item", 7),
    ("Card", "div — card container", 7),
    ("Alert", "div — alert/notification container", 7),
    ("Form", "form — form container", 7),
    ("Spinner", "div — loading spinner", 7),
    ("Progress", "progress — progress bar", 7),
]

OMNIVIEW_ELEMENT_PROPS = [
    ("bind:value", 'bind:value={signalName} — two-way binding for text input', 10),
    ("bind:checked", 'bind:checked={signalName} — two-way binding for checkbox', 10),
    ("key", 'key={expr} — unique key for @for list items', 10),
    ("ref", 'ref={refName} — DOM element reference', 10),
    ("class", 'class={style.className} | class="raw-css" — CSS class binding', 10),
    ("style", 'style={{color: "red", flex: 1}} — inline style object', 10),
    ("test-id", 'test-id="name" — testing attribute', 10),
    ("onPress", "onPress={handler} — click event handler", 10),
    ("onChange", "onChange={handler} — change event handler", 10),
    ("onSubmit", "onSubmit={handler} — form submit handler", 10),
    ("onFocus", "onFocus={handler} — focus event handler", 10),
    ("onBlur", "onBlur={handler} — blur event handler", 10),
    ("onHover", "onHover={handler} — mouse enter handler", 10),
    ("onLeave", "onLeave={handler} — mouse leave handler", 10),
    ("placeholder", 'placeholder="text" — input placeholder', 10),
    ("level", 'level="1"|"2"|"3" — Heading level', 10),
    ("direction", 'direction="row"|"column" — Flex direction', 10),
    ("gap", 'gap="8px" — spacing between flex/grid children', 10),
    ("align", 'align="start"|"center"|"end" — cross-axis alignment', 10),
    ("justify", 'justify="start"|"center"|"end"|"between" — main-axis alignment', 10),
]

# LSP CompletionItemKind values: 1=Text, 2=Method, 3=Function, 4=Constructor,
# 5=Field, 6=Variable, 7=Class, 8=Interface, 9=Module, 10=Property,
# 11=Unit, 12=Value, 13=Enum, 14=Keyword, 15=Snippet, 16=Color, 17=File,
# 18=Reference, 19=Folder, 20=EnumMember, 21=Constant, 22=Struct, 23=Event, 24=Operator, 25=TypeParameter


# ── Helpers ─────────────────────────────────────────────


def _make_lsp_position(line: int, col: int) -> dict:
    """Convert 1-based line/col to 0-based LSP position."""
    return {"line": max(0, line - 1), "character": max(0, col - 1)}


def _make_lsp_range(start_line: int, start_col: int, end_line: int, end_col: int) -> dict:
    """Convert 1-based spans to 0-based LSP range."""
    return {
        "start": _make_lsp_position(start_line, start_col),
        "end": _make_lsp_position(end_line, end_col),
    }


def _make_lsp_location(uri: str, start_line: int, start_col: int, end_line: int, end_col: int) -> dict:
    """Create an LSP Location object."""
    return {
        "uri": uri,
        "range": _make_lsp_range(start_line, start_col, end_line, end_col),
    }


# ── Server ──────────────────────────────────────────────

class OmniLangServer:
    """LSP server for OmniLang language support, backed by Rust analyze()."""

    def __init__(self):
        self.workspace_roots: list[str] = []
        self.documents: dict[str, str] = {}          # uri -> source text
        self.document_versions: dict[str, int] = {}   # uri -> version
        self.document_open: dict[str, bool] = {}      # uri -> is_open
        self.import_graph: dict[str, list[str]] = {}   # uri -> [resolved_import_uris]
        self._pending_diagnostics: dict[str, float] = {}  # uri -> last_change_time
        self._debounce_seconds: float = 0.3

    def _is_omniview_file(self, uri: str) -> bool:
        """Detect if a file is an OmniView .omni file."""
        if uri.endswith('.omni'):
            return True
        # Also detect by content: has import omniview or OmniView keywords
        source = self.documents.get(uri, '')
        if source:
            omni_keywords = ("component ", "store ", "view {", "@if", "@for",
                           "signal ", "computed ", "import omniview")
            if any(kw in source for kw in omni_keywords):
                return True
        return False

    def _extract_word_at(self, uri: str, line: int, col: int) -> str:
        """Extract the word at the given cursor position in a document."""
        source = self.documents.get(uri, "")
        lines = source.split("\n")
        if not (0 <= line - 1 < len(lines)):
            return ""
        src_line = lines[line - 1]
        idx = col - 1
        if not (0 <= idx < len(src_line)):
            return ""
        c = src_line[idx]
        if not (c.isalnum() or c == "_"):
            return ""
        start = idx
        while start > 0 and (src_line[start - 1].isalnum() or src_line[start - 1] == "_"):
            start -= 1
        end = idx
        while end < len(src_line) and (src_line[end].isalnum() or src_line[end] == "_"):
            end += 1
        return src_line[start:end]

    # ── Main loop ────────────────────────────────────

    def start(self):
        """Start the LSP server on stdio."""
        while True:
            try:
                line = sys.stdin.readline()
                if not line:
                    break
                self._handle_message(line.strip())
            except EOFError:
                break
            except Exception as e:
                self._send_error(str(e))

    def _handle_message(self, raw: str):
        """Parse and dispatch an LSP message."""
        # Ensure pending diagnostics fire even if no further requests arrive
        self._run_pending_diagnostics()

        try:
            msg = json.loads(raw)
        except json.JSONDecodeError:
            return
        method = msg.get("method", "")

        if method == "initialize":
            self._handle_initialize(msg)
        elif method == "textDocument/didOpen":
            self._handle_did_open(msg)
        elif method == "textDocument/didChange":
            self._handle_did_change(msg)
        elif method == "textDocument/didClose":
            self._handle_did_close(msg)
        elif method == "textDocument/hover":
            self._handle_hover(msg)
        elif method == "textDocument/definition":
            self._handle_definition(msg)
        elif method == "textDocument/completion":
            self._handle_completion(msg)
        elif method == "textDocument/references":
            self._handle_references(msg)
        elif method == "textDocument/codeAction":
            self._handle_code_action(msg)
        elif method == "textDocument/rename":
            self._handle_rename(msg)
        elif method == "textDocument/prepareRename":
            self._handle_prepare_rename(msg)
        elif method == "textDocument/semanticTokens/full":
            self._handle_semantic_tokens(msg)
        elif method == "textDocument/signatureHelp":
            self._handle_signature_help(msg)
        elif method == "textDocument/documentSymbol":
            self._handle_document_symbol(msg)
        elif method == "textDocument/foldingRange":
            self._handle_folding_range(msg)
        elif method == "textDocument/formatting":
            self._handle_formatting(msg)
        # Unknown methods — ignore silently

    # ── Send helpers ─────────────────────────────────

    def _send(self, msg: dict):
        """Send an LSP message."""
        content = json.dumps(msg)
        header = f"Content-Length: {len(content)}\r\n\r\n"
        sys.stdout.write(header + content)
        sys.stdout.flush()

    def _send_result(self, msg_id: int | None, result: Any):
        self._send({"jsonrpc": "2.0", "id": msg_id, "result": result})

    def _send_error(self, message: str):
        self._send({"jsonrpc": "2.0", "error": {"code": -1, "message": message}})

    def _send_notification(self, method: str, params: Any):
        self._send({"jsonrpc": "2.0", "method": method, "params": params})

    # ── Initialize ──────────────────────────────────

    def _handle_initialize(self, msg: dict):
        self.workspace_roots = []
        params = msg.get("params", {})
        root_uri = params.get("rootUri", "")
        if root_uri:
            import urllib.parse
            root_path = urllib.parse.urlparse(root_uri).path
            # Windows: /C:/... -> C:/...
            if root_path.startswith('/') and len(root_path) > 2 and root_path[2] == ':':
                root_path = root_path[1:]
            self.workspace_roots = [root_path]
            self._scan_workspace(root_path)

        self._send({
            "jsonrpc": "2.0",
            "id": msg.get("id"),
            "result": {
                "capabilities": {
                    "textDocumentSync": {
                        "openClose": True,
                        "change": 1,  # Full text sync
                    },
                    "hoverProvider": True,
                    "definitionProvider": True,
                    "completionProvider": {
                        "triggerCharacters": [".", ":", "@"],
                        "resolveProvider": False,
                    },
                    "referencesProvider": True,
                    "codeActionProvider": {
                        "codeActionKinds": ["quickfix"],
                    },
                    "renameProvider": {
                        "prepareProvider": True,
                    },
                    "semanticTokensProvider": {
                        "full": True,
                        "range": False,
                        "legend": {
                            "tokenTypes": [
                                "keyword", "type", "function", "variable",
                                "string", "number", "comment", "operator",
                            ],
                            "tokenModifiers": [],
                        },
                    },
                    "foldingRangeProvider": True,
                    "signatureHelpProvider": {
                        "triggerCharacters": ["(", ","],
                    },
                    "documentSymbolProvider": True,
                    "documentFormattingProvider": True,
                },
                "serverInfo": {
                    "name": "omnilang-lsp",
                    "version": "0.3.0",
                },
            }
        })

    # ── Workspace scanning ───────────────────────────

    def _scan_workspace(self, root_path: str):
        """Scan workspace for .omni files to pre-load for import resolution.

        Respects .gitignore and .omnilang.toml exclude patterns.
        """
        if not root_path or not os.path.isdir(root_path):
            return

        # Parse .gitignore patterns
        ignore_patterns = self._load_gitignore_patterns(root_path)

        # Parse .omnilang.toml exclude patterns
        try:
            from omnilang.config import load_config
            config = load_config(root_path)
            if config and hasattr(config, 'source') and hasattr(config.source, 'exclude'):
                ignore_patterns.extend(config.source.exclude)
        except Exception:
            pass

        for dirpath, dirnames, filenames in os.walk(root_path):
            # Filter directories in-place to skip ignored dirs
            dirnames[:] = [
                d for d in dirnames
                if not self._is_ignored(os.path.join(dirpath, d), root_path, ignore_patterns)
            ]
            for fname in filenames:
                full_path = os.path.join(dirpath, fname)
                if self._is_ignored(full_path, root_path, ignore_patterns):
                    continue
                if fname.endswith('.omni'):
                    uri = Path(full_path).as_uri()
                    if uri not in self.documents:
                        try:
                            with open(full_path, 'r', encoding='utf-8') as f:
                                self.documents[uri] = f.read()
                            self.document_open[uri] = False
                        except Exception:
                            pass

    @staticmethod
    def _load_gitignore_patterns(root: str) -> list[str]:
        """Load .gitignore patterns as simple glob strings."""
        patterns = ['.git', 'node_modules', '__pycache__', '*.pyc', '.DS_Store']
        gitignore = os.path.join(root, '.gitignore')
        if os.path.exists(gitignore):
            try:
                with open(gitignore, 'r', encoding='utf-8') as f:
                    for line in f:
                        line = line.strip()
                        if line and not line.startswith('#'):
                            patterns.append(line.rstrip('/'))
            except Exception:
                pass
        return patterns

    @staticmethod
    def _is_ignored(path: str, root: str, patterns: list[str]) -> bool:
        """Check if a path matches any ignore pattern."""
        import fnmatch
        rel = os.path.relpath(path, root).replace('\\', '/')
        name = os.path.basename(path)
        for pat in patterns:
            if pat.endswith('/'):
                pat = pat[:-1]
            if fnmatch.fnmatch(rel, pat) or fnmatch.fnmatch(name, pat):
                return True
            # Pattern like "dir/" matches directory contents
            if '/' not in pat and fnmatch.fnmatch(rel, pat + '/*'):
                return True
        return False

    # ── URI / path helpers ───────────────────────────

    @staticmethod
    def _uri_to_path(uri: str) -> str:
        """Convert file:// URI to filesystem path."""
        import urllib.parse
        path = urllib.parse.urlparse(uri).path
        # Windows drive letter normalization: /C:/... -> C:/...
        if path.startswith('/') and len(path) > 2 and path[2] == ':':
            path = path[1:]
        return path

    @staticmethod
    def _line_col_to_byte_offset(source: str, line: int, col: int) -> int:
        """Convert 1-based line/col to byte offset (UTF-8)."""
        source_bytes = source.encode('utf-8')
        lines = source.split('\n')
        offset = 0
        for i in range(min(line - 1, len(lines))):
            offset += len(lines[i].encode('utf-8')) + 1  # +1 for '\n' byte
        # col is character-based; for multi-byte chars, count bytes up to col
        if line <= len(lines):
            chars_before = lines[line - 1][:col - 1]
            offset += len(chars_before.encode('utf-8'))
        return min(offset, len(source_bytes))

    def _build_analysis_request(self, primary_uri: str, operation: str,
                                cursor_line: int = 1, cursor_col: int = 1,
                                extra: dict | None = None) -> dict:
        """Build an AnalysisRequest dict for a given operation."""
        files = []
        primary_path = self._uri_to_path(primary_uri)

        # Include primary file
        if primary_uri in self.documents:
            files.append({
                "path": primary_path,
                "content": self.documents[primary_uri],
                "is_open": self.document_open.get(primary_uri, False),
            })

        # Include imported files (from import graph)
        imported_uris = self.import_graph.get(primary_uri, [])
        for imp_uri in imported_uris:
            if imp_uri in self.documents and imp_uri != primary_uri:
                files.append({
                    "path": self._uri_to_path(imp_uri),
                    "content": self.documents[imp_uri],
                    "is_open": self.document_open.get(imp_uri, False),
                })

        # Calculate byte offset from line/col
        source = self.documents.get(primary_uri, "")
        cursor_offset = self._line_col_to_byte_offset(source, cursor_line, cursor_col)

        return {
            "files": files,
            "primary_file": primary_path,
            "operation": operation,
            "cursor_offset": cursor_offset,
            "extra": extra or {},
        }

    # ── Document lifecycle ─────────────────────────

    def _handle_did_open(self, msg: dict):
        doc = msg.get("params", {}).get("textDocument", {})
        uri = doc.get("uri", "")
        text = doc.get("text", "")
        version = doc.get("version", 0)
        self.documents[uri] = text
        self.document_versions[uri] = version
        self.document_open[uri] = True
        self._schedule_diagnostics(uri)

    def _handle_did_change(self, msg: dict):
        doc = msg.get("params", {}).get("textDocument", {})
        changes = msg.get("params", {}).get("contentChanges", [])
        uri = doc.get("uri", "")
        version = doc.get("version", 0)
        if changes:
            self.documents[uri] = changes[-1].get("text", "")
        self.document_versions[uri] = version
        self._schedule_diagnostics(uri)

    def _handle_did_close(self, msg: dict):
        doc = msg.get("params", {}).get("textDocument", {})
        uri = doc.get("uri", "")
        self.document_open[uri] = False
        # Keep content for import resolution — don't remove from self.documents

    # ── Debounced diagnostics ────────────────────────

    def _schedule_diagnostics(self, uri: str):
        """Schedule a debounced diagnostics run for the given URI."""
        self._pending_diagnostics[uri] = time.time()

    def _run_pending_diagnostics(self):
        """Process all pending diagnostics requests."""
        now = time.time()
        ready_uris = [
            uri for uri, t in self._pending_diagnostics.items()
            if now - t >= self._debounce_seconds
        ]
        for uri in ready_uris:
            self._pending_diagnostics.pop(uri, None)
            self._publish_diagnostics(uri)

    def _publish_diagnostics(self, uri: str):
        """Call Rust analyze() for diagnostics and publish results."""
        from omnilang.rust_bridge import RustBridge

        # Run pending diagnostics check before this one
        self._run_pending_diagnostics()

        source = self.documents.get(uri, "")
        if not source:
            return

        # Build request
        request = self._build_analysis_request(uri, "diagnostics")

        result = RustBridge.analyze(request)
        if result is None:
            return  # Rust not available — fallback to keyword diagnostics

        # Parse and publish
        lsp_diags = []
        primary = result.get("primary_result", {})
        for d in primary.get("diagnostics", []):
            lsp_diags.append({
                "range": self._convert_range(d.get("range", {})),
                "severity": d.get("severity", 2),
                "message": d.get("message", ""),
                "code": d.get("code", ""),
                "source": d.get("source", "omnilang"),
            })

        self._send_notification("textDocument/publishDiagnostics", {
            "uri": uri,
            "diagnostics": lsp_diags,
        })

        # Update import graph from resolved_imports
        resolved = primary.get("resolved_imports", [])
        if resolved:
            imported_uris = []
            for imp in resolved:
                resolved_path = imp.get("resolved_path", "")
                resolved_uri = Path(resolved_path).as_uri()
                imported_uris.append(resolved_uri)
                # Load the imported file if not already known
                if resolved_uri not in self.documents and os.path.exists(resolved_path):
                    try:
                        with open(resolved_path, 'r', encoding='utf-8') as f:
                            self.documents[resolved_uri] = f.read()
                        self.document_open[resolved_uri] = False
                    except Exception:
                        pass
            self.import_graph[uri] = imported_uris

        # Publish cascade diagnostics to other open files
        for other in result.get("other_diagnostics", []):
            other_uri = Path(other.get("file_path", "")).as_uri()
            other_diags = []
            for d in other.get("diagnostics", []):
                other_diags.append({
                    "range": self._convert_range(d.get("range", {})),
                    "severity": d.get("severity", 2),
                    "message": d.get("message", ""),
                    "code": d.get("code", ""),
                    "source": d.get("source", "omnilang"),
                })
            if other_diags:
                self._send_notification("textDocument/publishDiagnostics", {
                    "uri": other_uri,
                    "diagnostics": other_diags,
                })

    # ── Unified request helpers ──────────────────────

    def _extract_position(self, msg: dict) -> tuple[str, int, int]:
        """Extract (uri, 1-based-line, 1-based-col) from an LSP message."""
        params = msg.get("params", {})
        td = params.get("textDocument", {})
        pos = params.get("position", {})
        uri = td.get("uri", "")
        line = pos.get("line", 0) + 1
        col = pos.get("character", 0) + 1
        return uri, line, col

    def _call_analyze(self, msg: dict, operation: str, extra: dict | None = None):
        """Call Rust analyze() for a semantic operation, return parsed result."""
        from omnilang.rust_bridge import RustBridge
        uri, line, col = self._extract_position(msg)
        request = self._build_analysis_request(uri, operation, line, col, extra)
        return RustBridge.analyze(request), uri

    @staticmethod
    def _convert_range(rs_range: dict) -> dict:
        """Convert 1-based Rust range to 0-based LSP range."""
        return {
            "start": {
                "line": rs_range.get("start_line", 1) - 1,
                "character": rs_range.get("start_col", 1) - 1,
            },
            "end": {
                "line": rs_range.get("end_line", 1) - 1,
                "character": rs_range.get("end_col", 1) - 1,
            },
        }

    def _convert_location(self, loc: dict) -> dict:
        """Convert Rust Location to LSP Location."""
        path = loc.get("file_path", "")
        return {
            "uri": Path(path).as_uri() if path else "",
            "range": self._convert_range(loc.get("range", {})),
        }

    # ── textDocument/hover ───────────────────────────

    def _handle_hover(self, msg: dict):
        result, uri = self._call_analyze(msg, "hover")
        if result is None:
            # Fallback: keyword hover
            self._handle_hover_fallback(msg, uri)
            return

        hover = result.get("primary_result", {}).get("hover")
        if hover:
            lsp_hover = {"contents": {"kind": "markdown", "value": hover.get("contents", "")}}
            if hover.get("range"):
                lsp_hover["range"] = self._convert_range(hover["range"])
            self._send_result(msg.get("id"), lsp_hover)
        else:
            self._handle_hover_fallback(msg, uri)

    def _handle_hover_fallback(self, msg: dict, uri: str):
        """Keyword/builtin hover fallback when Rust produces no result."""
        _, line, col = self._extract_position(msg)
        word = self._extract_word_at(uri, line, col)
        if word in KEYWORD_DETAILS:
            self._send_result(msg.get("id"), {
                "contents": {
                    "kind": "markdown",
                    "value": f"**`{word}`** keyword\n\n```omnilang\n{KEYWORD_DETAILS[word]}\n```",
                },
            })
        elif word in BUILTIN_TYPE_DOCS:
            self._send_result(msg.get("id"), {
                "contents": {
                    "kind": "markdown",
                    "value": f"**{word}** — {BUILTIN_TYPE_DOCS[word]}",
                },
            })
        else:
            self._send_result(msg.get("id"), None)

    # ── textDocument/definition ──────────────────────

    def _handle_definition(self, msg: dict):
        result, _ = self._call_analyze(msg, "definition")
        if result is None:
            self._send_result(msg.get("id"), None)
            return

        defs = result.get("primary_result", {}).get("definitions", [])
        if defs:
            locations = [self._convert_location(d) for d in defs]
            self._send_result(msg.get("id"), locations if len(locations) > 1 else locations[0])
        else:
            self._send_result(msg.get("id"), None)

    # ── textDocument/completion ──────────────────────

    def _handle_completion(self, msg: dict):
        result, uri = self._call_analyze(msg, "completion")
        if result is None:
            # Fallback: static keyword completion
            self._handle_completion_fallback(msg, uri)
            return

        items = result.get("primary_result", {}).get("items", [])
        if items:
            self._send_result(msg.get("id"), {"isIncomplete": False, "items": items})
        else:
            self._handle_completion_fallback(msg, uri)

    def _handle_completion_fallback(self, msg: dict, uri: str):
        """Static keyword completion fallback — keyword + type completions."""
        is_omniview = self._is_omniview_file(uri)

        items = []
        if is_omniview:
            for kw, detail, kind in OMNIVIEW_KEYWORDS:
                items.append({"label": kw, "kind": kind, "detail": detail, "insertText": kw})
            for kw, detail, kind in OMNIVIEW_DIRECTIVES:
                items.append({"label": kw, "kind": kind, "detail": detail,
                              "insertText": kw[1:] if kw.startswith("@") else kw})
            for tag, detail, kind in OMNIVIEW_ELEMENT_TAGS:
                items.append({"label": tag, "kind": kind, "detail": detail, "insertText": tag})
            for prop, detail, kind in OMNIVIEW_ELEMENT_PROPS:
                items.append({"label": prop, "kind": kind, "detail": detail, "insertText": prop})
            for kw in KEYWORDS:
                items.append({"label": kw, "kind": 14,
                              "detail": KEYWORD_DETAILS.get(kw, ""), "insertText": kw})
        else:
            for kw in KEYWORDS:
                items.append({"label": kw, "kind": 14,
                              "detail": KEYWORD_DETAILS.get(kw, ""), "insertText": kw})
            for type_name in BUILTIN_TYPE_DOCS:
                items.append({"label": type_name, "kind": 9,
                              "detail": BUILTIN_TYPE_DOCS[type_name], "insertText": type_name})

        self._send_result(msg.get("id"), {"isIncomplete": False, "items": items})

    # ── textDocument/references ──────────────────────

    def _handle_references(self, msg: dict):
        result, _ = self._call_analyze(msg, "references")
        if result is None:
            self._send_result(msg.get("id"), [])
            return

        refs = result.get("primary_result", {}).get("references", [])
        locations = [self._convert_location(r) for r in refs]
        self._send_result(msg.get("id"), locations)

    # ── textDocument/rename ──────────────────────────

    def _handle_rename(self, msg: dict):
        params = msg.get("params", {})
        new_name = params.get("newName", "")
        result, uri = self._call_analyze(msg, "rename", {"new_name": new_name})
        if result is None:
            self._send_result(msg.get("id"), {"changes": {}})
            return

        edits = result.get("primary_result", {}).get("edits", [])
        changes: dict[str, list] = {}
        for edit in edits:
            lsp_edit = {
                "range": self._convert_range(edit.get("range", {})),
                "newText": edit.get("new_text", ""),
            }
            file_path = edit.get("file_path", self._uri_to_path(uri))
            file_uri = Path(file_path).as_uri()
            if file_uri not in changes:
                changes[file_uri] = []
            changes[file_uri].append(lsp_edit)
        self._send_result(msg.get("id"), {"changes": changes})

    def _handle_prepare_rename(self, msg: dict):
        result, _ = self._call_analyze(msg, "prepare_rename")
        if result is None:
            self._send_result(msg.get("id"), None)
            return

        rename_range = result.get("primary_result", {}).get("rename_range")
        if rename_range:
            self._send_result(msg.get("id"), self._convert_range(rename_range))
        else:
            self._send_result(msg.get("id"), None)

    # ── textDocument/semanticTokens/full ─────────────

    def _handle_semantic_tokens(self, msg: dict):
        result, _ = self._call_analyze(msg, "semantic_tokens")
        if result is None:
            self._send_result(msg.get("id"), {"data": []})
            return

        tokens = result.get("primary_result", {}).get("semantic_tokens", [])
        self._send_result(msg.get("id"), {"data": tokens})

    # ── textDocument/signatureHelp ───────────────────

    def _handle_signature_help(self, msg: dict):
        result, _ = self._call_analyze(msg, "signature_help")
        if result is None:
            self._send_result(msg.get("id"), None)
            return

        sig = result.get("primary_result", {}).get("signature")
        if sig:
            self._send_result(msg.get("id"), {
                "signatures": [sig],
                "activeSignature": 0,
                "activeParameter": sig.get("active_parameter", 0),
            })
        else:
            self._send_result(msg.get("id"), None)

    # ── textDocument/documentSymbol ──────────────────

    def _handle_document_symbol(self, msg: dict):
        result, _ = self._call_analyze(msg, "document_symbol")
        if result is None:
            self._send_result(msg.get("id"), [])
            return

        symbols = result.get("primary_result", {}).get("symbols", [])
        self._send_result(msg.get("id"), symbols)

    # ── textDocument/foldingRange ────────────────────

    def _handle_folding_range(self, msg: dict):
        result, _ = self._call_analyze(msg, "folding_ranges")
        if result is None:
            self._send_result(msg.get("id"), [])
            return

        folds = result.get("primary_result", {}).get("folds", [])
        # Convert 1-based to 0-based line numbers
        for f in folds:
            f["startLine"] = f.get("start_line", 1) - 1
            f["endLine"] = f.get("end_line", 1) - 1
            if "start_line" in f:
                del f["start_line"]
            if "end_line" in f:
                del f["end_line"]
        self._send_result(msg.get("id"), folds)

    # ── textDocument/formatting ──────────────────────

    def _handle_formatting(self, msg: dict):
        from omnilang.rust_bridge import RustBridge

        params = msg.get("params", {})
        td = params.get("textDocument", {})
        uri = td.get("uri", "")
        source = self.documents.get(uri, "")

        formatted = RustBridge.format(source)
        if formatted is None or formatted == source:
            self._send_result(msg.get("id"), [])
            return

        lines = source.split("\n")
        full_range = {
            "start": {"line": 0, "character": 0},
            "end": {"line": len(lines), "character": len(lines[-1]) if lines else 0},
        }
        self._send_result(msg.get("id"), [{"range": full_range, "newText": formatted}])

    # ── textDocument/codeAction ──────────────────────

    def _handle_code_action(self, msg: dict):
        """Return quick-fix suggestions based on diagnostics — static rules, no AST needed."""
        params = msg.get("params", {})
        td = params.get("textDocument", {})
        uri = td.get("uri", "")
        context = params.get("context", {})
        diags = context.get("diagnostics", [])

        actions = []
        for diag in diags:
            code = diag.get("code", "")
            range_val = diag.get("range", {})

            if code == "P0101":
                actions.append({
                    "title": "Convert to float division (f64)",
                    "kind": "quickfix", "diagnostics": [diag],
                    "edit": {"changes": {uri: [{"range": range_val,
                        "newText": "(a as f64) / (b as f64)"}]}},
                    "isPreferred": False,
                })
                actions.append({
                    "title": "Add integer division helper (_idiv)",
                    "kind": "quickfix", "diagnostics": [diag],
                    "edit": {"changes": {uri: [{"range": range_val,
                        "newText": "_idiv(a, b)"}]}},
                    "isPreferred": True,
                })
            elif code == "S0101":
                actions.append({
                    "title": "Prefix unused variable with _",
                    "kind": "quickfix", "diagnostics": [diag],
                    "isPreferred": True,
                })
            elif code == "E0202":
                actions.append({
                    "title": "Add _ = await expression to use async result",
                    "kind": "quickfix", "diagnostics": [diag],
                    "isPreferred": True,
                })
            elif code == "P0110":
                actions.append({
                    "title": "Add zero-division guard",
                    "kind": "quickfix", "diagnostics": [diag],
                    "isPreferred": True,
                })

        self._send_result(msg.get("id"), actions)


if __name__ == "__main__":
    OmniLangServer().start()
