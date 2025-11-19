"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const path = __importStar(require("path"));
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
const child_process_1 = require("child_process");
const util_1 = require("util");
let client;
const execAsync = (0, util_1.promisify)(child_process_1.exec);
// Code Lens Provider for run/build buttons
class ZenCodeLensProvider {
    constructor() {
        this.codeLenses = [];
        this.regex = /^\s*(fn|main|build)\s+(\w+)\s*(\(|{)/gm;
    }
    async provideCodeLenses(document) {
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
            }
            else if (keyword === 'fn') {
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
    resolveCodeLens(codeLens) {
        return codeLens;
    }
}
function activate(context) {
    // Get the path to the language server
    let serverPath = vscode.workspace.getConfiguration('zen').get('serverPath', 'zen-lsp');
    // Expand VS Code variables like ${workspaceFolder}
    const workspaceFolder = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '';
    serverPath = serverPath.replace(/\$\{workspaceFolder\}/g, workspaceFolder);
    // If not absolute, try to resolve it
    let resolvedServerPath;
    if (path.isAbsolute(serverPath)) {
        resolvedServerPath = serverPath;
    }
    else {
        // First try workspace root
        const workspacePath = path.join(workspaceFolder, serverPath);
        if (workspacePath && require('fs').existsSync(workspacePath)) {
            resolvedServerPath = workspacePath;
        }
        else {
            // Try relative to extension directory (for development)
            const extPath = path.join(context.extensionPath, '..', '..', 'target', 'release', 'zen-lsp');
            if (require('fs').existsSync(extPath)) {
                resolvedServerPath = extPath;
            }
            else {
                // Fall back to PATH lookup
                resolvedServerPath = serverPath;
            }
        }
    }
    // Server options
    const serverOptions = {
        run: {
            command: resolvedServerPath,
            transport: node_1.TransportKind.stdio
        },
        debug: {
            command: resolvedServerPath,
            transport: node_1.TransportKind.stdio,
            options: { env: { RUST_LOG: 'debug' } }
        }
    };
    // Client options
    const clientOptions = {
        documentSelector: [{ scheme: 'file', language: 'zen' }],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.zen')
        },
        outputChannelName: 'Zen Language Server',
        traceOutputChannel: vscode.window.createOutputChannel('Zen Language Server Trace')
    };
    // Create and start the language client
    client = new node_1.LanguageClient('zenLanguageServer', 'Zen Language Server', serverOptions, clientOptions);
    // Register Code Lens Provider
    const codeLensProvider = new ZenCodeLensProvider();
    context.subscriptions.push(vscode.languages.registerCodeLensProvider({ language: 'zen' }, codeLensProvider));
    // Register run command
    context.subscriptions.push(vscode.commands.registerCommand('zen.run', async (uri, functionName) => {
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
            if (stdout)
                outputChannel.appendLine(stdout);
            if (stderr)
                outputChannel.appendLine('STDERR: ' + stderr);
            outputChannel.appendLine(`✓ ${functionName} completed`);
        }
        catch (error) {
            outputChannel.appendLine(`✗ Error running ${functionName}:`);
            outputChannel.appendLine(error.message);
            vscode.window.showErrorMessage(`Failed to run ${functionName}: ${error.message}`);
        }
    }));
    // Register build command
    context.subscriptions.push(vscode.commands.registerCommand('zen.build', async (uri, functionName) => {
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
            if (stdout)
                outputChannel.appendLine(stdout);
            if (stderr)
                outputChannel.appendLine('STDERR: ' + stderr);
            outputChannel.appendLine(`✓ Build completed`);
        }
        catch (error) {
            outputChannel.appendLine(`✗ Build failed:`);
            outputChannel.appendLine(error.message);
            vscode.window.showErrorMessage(`Build failed: ${error.message}`);
        }
    }));
    // Start the client
    client.start();
    vscode.window.showInformationMessage('Zen Language Server is now active!');
}
function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
//# sourceMappingURL=extension.js.map