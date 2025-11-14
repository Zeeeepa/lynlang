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
let client;
function activate(context) {
    // Get the path to the language server
    let serverPath = vscode.workspace.getConfiguration('zen').get('serverPath', 'zen-lsp');
    // If not absolute, try to resolve it
    let resolvedServerPath;
    if (path.isAbsolute(serverPath)) {
        resolvedServerPath = serverPath;
    }
    else {
        // First try workspace root
        const workspacePath = path.join(vscode.workspace.workspaceFolders?.[0]?.uri.fsPath || '', serverPath);
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