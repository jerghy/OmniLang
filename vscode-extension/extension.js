const vscode = require('vscode');
const { exec } = require('child_process');

function activate(context) {
    const diagnosticCollection = vscode.languages.createDiagnosticCollection('omnilang');

    async function runCheck(document) {
        if (document.languageId !== 'omnilang') return;
        const diagnostics = [];
        try {
            const result = await new Promise((resolve, reject) => {
                exec(`omnic check "${document.fileName}" --format json`, (err, stdout, stderr) => {
                    if (err && err.code > 1) { reject(stderr); return; }
                    resolve(stdout);
                });
            });
            const data = JSON.parse(result);
            for (const d of (data.diagnostics || [])) {
                const range = new vscode.Range(
                    d.location.line - 1, d.location.column - 1,
                    d.location.line - 1, d.location.column
                );
                const severity = d.severity === 'error' ? vscode.DiagnosticSeverity.Error :
                    d.severity === 'warning' ? vscode.DiagnosticSeverity.Warning :
                    vscode.DiagnosticSeverity.Information;
                diagnostics.push(new vscode.Diagnostic(range, d.message, severity));
            }
        } catch (e) { /* ignore */ }
        diagnosticCollection.set(document.uri, diagnostics);
    }

    context.subscriptions.push(
        vscode.workspace.onDidOpenTextDocument(runCheck),
        vscode.workspace.onDidSaveTextDocument(runCheck),
        vscode.window.onDidChangeActiveTextEditor(e => { if (e) runCheck(e.document); }),
        diagnosticCollection
    );

    if (vscode.window.activeTextEditor) {
        runCheck(vscode.window.activeTextEditor.document);
    }
}

function deactivate() {}

module.exports = { activate, deactivate };
