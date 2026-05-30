"""OmniLang Language Server Protocol server.

Provides IDE features via stdio LSP protocol.
Full implementation in Phase 8; this is the Phase 0 skeleton.
"""

import json
import sys


class OmniLangServer:
    """LSP server for OmniLang language support."""

    def __init__(self):
        self.workspace_roots: list[str] = []
        self.documents: dict[str, str] = {}  # uri → source text

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

    def _send(self, msg: dict):
        """Send an LSP message."""
        content = json.dumps(msg)
        header = f"Content-Length: {len(content)}\r\n\r\n"
        sys.stdout.write(header + content)
        sys.stdout.flush()

    def _send_error(self, message: str):
        self._send({"jsonrpc": "2.0", "error": {"code": -1, "message": message}})

    def _handle_initialize(self, msg: dict):
        self.workspace_roots = [msg.get("params", {}).get("rootUri", "")]
        self._send({
            "jsonrpc": "2.0",
            "id": msg.get("id"),
            "result": {
                "capabilities": {
                    "textDocumentSync": 1,
                    "hoverProvider": True,
                    "definitionProvider": True,
                    "completionProvider": {"triggerCharacters": ["."]},
                }
            }
        })

    def _handle_did_open(self, msg: dict):
        doc = msg.get("params", {}).get("textDocument", {})
        self.documents[doc.get("uri", "")] = doc.get("text", "")

    def _handle_did_change(self, msg: dict):
        doc = msg.get("params", {}).get("textDocument", {})
        changes = msg.get("params", {}).get("contentChanges", [])
        if changes:
            self.documents[doc.get("uri", "")] = changes[-1].get("text", "")

    def _handle_hover(self, msg: dict):
        self._send({"jsonrpc": "2.0", "id": msg.get("id"), "result": None})

    def _handle_definition(self, msg: dict):
        self._send({"jsonrpc": "2.0", "id": msg.get("id"), "result": None})


if __name__ == "__main__":
    OmniLangServer().start()
