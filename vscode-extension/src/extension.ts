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
    const serverPath = vscode.workspace.getConfiguration('zen').get<string>('serverPath', 'zen-lsp');
    
    // If the server path is relative, resolve it relative to the workspace
    const resolvedServerPath = path.isAbsolute(serverPath) 
        ? serverPath 
        : path.join(vscode.workspace.rootPath || '', serverPath);

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