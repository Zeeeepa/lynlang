import * as path from 'path';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';
import { exec } from 'child_process';
import { promisify } from 'util';

let client: LanguageClient;
const execAsync = promisify(exec);

// Code Lens Provider for run/build buttons
class ZenCodeLensProvider implements vscode.CodeLensProvider {
    public codeLenses: vscode.CodeLens[] = [];
    private regex = /^\s*(fn|main|build)\s+(\w+)\s*(\(|{)/gm;

    async provideCodeLenses(document: vscode.TextDocument): Promise<vscode.CodeLens[]> {
        this.codeLenses = [];
        const text = document.getText();
        let match;

        // Find all function definitions, with special focus on main and build functions
        while ((match = this.regex.exec(text)) !== null) {
            const keyword = match[1];
            const functionName = match[2];
            const line = document.lineAt(document.positionAt(match.index).line);
            const range = new vscode.Range(line.range.start, line.range.start.translate(0, 40));

            // Check if it's a main or build function
            if (functionName === 'main' || keyword === 'main' || keyword === 'build') {
                const runCommand = new vscode.CodeLens(range, {
                    title: '▶ Run',
                    command: 'zen.run',
                    arguments: [document.uri, functionName]
                });

                const buildCommand = new vscode.CodeLens(range.translate(0, 50), {
                    title: '🔨 Build',
                    command: 'zen.build',
                    arguments: [document.uri, functionName]
                });

                this.codeLenses.push(runCommand);
                this.codeLenses.push(buildCommand);
            } else if (keyword === 'fn') {
                // For regular functions, just add a run option
                const runCommand = new vscode.CodeLens(range, {
                    title: '▶ Run',
                    command: 'zen.run',
                    arguments: [document.uri, functionName]
                });
                this.codeLenses.push(runCommand);
            }
        }

        return this.codeLenses;
    }

    resolveCodeLens(codeLens: vscode.CodeLens): vscode.CodeLens {
        return codeLens;
    }
}

export function activate(context: vscode.ExtensionContext) {
    // Get the path to the language server
    let serverPath = vscode.workspace.getConfiguration('zen').get<string>('serverPath', 'zen-lsp');
    
    // Expand VS Code variables like ${workspaceFolder}
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '';
    serverPath = serverPath.replace(/\$\{workspaceFolder\}/g, workspaceFolder);
    
    // If not absolute, try to resolve it
    let resolvedServerPath: string;
    if (path.isAbsolute(serverPath)) {
        resolvedServerPath = serverPath;
    } else {
        // First try workspace root
        const workspacePath = path.join(workspaceFolder, serverPath);
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

    // Register Code Lens Provider
    const codeLensProvider = new ZenCodeLensProvider();
    context.subscriptions.push(
        vscode.languages.registerCodeLensProvider({ language: 'zen' }, codeLensProvider)
    );

    // Register run command
    context.subscriptions.push(
        vscode.commands.registerCommand('zen.run', async (uri: vscode.Uri, functionName: string) => {
            const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri);
            if (!workspaceFolder) {
                vscode.window.showErrorMessage('No workspace folder found');
                return;
            }

            const outputChannel = vscode.window.createOutputChannel('Zen Run');
            outputChannel.show();
            outputChannel.appendLine(`Running ${functionName}...`);

            try {
                const { stdout, stderr } = await execAsync(`zen run ${uri.fsPath}`, {
                    cwd: workspaceFolder.uri.fsPath
                });
                if (stdout) outputChannel.appendLine(stdout);
                if (stderr) outputChannel.appendLine('STDERR: ' + stderr);
                outputChannel.appendLine(`✓ ${functionName} completed`);
            } catch (error: any) {
                outputChannel.appendLine(`✗ Error running ${functionName}:`);
                outputChannel.appendLine(error.message);
                vscode.window.showErrorMessage(`Failed to run ${functionName}: ${error.message}`);
            }
        })
    );

    // Register build command
    context.subscriptions.push(
        vscode.commands.registerCommand('zen.build', async (uri: vscode.Uri, functionName: string) => {
            const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri);
            if (!workspaceFolder) {
                vscode.window.showErrorMessage('No workspace folder found');
                return;
            }

            const outputChannel = vscode.window.createOutputChannel('Zen Build');
            outputChannel.show();
            outputChannel.appendLine(`Building ${functionName}...`);

            try {
                const { stdout, stderr } = await execAsync(`zen build ${uri.fsPath}`, {
                    cwd: workspaceFolder.uri.fsPath
                });
                if (stdout) outputChannel.appendLine(stdout);
                if (stderr) outputChannel.appendLine('STDERR: ' + stderr);
                outputChannel.appendLine(`✓ Build completed`);
            } catch (error: any) {
                outputChannel.appendLine(`✗ Build failed:`);
                outputChannel.appendLine(error.message);
                vscode.window.showErrorMessage(`Build failed: ${error.message}`);
            }
        })
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