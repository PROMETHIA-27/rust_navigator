import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';
import * as vscode from 'vscode';

import {
	Executable,
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind
} from 'vscode-languageclient/node';
import * as lc from 'vscode-languageclient/node';

let client: LanguageClient;

// This method is called when your extension is activated
// Your extension is activated the very first time the command is executed
export async function activate(context: ExtensionContext) {
	let statusBar = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 0);
	statusBar.name = "rust-navigator";
	statusBar.text = "rust-navigator text";
	statusBar.command = 'rust-navigator.stop';
	statusBar.show();

	context.subscriptions.push(statusBar);

	const disposable = vscode.commands.registerCommand('rust-navigator.stop', () => {
		client.stop();
		statusBar.text = "$(stop-circle) rust-navigator";
	});

	context.subscriptions.push(disposable);

	let extension = process.platform == "win32" ? ".exe" : "";
	let serverPath = vscode.Uri.joinPath(context.extensionUri, "server", `rust_navigator${extension}`);
	if (!await fileExists(serverPath)) {
		console.error("failed to load server module");
		return;
	}

	// If the extension is launched in debug mode then the debug server options are used
	// Otherwise the run options are used
	let run: Executable = {
		command: serverPath.fsPath,
		options: {
			env: { ...process.env }
		}
	};
	let serverOptions: ServerOptions = {
		run,
		debug: run,
	};

	// Options to control the language client
	let clientOptions: LanguageClientOptions = {
		// Register the server for plain text documents
		documentSelector: [{ scheme: 'file', language: 'rust' }],
		synchronize: {
			// Notify the server about file changes to '.clientrc files contained in the workspace
			fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
		}
	};

	// Create the language client and start the client.
	client = new LanguageClient(
		'rust-navigator',
		'Rust Navigator',
		serverOptions,
		clientOptions
	);

	// Start the client. This will also launch the server
	client.start();
}

export function deactivate(): Thenable<void> | undefined {
	if (!client) {
		return undefined;
	}
	return client.stop();
}

async function fileExists(uri: vscode.Uri) {
	return await vscode.workspace.fs.stat(uri).then(
		() => true,
		() => false,
	);
}