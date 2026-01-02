import * as path from 'path';
import * as fs from 'fs';
import * as vscode from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind
} from 'vscode-languageclient/node';
import { exec } from 'child_process';
import { promisify } from 'util';

let client: LanguageClient | undefined;
let isActivating = false;
let hasLock = false;
const execAsync = promisify(exec);
const LOCK_FILE = '/tmp/zen-lsp.lock';

function tryAcquireLock(): boolean {
    try {
        // Try to create lock file exclusively
        const fd = fs.openSync(LOCK_FILE, 'wx');
        fs.writeSync(fd, String(process.pid));
        fs.closeSync(fd);
        return true;
    } catch (e) {
        // Lock exists - check if the owning process is still alive
        try {
            const pid = parseInt(fs.readFileSync(LOCK_FILE, 'utf8').trim());
            // Check if process exists by sending signal 0
            process.kill(pid, 0);
            // Process exists, lock is valid
            return false;
        } catch {
            // Process doesn't exist or can't read file, take over the lock
            try {
                fs.writeFileSync(LOCK_FILE, String(process.pid));
                return true;
            } catch {
                return false;
            }
        }
    }
}

function releaseLock(): void {
    if (hasLock) {
        try {
            fs.unlinkSync(LOCK_FILE);
        } catch {
            // Ignore
        }
        hasLock = false;
    }
}

function registerCommands(context: vscode.ExtensionContext): void {
    // Register run command
    context.subscriptions.push(
        vscode.commands.registerCommand('zen.run', async (uriArg: vscode.Uri | string, functionName: string, lineNumber?: number) => {
            // Handle URI coming as string from LSP code lens
            const uri = typeof uriArg === 'string' ? vscode.Uri.parse(uriArg) : uriArg;
            const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri);
            if (!workspaceFolder) {
                vscode.window.showErrorMessage('No workspace folder found');
                return;
            }

            const outputChannel = vscode.window.createOutputChannel('Zen Run');
            outputChannel.show();
            outputChannel.appendLine(`▶ Running ${functionName}${lineNumber !== undefined ? ` (line ${lineNumber + 1})` : ''}...`);
            outputChannel.appendLine('---');

            try {
                const { stdout, stderr } = await execAsync(`zen "${uri.fsPath}"`, {
                    cwd: workspaceFolder.uri.fsPath,
                    maxBuffer: 1024 * 1024 * 10
                });
                if (stdout) outputChannel.appendLine(stdout);
                if (stderr) outputChannel.appendLine('STDERR:\n' + stderr);
                outputChannel.appendLine('---');
                outputChannel.appendLine(`✓ ${functionName} completed successfully`);
            } catch (error: unknown) {
                const message = error instanceof Error ? error.message : String(error);
                outputChannel.appendLine(`✗ Error running ${functionName}:`);
                outputChannel.appendLine(message);
                outputChannel.appendLine('---');
                vscode.window.showErrorMessage(`Failed to run ${functionName}: ${message}`);
            }
        })
    );

    // Register build command
    context.subscriptions.push(
        vscode.commands.registerCommand('zen.build', async (uriArg: vscode.Uri | string, functionName: string, lineNumber?: number) => {
            // Handle URI coming as string from LSP code lens
            const uri = typeof uriArg === 'string' ? vscode.Uri.parse(uriArg) : uriArg;
            const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri);
            if (!workspaceFolder) {
                vscode.window.showErrorMessage('No workspace folder found');
                return;
            }

            const outputChannel = vscode.window.createOutputChannel('Zen Build');
            outputChannel.show();
            outputChannel.appendLine(`🔨 Building ${functionName}${lineNumber !== undefined ? ` (line ${lineNumber + 1})` : ''}...`);
            outputChannel.appendLine('---');

            try {
                const filename = uri.fsPath.split('/').pop() || 'output';
                const output = filename.replace('.zen', '');
                const { stdout, stderr } = await execAsync(`zen "${uri.fsPath}" -o "${output}"`, {
                    cwd: workspaceFolder.uri.fsPath,
                    maxBuffer: 1024 * 1024 * 10
                });
                if (stdout) outputChannel.appendLine(stdout);
                if (stderr) outputChannel.appendLine('STDERR:\n' + stderr);
                outputChannel.appendLine('---');
                outputChannel.appendLine(`✓ Build completed successfully`);
            } catch (error: unknown) {
                const message = error instanceof Error ? error.message : String(error);
                outputChannel.appendLine(`✗ Build failed:`);
                outputChannel.appendLine(message);
                outputChannel.appendLine('---');
                vscode.window.showErrorMessage(`Build failed: ${message}`);
            }
        })
    );

    // Register test command
    context.subscriptions.push(
        vscode.commands.registerCommand('zen.runTest', async (uriArg: vscode.Uri | string, testName: string) => {
            // Handle URI coming as string from LSP code lens
            const uri = typeof uriArg === 'string' ? vscode.Uri.parse(uriArg) : uriArg;
            const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri);
            if (!workspaceFolder) {
                vscode.window.showErrorMessage('No workspace folder found');
                return;
            }

            const outputChannel = vscode.window.createOutputChannel('Zen Tests');
            outputChannel.show();
            outputChannel.appendLine(`▶ Running test: ${testName}...`);
            outputChannel.appendLine('---');

            try {
                const { stdout, stderr } = await execAsync(`zen test "${uri.fsPath}" --filter "${testName}"`, {
                    cwd: workspaceFolder.uri.fsPath,
                    maxBuffer: 1024 * 1024 * 10
                });
                if (stdout) outputChannel.appendLine(stdout);
                if (stderr) outputChannel.appendLine('STDERR:\n' + stderr);
                outputChannel.appendLine('---');
                outputChannel.appendLine(`✓ Test ${testName} passed`);
            } catch (error: unknown) {
                const message = error instanceof Error ? error.message : String(error);
                outputChannel.appendLine(`✗ Test ${testName} failed:`);
                outputChannel.appendLine(message);
                outputChannel.appendLine('---');
                vscode.window.showErrorMessage(`Test failed: ${message}`);
            }
        })
    );
}

export async function activate(context: vscode.ExtensionContext) {
    // Prevent multiple concurrent activations
    if (isActivating) {
        return;
    }
    isActivating = true;

    // Try to acquire lock - only one extension host should run the LSP
    if (!tryAcquireLock()) {
        isActivating = false;
        // Another extension host owns the LSP, just register commands without starting server
        registerCommands(context);
        return;
    }
    hasLock = true;

    // Prevent multiple activations - stop existing client first and WAIT for it
    if (client) {
        try {
            await client.stop();
        } catch (e) {
            // Ignore errors during stop
        }
        client = undefined;
    }

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

    // Client options with middleware to deduplicate code lenses
    const clientOptions: LanguageClientOptions = {
        documentSelector: [{ scheme: 'file', language: 'zen' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.zen')
        },
        outputChannelName: 'Zen Language Server',
        traceOutputChannel: vscode.window.createOutputChannel('Zen Language Server Trace'),
        middleware: {
            provideCodeLenses: async (document, token, next) => {
                const lenses = await next(document, token);
                if (!lenses) return lenses;
                // Deduplicate by line number and command title
                const seen = new Set<string>();
                return lenses.filter(lens => {
                    const key = `${lens.range.start.line}:${lens.command?.title ?? ''}`;
                    if (seen.has(key)) return false;
                    seen.add(key);
                    return true;
                });
            }
        }
    };

    // Create and start the language client
    client = new LanguageClient(
        'zenLanguageServer',
        'Zen Language Server',
        serverOptions,
        clientOptions
    );

    // Register commands
    registerCommands(context);

    // Start the client (CodeLens is provided by the LSP server)
    await client.start();

    isActivating = false;
    vscode.window.showInformationMessage('Zen Language Server is now active!');
}

export async function deactivate(): Promise<void> {
    isActivating = false;
    releaseLock();
    if (!client) {
        return;
    }
    await client.stop();
    client = undefined;
}
