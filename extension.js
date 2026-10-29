// Starts `marrow lsp` for Marrow files: diagnostics while you type and format-on-demand.
const vscode = require("vscode");
const { LanguageClient, TransportKind } = require("vscode-languageclient/node");

let client;

function activate(context) {
  const command = vscode.workspace.getConfiguration("marrow").get("path") || "marrow";
  client = new LanguageClient(
    "marrow",
    "Marrow",
    { command, args: ["lsp"], transport: TransportKind.stdio },
    { documentSelector: [{ scheme: "file", language: "marrow" }] }
  );
  client.start().catch((err) => {
    vscode.window.showWarningMessage(
      `Marrow: could not start \`${command} lsp\` (${err.message}). Install Marrow or set "marrow.path".`
    );
  });
  context.subscriptions.push({ dispose: () => client && client.stop() });
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
