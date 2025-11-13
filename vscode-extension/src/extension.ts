import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    // Get the path to the language server
    let serverPath = vscode.workspace.getConfiguration('zen').get<string>('serverPath', 'zen-lsp');
    
    // If not absolute, try to resolve it
    let resolvedServerPath: string;
    if (path.isAbsolute(serverPath)) {
        resolvedServerPath = serverPath;
    } else {
        // First try workspace root
        const workspacePath = path.join(vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '', serverPath);
        if (workspacePath && require('fs').existsSync(workspacePath)) {
            resolvedServerPath = workspacePath;
        } else {
            // Try relative to extension directory (for development)
            const extPath = path.join(context.extensionPath, '..', '..', 'target', 'release', 'zen-lsp');
            if (require('fs').existsSync(extPath)) {
                resolvedServerPath = extPath;
            } else {
                // Fall back to PATH lookup
                resolvedServerPath = serverPath;
            }
        }
    }

    // Server options
    const serverOptions: ServerOptions = {
        run: {
            command: resolvedServerPath,
            transport: TransportKind.stdio
        },
        debug: {
            command: resolvedServerPath,
            transport: TransportKind.stdio,
            options: { env: { RUST_LOG: 'debug' } }
        }
    };

    // Client options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'zen' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.zen')
        },
        outputChannelName: 'Zen Language Server',
        traceOutputChannel: vscode.window.createOutputChannel('Zen Language Server Trace')
    };

    // Create and start the language client
    client = new LanguageClient(
        'zenLanguageServer',
        'Zen Language Server',
        serverOptions,
        clientOptions
    );

    // Start the client
    client.start();
    
    vscode.window.showInformationMessage('Zen Language Server is now active!');
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}