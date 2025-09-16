// Zen Compiler - Bootstrap C Implementation
// This is a minimal C implementation to compile basic Zen programs

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <ctype.h>

// Token types
typedef enum {
    TOK_EOF,
    TOK_IDENTIFIER,
    TOK_NUMBER,
    TOK_STRING,
    TOK_ASSIGN,
    TOK_COLON_COLON_ASSIGN,
    TOK_COLON,
    TOK_SEMICOLON,
    TOK_LPAREN,
    TOK_RPAREN,
    TOK_LBRACE,
    TOK_RBRACE,
    TOK_LBRACKET,
    TOK_RBRACKET,
    TOK_DOT,
    TOK_COMMA,
    TOK_QUESTION,
    TOK_PIPE,
    TOK_AT,
    TOK_PLUS,
    TOK_MINUS,
    TOK_STAR,
    TOK_SLASH,
    TOK_PERCENT,
    TOK_EQUAL,
    TOK_NOT_EQUAL,
    TOK_LESS,
    TOK_GREATER,
    TOK_LESS_EQUAL,
    TOK_GREATER_EQUAL,
    TOK_ARROW,
    TOK_KEYWORD,
    TOK_BUILTIN_SYMBOL,
} TokenType;

typedef struct {
    TokenType type;
    char* value;
    int line;
    int column;
} Token;

typedef struct {
    char* source;
    size_t pos;
    size_t len;
    int line;
    int column;
} Lexer;

typedef struct ASTNode {
    enum {
        AST_PROGRAM,
        AST_VAR_DECL,
        AST_ASSIGNMENT,
        AST_FUNCTION,
        AST_CALL,
        AST_IDENTIFIER,
        AST_NUMBER,
        AST_STRING,
        AST_BINARY_OP,
        AST_BLOCK,
        AST_PATTERN_MATCH,
        AST_RETURN,
    } type;
    union {
        struct {
            struct ASTNode** statements;
            size_t count;
        } program;
        struct {
            char* name;
            struct ASTNode* value;
            bool is_mutable;
        } var_decl;
        struct {
            char* name;
            struct ASTNode** params;
            size_t param_count;
            struct ASTNode* body;
        } function;
        struct {
            char* name;
            struct ASTNode** args;
            size_t arg_count;
        } call;
        struct {
            char* value;
        } literal;
        struct {
            char* op;
            struct ASTNode* left;
            struct ASTNode* right;
        } binary;
        struct {
            struct ASTNode** statements;
            size_t count;
        } block;
        struct {
            struct ASTNode* expr;
            struct ASTNode** arms;
            size_t arm_count;
        } pattern_match;
        struct {
            struct ASTNode* value;
        } ret;
    } data;
} ASTNode;

// Lexer functions
Lexer* lexer_new(const char* source) {
    Lexer* lex = malloc(sizeof(Lexer));
    lex->source = strdup(source);
    lex->pos = 0;
    lex->len = strlen(source);
    lex->line = 1;
    lex->column = 1;
    return lex;
}

void lexer_free(Lexer* lex) {
    free(lex->source);
    free(lex);
}

char peek(Lexer* lex) {
    if (lex->pos >= lex->len) return '\0';
    return lex->source[lex->pos];
}

char advance(Lexer* lex) {
    if (lex->pos >= lex->len) return '\0';
    char c = lex->source[lex->pos++];
    if (c == '\n') {
        lex->line++;
        lex->column = 1;
    } else {
        lex->column++;
    }
    return c;
}

void skip_whitespace(Lexer* lex) {
    while (isspace(peek(lex))) {
        advance(lex);
    }
}

Token next_token(Lexer* lex) {
    skip_whitespace(lex);
    
    Token tok = {TOK_EOF, "", lex->line, lex->column};
    
    char c = peek(lex);
    if (c == '\0') {
        return tok;
    }
    
    // Check for @std or @this
    if (c == '@') {
        advance(lex);
        if (isalpha(peek(lex))) {
            size_t start = lex->pos;
            while (isalnum(peek(lex)) || peek(lex) == '_') {
                advance(lex);
            }
            size_t len = lex->pos - start;
            char* value = malloc(len + 1);
            strncpy(value, lex->source + start, len);
            value[len] = '\0';
            
            if (strcmp(value, "std") == 0 || strcmp(value, "this") == 0) {
                tok.type = TOK_BUILTIN_SYMBOL;
                tok.value = value;
                return tok;
            }
            free(value);
        }
        tok.type = TOK_AT;
        return tok;
    }
    
    // Numbers
    if (isdigit(c)) {
        size_t start = lex->pos;
        while (isdigit(peek(lex)) || peek(lex) == '.') {
            advance(lex);
        }
        size_t len = lex->pos - start;
        tok.type = TOK_NUMBER;
        tok.value = malloc(len + 1);
        strncpy(tok.value, lex->source + start, len);
        tok.value[len] = '\0';
        return tok;
    }
    
    // Identifiers and keywords
    if (isalpha(c) || c == '_') {
        size_t start = lex->pos;
        while (isalnum(peek(lex)) || peek(lex) == '_') {
            advance(lex);
        }
        size_t len = lex->pos - start;
        tok.value = malloc(len + 1);
        strncpy(tok.value, lex->source + start, len);
        tok.value[len] = '\0';
        
        // Check for keywords
        if (strcmp(tok.value, "loop") == 0 ||
            strcmp(tok.value, "return") == 0 ||
            strcmp(tok.value, "break") == 0 ||
            strcmp(tok.value, "continue") == 0) {
            tok.type = TOK_KEYWORD;
        } else {
            tok.type = TOK_IDENTIFIER;
        }
        return tok;
    }
    
    // Strings
    if (c == '"') {
        advance(lex); // skip opening quote
        size_t start = lex->pos;
        while (peek(lex) != '"' && peek(lex) != '\0') {
            if (peek(lex) == '\\') {
                advance(lex); // skip escape char
            }
            advance(lex);
        }
        size_t len = lex->pos - start;
        tok.type = TOK_STRING;
        tok.value = malloc(len + 1);
        strncpy(tok.value, lex->source + start, len);
        tok.value[len] = '\0';
        advance(lex); // skip closing quote
        return tok;
    }
    
    // Operators and punctuation
    advance(lex);
    switch (c) {
        case '=':
            if (peek(lex) == '=') {
                advance(lex);
                tok.type = TOK_EQUAL;
            } else {
                tok.type = TOK_ASSIGN;
            }
            break;
        case ':':
            if (peek(lex) == ':') {
                advance(lex);
                if (peek(lex) == '=') {
                    advance(lex);
                    tok.type = TOK_COLON_COLON_ASSIGN;
                }
            } else {
                tok.type = TOK_COLON;
            }
            break;
        case '(': tok.type = TOK_LPAREN; break;
        case ')': tok.type = TOK_RPAREN; break;
        case '{': tok.type = TOK_LBRACE; break;
        case '}': tok.type = TOK_RBRACE; break;
        case '[': tok.type = TOK_LBRACKET; break;
        case ']': tok.type = TOK_RBRACKET; break;
        case ';': tok.type = TOK_SEMICOLON; break;
        case ',': tok.type = TOK_COMMA; break;
        case '.': tok.type = TOK_DOT; break;
        case '?': tok.type = TOK_QUESTION; break;
        case '|': tok.type = TOK_PIPE; break;
        case '+': tok.type = TOK_PLUS; break;
        case '-':
            if (peek(lex) == '>') {
                advance(lex);
                tok.type = TOK_ARROW;
            } else {
                tok.type = TOK_MINUS;
            }
            break;
        case '*': tok.type = TOK_STAR; break;
        case '/': tok.type = TOK_SLASH; break;
        case '%': tok.type = TOK_PERCENT; break;
        case '<':
            if (peek(lex) == '=') {
                advance(lex);
                tok.type = TOK_LESS_EQUAL;
            } else {
                tok.type = TOK_LESS;
            }
            break;
        case '>':
            if (peek(lex) == '=') {
                advance(lex);
                tok.type = TOK_GREATER_EQUAL;
            } else {
                tok.type = TOK_GREATER;
            }
            break;
        case '!':
            if (peek(lex) == '=') {
                advance(lex);
                tok.type = TOK_NOT_EQUAL;
            }
            break;
    }
    
    return tok;
}

// Code generator
void generate_c_code(ASTNode* node, FILE* out, int indent) {
    if (!node) return;
    
    for (int i = 0; i < indent; i++) fprintf(out, "    ");
    
    switch (node->type) {
        case AST_PROGRAM:
            fprintf(out, "#include <stdio.h>\n");
            fprintf(out, "#include <stdlib.h>\n");
            fprintf(out, "#include <stdbool.h>\n\n");
            
            for (size_t i = 0; i < node->data.program.count; i++) {
                generate_c_code(node->data.program.statements[i], out, 0);
                fprintf(out, "\n");
            }
            break;
            
        case AST_FUNCTION:
            if (strcmp(node->data.function.name, "main") == 0) {
                fprintf(out, "int main(void) {\n");
            } else {
                fprintf(out, "void %s(", node->data.function.name);
                for (size_t i = 0; i < node->data.function.param_count; i++) {
                    if (i > 0) fprintf(out, ", ");
                    fprintf(out, "void* param%zu", i);
                }
                fprintf(out, ") {\n");
            }
            generate_c_code(node->data.function.body, out, indent + 1);
            for (int i = 0; i < indent; i++) fprintf(out, "    ");
            fprintf(out, "}\n");
            break;
            
        case AST_VAR_DECL:
            if (node->data.var_decl.is_mutable) {
                fprintf(out, "int %s = ", node->data.var_decl.name);
            } else {
                fprintf(out, "const int %s = ", node->data.var_decl.name);
            }
            generate_c_code(node->data.var_decl.value, out, 0);
            fprintf(out, ";");
            break;
            
        case AST_ASSIGNMENT:
            fprintf(out, "%s = ", node->data.literal.value);
            generate_c_code(node->data.binary.right, out, 0);
            fprintf(out, ";");
            break;
            
        case AST_CALL:
            if (strcmp(node->data.call.name, "println") == 0) {
                fprintf(out, "printf(\"%%s\\n\", ");
                if (node->data.call.arg_count > 0) {
                    generate_c_code(node->data.call.args[0], out, 0);
                } else {
                    fprintf(out, "\"\"");
                }
                fprintf(out, ");");
            } else {
                fprintf(out, "%s(", node->data.call.name);
                for (size_t i = 0; i < node->data.call.arg_count; i++) {
                    if (i > 0) fprintf(out, ", ");
                    generate_c_code(node->data.call.args[i], out, 0);
                }
                fprintf(out, ")");
            }
            break;
            
        case AST_IDENTIFIER:
            fprintf(out, "%s", node->data.literal.value);
            break;
            
        case AST_NUMBER:
            fprintf(out, "%s", node->data.literal.value);
            break;
            
        case AST_STRING:
            fprintf(out, "\"%s\"", node->data.literal.value);
            break;
            
        case AST_BINARY_OP:
            fprintf(out, "(");
            generate_c_code(node->data.binary.left, out, 0);
            fprintf(out, " %s ", node->data.binary.op);
            generate_c_code(node->data.binary.right, out, 0);
            fprintf(out, ")");
            break;
            
        case AST_BLOCK:
            for (size_t i = 0; i < node->data.block.count; i++) {
                generate_c_code(node->data.block.statements[i], out, indent);
                fprintf(out, "\n");
            }
            break;
            
        case AST_RETURN:
            fprintf(out, "return");
            if (node->data.ret.value) {
                fprintf(out, " ");
                generate_c_code(node->data.ret.value, out, 0);
            }
            fprintf(out, ";");
            break;
            
        case AST_PATTERN_MATCH:
            // Simple if-else chain for now
            fprintf(out, "/* Pattern match not yet implemented */");
            break;
    }
}

// Simple parser for demo
ASTNode* parse_simple_program(const char* source) {
    Lexer* lex = lexer_new(source);
    
    ASTNode* program = malloc(sizeof(ASTNode));
    program->type = AST_PROGRAM;
    program->data.program.statements = malloc(sizeof(ASTNode*) * 100);
    program->data.program.count = 0;
    
    // Create a simple main function
    ASTNode* main_func = malloc(sizeof(ASTNode));
    main_func->type = AST_FUNCTION;
    main_func->data.function.name = "main";
    main_func->data.function.params = NULL;
    main_func->data.function.param_count = 0;
    
    // Create body
    ASTNode* body = malloc(sizeof(ASTNode));
    body->type = AST_BLOCK;
    body->data.block.statements = malloc(sizeof(ASTNode*) * 10);
    body->data.block.count = 0;
    
    // Add a simple print statement
    ASTNode* print_call = malloc(sizeof(ASTNode));
    print_call->type = AST_CALL;
    print_call->data.call.name = "println";
    print_call->data.call.args = malloc(sizeof(ASTNode*));
    print_call->data.call.arg_count = 1;
    
    ASTNode* string_arg = malloc(sizeof(ASTNode));
    string_arg->type = AST_STRING;
    string_arg->data.literal.value = "Hello from Zen!";
    print_call->data.call.args[0] = string_arg;
    
    body->data.block.statements[body->data.block.count++] = print_call;
    
    // Add return statement
    ASTNode* ret = malloc(sizeof(ASTNode));
    ret->type = AST_RETURN;
    ret->data.ret.value = NULL;
    body->data.block.statements[body->data.block.count++] = ret;
    
    main_func->data.function.body = body;
    program->data.program.statements[program->data.program.count++] = main_func;
    
    lexer_free(lex);
    return program;
}

int main(int argc, char** argv) {
    if (argc < 2) {
        printf("Usage: zenc <input.zen> [-o output.c]\n");
        return 1;
    }
    
    // Read input file
    FILE* input = fopen(argv[1], "r");
    if (!input) {
        fprintf(stderr, "Error: Cannot open file %s\n", argv[1]);
        return 1;
    }
    
    fseek(input, 0, SEEK_END);
    long size = ftell(input);
    fseek(input, 0, SEEK_SET);
    
    char* source = malloc(size + 1);
    fread(source, 1, size, input);
    source[size] = '\0';
    fclose(input);
    
    // Determine output file
    char* output_name = "output.c";
    for (int i = 2; i < argc - 1; i++) {
        if (strcmp(argv[i], "-o") == 0) {
            output_name = argv[i + 1];
            break;
        }
    }
    
    // Parse and generate
    ASTNode* ast = parse_simple_program(source);
    
    FILE* output = fopen(output_name, "w");
    if (!output) {
        fprintf(stderr, "Error: Cannot create output file %s\n", output_name);
        free(source);
        return 1;
    }
    
    generate_c_code(ast, output, 0);
    fclose(output);
    
    printf("Generated %s\n", output_name);
    
    // Compile the C code
    char compile_cmd[256];
    snprintf(compile_cmd, sizeof(compile_cmd), "gcc -o %s.out %s", output_name, output_name);
    system(compile_cmd);
    printf("Compiled to %s.out\n", output_name);
    
    free(source);
    return 0;
}