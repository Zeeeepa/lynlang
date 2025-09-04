import * as vscode from 'vscode';
import * as path from 'path';
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind, Executable } from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    console.log('Zenlang extension is activating');

    const config = vscode.workspace.getConfiguration('zenlang');
    const lspEnabled = config.get<boolean>('lsp.enabled', true);
    
    if (!lspEnabled) {
        console.log('Zenlang LSP is disabled in settings');
        return;
    }

    // Get the workspace root path
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    
    // Try to find zen-lsp binary in the workspace or use the configured path
    let lspPath = config.get<string>('lsp.path', '');
    if (!lspPath) {
        // Try to find zen-lsp binary in target/release or target/debug
        if (workspaceRoot) {
            const releasePath = path.join(workspaceRoot, 'target', 'release', 'zen-lsp');
            const debugPath = path.join(workspaceRoot, 'target', 'debug', 'zen-lsp');
            if (require('fs').existsSync(releasePath)) {
                lspPath = releasePath;
            } else if (require('fs').existsSync(debugPath)) {
                lspPath = debugPath;
            } else {
                // Fallback to assuming zen-lsp is in PATH
                lspPath = 'zen-lsp';
            }
        } else {
            lspPath = 'zen-lsp';
        }
    }
    
    console.log(`Using LSP path: ${lspPath}`);
    
    // Server options - running the LSP server
    const serverExecutable: Executable = {
        command: lspPath,
        args: [],  // zen-lsp expects no args for stdio mode
        options: {
            env: process.env,
            cwd: workspaceRoot || process.cwd()
        }
    };

    const serverOptions: ServerOptions = {
        run: serverExecutable,
        debug: serverExecutable
    };

    // Client options - how the client should communicate
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'zenlang' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.zen')
        },
        outputChannelName: 'Zenlang Language Server',
        traceOutputChannel: vscode.window.createOutputChannel('Zenlang LSP Trace'),
        middleware: {
            provideCompletionItem: async (document, position, context, token, next) => {
                // Add custom completion logic if needed
                return next(document, position, context, token);
            },
            provideHover: async (document, position, token, next) => {
                // Add custom hover logic if needed
                return next(document, position, token);
            }
        }
    };

    // Create and start the language client
    client = new LanguageClient(
        'zenlangLSP',
        'Zenlang Language Server',
        serverOptions,
        clientOptions
    );

    // Start the client
    client.start().then(() => {
        console.log('Zenlang Language Server started successfully');
    }).catch((error) => {
        console.error('Failed to start Zenlang Language Server:', error);
        vscode.window.showErrorMessage(`Failed to start Zenlang Language Server: ${error.message}`);
    });

    // Register additional commands
    const disposables = [
        vscode.commands.registerCommand('zenlang.restartLSP', () => {
            vscode.window.showInformationMessage('Restarting Zenlang Language Server...');
            client.stop().then(() => {
                client.start();
            });
        }),

        vscode.commands.registerCommand('zenlang.showLSPOutput', () => {
            client.outputChannel.show();
        }),

        vscode.commands.registerCommand('zenlang.build', async () => {
            const terminal = vscode.window.createTerminal('Zen Build');
            terminal.show();
            if (workspaceRoot) {
                terminal.sendText(`cd "${workspaceRoot}"`);
            }
            terminal.sendText('zen build');
        }),

        vscode.commands.registerCommand('zenlang.run', async () => {
            const terminal = vscode.window.createTerminal('Zen Run');
            terminal.show();
            if (workspaceRoot) {
                terminal.sendText(`cd "${workspaceRoot}"`);
            }
            terminal.sendText('zen run');
        }),

        vscode.commands.registerCommand('zenlang.test', async () => {
            const terminal = vscode.window.createTerminal('Zen Test');
            terminal.show();
            if (workspaceRoot) {
                terminal.sendText(`cd "${workspaceRoot}"`);
                terminal.sendText('# Running Zen tests');
                terminal.sendText('for test in tests/zen_*.zen; do');
                terminal.sendText('  echo "Running $test"');
                terminal.sendText('  zen run "$test"');
                terminal.sendText('done');
            }
        })
    ];

    disposables.forEach(d => context.subscriptions.push(d));

    // Handle configuration changes
    vscode.workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration('zenlang.lsp')) {
            vscode.window.showInformationMessage('Zenlang LSP configuration changed. Please restart VS Code to apply changes.');
        }
    });

    console.log('Zenlang extension activated');
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}