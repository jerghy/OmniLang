"""OmniLang Language Server Protocol server.

Provides IDE features via stdio LSP protocol.
Supports: hover, definition, completion, references, codeAction, rename.
"""

import json
import sys
from typing import Any

from omnilang.parser import parse
from omnilang.ast_nodes import (
    Program, FunctionDef, VarDecl, Block, VarRef, BinaryOp, UnaryOp, MethodCall, FuncCall, Assign, IfStmt, WhileStmt, ForInStmt,
    ReturnStmt, PrintStmt, ExprStmt,
    TypeAnnotation, UnionType,
)


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

# ── Helpers ─────────────────────────────────────────────

def _pos_in_span(line: int, col: int, span) -> bool:
    """Check if a cursor position falls within a source span."""
    if span is None:
        return False
    if line < span.line or line > span.end_line:
        return False
    if line == span.line and col < span.col:
        return False
    if line == span.end_line and col > span.end_col:
        return False
    return True


def _find_function_at(prog: Program, line: int, col: int) -> FunctionDef | None:
    """Find the function definition containing the cursor position."""
    for func in prog.functions:
        if func.span and _pos_in_span(line, col, func.span):
            return func
    return None


def _find_symbol_at(prog: Program, line: int, col: int) -> tuple[str, str] | None:
    """Try to find a symbol (function name, variable name, type name) at cursor.

    Returns (name, kind) where kind is one of 'function', 'variable', 'type', 'keyword'.
    """
    # Check function definitions
    for func in prog.functions:
        if func.span and _pos_in_span(line, col, func.span):
            # Check if cursor is on the function name
            # The function name span is typically at line=func.span.line, col offset
            name_span = func.span
            # Approximate: the function name is the second token (after 'fn')
            # We just return the function since we have it
            if line == name_span.line:
                return (func.name, "function")

    # Check struct definitions
    for s in prog.structs:
        if s.span and _pos_in_span(line, col, s.span):
            return (s.name, "type")

    # Check class definitions
    for c in prog.classes:
        if c.span and _pos_in_span(line, col, c.span):
            return (c.name, "type")

    # Check tagged unions
    for tu in prog.tagged_unions:
        if tu.span and _pos_in_span(line, col, tu.span):
            return (tu.name, "type")

    # Walk all function bodies looking for variable references
    for func in prog.functions:
        if func.body:
            result = _find_symbol_in_block(func.body, line, col)
            if result:
                return result

    return None


def _find_symbol_in_block(block: Block, line: int, col: int) -> tuple[str, str] | None:
    """Recursively search a block for a symbol at the given position."""
    for stmt in block.stmts:
        if isinstance(stmt, VarDecl):
            if stmt.span and _pos_in_span(line, col, stmt.span):
                # Check if cursor is on the variable name (typically at col after "let"/"let mut")
                # The VarDecl name span would be approximated
                return (stmt.name, "variable")
        elif isinstance(stmt, ReturnStmt):
            if stmt.value and isinstance(stmt.value, VarRef):
                if stmt.value.span and _pos_in_span(line, col, stmt.value.span):
                    return (stmt.value.name, "variable")
        elif isinstance(stmt, ExprStmt):
            if isinstance(stmt.expr, VarRef):
                if stmt.expr.span and _pos_in_span(line, col, stmt.expr.span):
                    return (stmt.expr.name, "variable")
            elif isinstance(stmt.expr, FuncCall):
                if stmt.expr.func and isinstance(stmt.expr.func, VarRef):
                    if stmt.expr.func.span and _pos_in_span(line, col, stmt.expr.func.span):
                        return (stmt.expr.func.name, "function")
        elif isinstance(stmt, Assign):
            if stmt.span and _pos_in_span(line, col, stmt.span):
                return (stmt.target, "variable")
        elif isinstance(stmt, IfStmt):
            if stmt.then_body:
                result = _find_symbol_in_block(stmt.then_body, line, col)
                if result:
                    return result
            if stmt.else_body:
                result = _find_symbol_in_block(stmt.else_body, line, col)
                if result:
                    return result
            for _, elif_body in stmt.else_ifs:
                result = _find_symbol_in_block(elif_body, line, col)
                if result:
                    return result
        elif isinstance(stmt, WhileStmt):
            if stmt.body:
                result = _find_symbol_in_block(stmt.body, line, col)
                if result:
                    return result
        elif isinstance(stmt, ForInStmt):
            if stmt.span and _pos_in_span(line, col, stmt.span):
                return (stmt.var_name, "variable")
            if stmt.body:
                result = _find_symbol_in_block(stmt.body, line, col)
                if result:
                    return result
        # Check expressions in various statement types
        if isinstance(stmt, (ReturnStmt, ExprStmt, PrintStmt)):
            result = _find_symbol_in_trailing(block, line, col)
            if result:
                return result

    # Check trailing expression
    if block.trailing_expr:
        result = _find_symbol_in_expr(block.trailing_expr, line, col)
        if result:
            return result

    return None


def _find_symbol_in_expr(expr, line: int, col: int) -> tuple[str, str] | None:
    """Search an expression for a symbol at the given position."""
    if isinstance(expr, VarRef) and expr.span:
        if _pos_in_span(line, col, expr.span):
            return (expr.name, "variable")
    if isinstance(expr, FuncCall):
        if isinstance(expr.func, VarRef) and expr.func.span:
            if _pos_in_span(line, col, expr.func.span):
                return (expr.func.name, "function")
        for arg in expr.args:
            result = _find_symbol_in_expr(arg, line, col)
            if result:
                return result
    if isinstance(expr, MethodCall):
        result = _find_symbol_in_expr(expr.object, line, col)
        if result:
            return result
    if isinstance(expr, BinaryOp):
        result = _find_symbol_in_expr(expr.left, line, col)
        if result:
            return result
        result = _find_symbol_in_expr(expr.right, line, col)
        if result:
            return result
    if isinstance(expr, UnaryOp):
        return _find_symbol_in_expr(expr.operand, line, col)
    return None


def _find_symbol_in_trailing(block: Block, line: int, col: int) -> tuple[str, str] | None:
    """Search the trailing expression of a block."""
    if block.trailing_expr:
        return _find_symbol_in_expr(block.trailing_expr, line, col)
    return None


def _find_function_def(prog: Program, name: str) -> FunctionDef | None:
    """Find a function definition by name."""
    for func in prog.functions:
        if func.name == name:
            return func
    return None


def _get_type_doc(type_ann) -> str:
    """Get documentation string for a type annotation."""
    if isinstance(type_ann, TypeAnnotation):
        base = BUILTIN_TYPE_DOCS.get(type_ann.name, f"User-defined type '{type_ann.name}'")
        if type_ann.type_args:
            args = ", ".join(_get_type_doc(t) for t in type_ann.type_args)
            return f"{type_ann.name}<{args}>"
        return base
    elif isinstance(type_ann, UnionType):
        types = ", ".join(_get_type_doc(t) for t in type_ann.types)
        return f"Union: {types}"
    return str(type_ann) if type_ann else "Unknown"


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
    """LSP server for OmniLang language support."""

    def __init__(self):
        self.workspace_roots: list[str] = []
        self.documents: dict[str, str] = {}       # uri -> source text
        self.document_asts: dict[str, Program] = {}  # uri -> parsed AST

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
        try:
            msg = json.loads(raw)
        except json.JSONDecodeError:
            return
        method = msg.get("method", "")
        _msg_id = msg.get("id")

        if method == "initialize":
            self._handle_initialize(msg)
        elif method == "textDocument/didOpen":
            self._handle_did_open(msg)
        elif method == "textDocument/didChange":
            self._handle_did_change(msg)
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
        elif method == "textDocument/didClose":
            self._handle_did_close(msg)
        # Unknown method — ignore

    def _get_or_parse(self, uri: str) -> Program | None:
        """Get cached AST or parse the document."""
        if uri not in self.documents:
            return None
        if uri not in self.document_asts:
            try:
                self.document_asts[uri] = parse(self.documents[uri], filename=uri)
            except Exception:
                return None
        return self.document_asts[uri]

    def _invalidate_ast(self, uri: str):
        """Clear cached AST for a document on change."""
        self.document_asts.pop(uri, None)

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

    # ── Handlers ─────────────────────────────────────

    def _handle_initialize(self, msg: dict):
        self.workspace_roots = [msg.get("params", {}).get("rootUri", "")]
        self._send({
            "jsonrpc": "2.0",
            "id": msg.get("id"),
            "result": {
                "capabilities": {
                    "textDocumentSync": {
                        "openClose": True,
                        "change": 1,          # Full text sync
                    },
                    "hoverProvider": True,
                    "definitionProvider": True,
                    "completionProvider": {
                        "triggerCharacters": [".", ":"],
                        "resolveProvider": False,
                    },
                    "referencesProvider": True,
                    "codeActionProvider": {
                        "codeActionKinds": ["quickfix"],
                    },
                    "renameProvider": {
                        "prepareProvider": False,
                    },
                },
                "serverInfo": {
                    "name": "omnilang-lsp",
                    "version": "0.2.0",
                },
            }
        })

    def _handle_did_open(self, msg: dict):
        doc = msg.get("params", {}).get("textDocument", {})
        uri = doc.get("uri", "")
        text = doc.get("text", "")
        self.documents[uri] = text
        self._invalidate_ast(uri)

    def _handle_did_change(self, msg: dict):
        doc = msg.get("params", {}).get("textDocument", {})
        changes = msg.get("params", {}).get("contentChanges", [])
        uri = doc.get("uri", "")
        if changes:
            self.documents[uri] = changes[-1].get("text", "")
        self._invalidate_ast(uri)

    def _handle_did_close(self, msg: dict):
        doc = msg.get("params", {}).get("textDocument", {})
        uri = doc.get("uri", "")
        self.documents.pop(uri, None)
        self.document_asts.pop(uri, None)

    # ── textDocument/hover ───────────────────────────

    def _handle_hover(self, msg: dict):
        """Return type information and documentation for the symbol under cursor."""
        uri, line, col = self._extract_hover_position(msg)
        prog = self._get_or_parse(uri)
        if prog is None:
            self._send_result(msg.get("id"), None)
            return

        result = _find_symbol_at(prog, line, col)
        if result is None:
            self._handle_hover_word(msg, uri, line, col)
            return

        name, kind = result

        if kind == "function":
            self._handle_hover_function(msg, prog, name)
        elif kind == "type":
            self._handle_hover_type(msg, name)
        elif kind == "variable":
            self._handle_hover_variable(msg, name)
        else:
            self._send_result(msg.get("id"), None)

    def _extract_hover_position(self, msg: dict) -> tuple[str, int, int]:
        """Extract (uri, line, col) from hover message, converting to 1-based."""
        params = msg.get("params", {})
        td = params.get("textDocument", {})
        pos = params.get("position", {})
        uri = td.get("uri", "")
        line = pos.get("line", 0) + 1
        col = pos.get("character", 0) + 1
        return uri, line, col

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

    def _handle_hover_word(self, msg: dict, uri: str, line: int, col: int):
        """Handle hover when the cursor is on a keyword or builtin type."""
        word = self._extract_word_at(uri, line, col)

        if word in BUILTIN_TYPE_DOCS:
            self._send_result(msg.get("id"), {
                "contents": {
                    "kind": "markdown",
                    "value": f"**{word}** — {BUILTIN_TYPE_DOCS[word]}",
                },
                "range": _make_lsp_range(line, col, line, col + len(word)),
            })
            return
        elif word in KEYWORD_DETAILS:
            detail = KEYWORD_DETAILS[word]
            self._send_result(msg.get("id"), {
                "contents": {
                    "kind": "markdown",
                    "value": f"**`{word}`** keyword\n\n```omnilang\n{detail}\n```",
                },
                "range": _make_lsp_range(line, col, line, col + len(word)),
            })
            return

        self._send_result(msg.get("id"), None)

    def _handle_hover_function(self, msg: dict, prog: Program, name: str):
        """Handle hover on a function symbol."""
        func = _find_function_def(prog, name)
        if func:
            parts = [f"**Function `{name}`**\n\n"]
            params_str = ", ".join(
                f"{p.name}: {p.type_ann}" if p.type_ann else p.name
                for p in func.params
            )
            ret_str = f" -> {func.return_type}" if func.return_type else ""
            sig = f"```omnilang\nfn {name}({params_str}){ret_str}\n```"
            parts.append(sig)
            if func.is_async:
                parts.append("\n*async function*")
            self._send_result(msg.get("id"), {
                "contents": {
                    "kind": "markdown",
                    "value": "\n".join(parts),
                },
            })
            return
        self._send_result(msg.get("id"), None)

    def _handle_hover_type(self, msg: dict, name: str):
        """Handle hover on a type symbol."""
        self._send_result(msg.get("id"), {
            "contents": {
                "kind": "markdown",
                "value": f"**Type `{name}`**\n\nCustom type defined in this module.",
            },
        })

    def _handle_hover_variable(self, msg: dict, name: str):
        """Handle hover on a variable symbol."""
        self._send_result(msg.get("id"), {
            "contents": {
                "kind": "markdown",
                "value": f"**Variable `{name}`**",
            },
        })

    # ── textDocument/definition ──────────────────────

    def _handle_definition(self, msg: dict):
        """Return the definition location for the symbol under cursor."""
        params = msg.get("params", {})
        td = params.get("textDocument", {})
        pos = params.get("position", {})
        uri = td.get("uri", "")
        line = pos.get("line", 0) + 1
        col = pos.get("character", 0) + 1

        prog = self._get_or_parse(uri)
        if prog is None:
            self._send_result(msg.get("id"), None)
            return

        symbol = _find_symbol_at(prog, line, col)
        if symbol is None:
            self._send_result(msg.get("id"), None)
            return

        name, kind = symbol

        if kind == "function":
            func = _find_function_def(prog, name)
            if func and func.span:
                loc = _make_lsp_location(
                    uri,
                    func.span.line, func.span.col,
                    func.span.end_line, func.span.end_col,
                )
                self._send_result(msg.get("id"), loc)
                return

        elif kind == "type":
            for s in prog.structs:
                if s.name == name and s.span:
                    loc = _make_lsp_location(
                        uri, s.span.line, s.span.col,
                        s.span.end_line, s.span.end_col,
                    )
                    self._send_result(msg.get("id"), loc)
                    return
            for c in prog.classes:
                if c.name == name and c.span:
                    loc = _make_lsp_location(
                        uri, c.span.line, c.span.col,
                        c.span.end_line, c.span.end_col,
                    )
                    self._send_result(msg.get("id"), loc)
                    return

        elif kind == "variable":
            # Walk all function bodies to find the variable declaration
            for func in prog.functions:
                if func.body:
                    decl = _find_var_decl_in_block(func.body, name)
                    if decl and decl.span:
                        loc = _make_lsp_location(
                            uri,
                            decl.span.line, decl.span.col,
                            decl.span.end_line, decl.span.end_col,
                        )
                        self._send_result(msg.get("id"), loc)
                        return

        self._send_result(msg.get("id"), None)

    # ── textDocument/completion ──────────────────────

    def _handle_completion(self, msg: dict):
        """Return basic keyword completions."""
        items = []
        for kw in KEYWORDS:
            detail = KEYWORD_DETAILS.get(kw, "")
            insert_text = kw
            # Determine kind (14 = Keyword, 1 = Text in LSP)
            items.append({
                "label": kw,
                "kind": 14,         # CompletionItemKind.Keyword
                "detail": detail,
                "insertText": insert_text,
            })

        # Add built-in types
        for type_name in BUILTIN_TYPE_DOCS:
            items.append({
                "label": type_name,
                "kind": 9,          # CompletionItemKind.Struct
                "detail": BUILTIN_TYPE_DOCS[type_name],
                "insertText": type_name,
            })

        self._send_result(msg.get("id"), {
            "isIncomplete": False,
            "items": items,
        })

    # ── textDocument/references ──────────────────────

    def _handle_references(self, msg: dict):
        """Return references (placeholder — returns empty)."""
        self._send_result(msg.get("id"), [])

    # ── textDocument/codeAction ──────────────────────

    def _handle_code_action(self, msg: dict):
        """Return quick-fix suggestions based on diagnostics or context."""
        params = msg.get("params", {})
        td = params.get("textDocument", {})
        uri = td.get("uri", "")
        context = params.get("context", {})
        diags = context.get("diagnostics", [])

        actions = []

        for diag in diags:
            code = diag.get("code", "")
            _message = diag.get("message", "")
            range_val = diag.get("range", {})

            # P0101: integer division -> suggest float cast or explicit idiv
            if code == "P0101":
                actions.append({
                    "title": "Convert to float division (f64)",
                    "kind": "quickfix",
                    "diagnostics": [diag],
                    "edit": {
                        "changes": {
                            uri: [
                                {
                                    "range": range_val,
                                    "newText": "(a as f64) / (b as f64)",
                                }
                            ]
                        }
                    },
                    "isPreferred": False,
                })
                actions.append({
                    "title": "Add integer division helper (_idiv)",
                    "kind": "quickfix",
                    "diagnostics": [diag],
                    "edit": {
                        "changes": {
                            uri: [
                                {
                                    "range": range_val,
                                    "newText": "_idiv(a, b)",
                                }
                            ]
                        }
                    },
                    "isPreferred": True,
                })

            # S0101: unused variable -> suggest prefix with _
            elif code == "S0101":
                actions.append({
                    "title": "Prefix unused variable with _",
                    "kind": "quickfix",
                    "diagnostics": [diag],
                    "isPreferred": True,
                })

            # E0202: async without await -> suggest using await
            elif code == "E0202":
                actions.append({
                    "title": "Add _ = await expression to use async result",
                    "kind": "quickfix",
                    "diagnostics": [diag],
                    "isPreferred": True,
                })

            # P0110: division by zero -> suggest check
            elif code == "P0110":
                actions.append({
                    "title": "Add zero-division guard",
                    "kind": "quickfix",
                    "diagnostics": [diag],
                    "isPreferred": True,
                })

        if not actions:
            # Suggest code actions for the current context
            actions.append({
                "title": "Check for common issues",
                "kind": "quickfix",
                "diagnostics": [],
                "isPreferred": False,
            })

        self._send_result(msg.get("id"), actions)

    # ── textDocument/rename ──────────────────────────

    def _handle_rename(self, msg: dict):
        """Return workspace edit for rename (placeholder)."""
        params = msg.get("params", {})
        new_name = params.get("newName", "")
        td = params.get("textDocument", {})
        uri = td.get("uri", "")

        if not new_name:
            self._send_error("No new name provided for rename.")
            return

        # Placeholder: return empty changes
        self._send_result(msg.get("id"), {
            "changes": {
                uri: [],
            },
        })


# ── Module-level helpers ────────────────────────────────

def _find_var_decl_in_block(block: Block, name: str) -> VarDecl | None:
    """Recursively find a variable declaration by name in a block."""
    for stmt in block.stmts:
        if isinstance(stmt, VarDecl) and stmt.name == name:
            return stmt
        # Recursively search nested blocks
        if isinstance(stmt, IfStmt):
            for b in ([stmt.then_body] + [b for _, b in stmt.else_ifs] + ([stmt.else_body] if stmt.else_body else [])):
                if b:
                    result = _find_var_decl_in_block(b, name)
                    if result:
                        return result
        elif isinstance(stmt, WhileStmt) and stmt.body:
            result = _find_var_decl_in_block(stmt.body, name)
            if result:
                return result
        elif isinstance(stmt, ForInStmt):
            if stmt.var_name == name:
                return VarDecl(mutable=False, name=name, init=None, span=stmt.span)
            if stmt.body:
                result = _find_var_decl_in_block(stmt.body, name)
                if result:
                    return result
    return None


if __name__ == "__main__":
    OmniLangServer().start()
