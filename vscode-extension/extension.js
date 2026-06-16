const vscode = require('vscode');
const { LanguageClient } = require('vscode-languageclient/node');

let client;

function activate(context) {
    client = new LanguageClient(
        'omnilang',
        'OmniLang Language Server',
        { command: 'omnic', args: ['lsp'] },
        { documentSelector: [{ language: 'omnilang' }] }
    );

    context.subscriptions.push(client.start());
}

function deactivate() {
    if (client) {
        return client.stop();
    }
}

module.exports = { activate, deactivate };
