// Zen Compiler v3 - Fixed implementation per LANGUAGE_SPEC.zen
// Addresses parsing bugs in zenc2.c and supports basic Zen features

#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdbool.h>
#include <ctype.h>
#include <stdarg.h>

// Token types
typedef enum {
    TOK_EOF,
    TOK_IDENTIFIER,
    TOK_NUMBER,
    TOK_STRING,
    TOK_ASSIGN,             // =
    TOK_COLON_COLON_ASSIGN, // ::=
    TOK_COLON_COLON,        // ::
    TOK_COLON,              // :
    TOK_SEMICOLON,
    TOK_LPAREN,
    TOK_RPAREN,
    TOK_LBRACE,
    TOK_RBRACE,
    TOK_LBRACKET,
    TOK_RBRACKET,
    TOK_DOT,
    TOK_DOUBLE_DOT,         // ..
    TOK_COMMA,
    TOK_QUESTION,           // ?
    TOK_PIPE,               // |
    TOK_AT,                 // @
    TOK_PLUS,
    TOK_MINUS,
    TOK_STAR,
    TOK_SLASH,
    TOK_PERCENT,
    TOK_EQUAL,              // ==
    TOK_NOT_EQUAL,          // !=
    TOK_LESS,
    TOK_GREATER,
    TOK_LESS_EQUAL,
    TOK_GREATER_EQUAL,
    TOK_ARROW,              // ->
    TOK_KEYWORD,
    TOK_STRING_INTERP_START, // "${
    TOK_TRUE,
    TOK_FALSE,
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
    Token* tokens;
    size_t token_count;
    size_t token_capacity;
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
        AST_STRING_INTERP,
        AST_BOOL,
        AST_BINARY_OP,
        AST_UNARY_OP,
        AST_BLOCK,
        AST_PATTERN_MATCH,
        AST_PATTERN_ARM,
        AST_RETURN,
        AST_BREAK,
        AST_CONTINUE,
        AST_LOOP,
        AST_RANGE,
        AST_STRUCT_LITERAL,
        AST_MEMBER_ACCESS,
        AST_OPTION_SOME,
        AST_OPTION_NONE,
        AST_AT_SYMBOL,
    } type;
    union {
        struct {
            struct ASTNode** statements;
            size_t count;
        } program;
        struct {
            char* name;
            char* type_name;
            struct ASTNode* value;
            bool is_mutable;
            bool has_type;
        } var_decl;
        struct {
            char* name;
            struct ASTNode** params;
            size_t param_count;
            char* return_type;
            struct ASTNode* body;
        } function;
        struct {
            struct ASTNode* func;
            struct ASTNode** args;
            size_t arg_count;
        } call;
        struct {
            char* value;
        } literal;
        struct {
            struct ASTNode** parts;
            size_t part_count;
        } string_interp;
        struct {
            bool value;
        } boolean;
        struct {
            char* op;
            struct ASTNode* left;
            struct ASTNode* right;
        } binary;
        struct {
            char* op;
            struct ASTNode* expr;
        } unary;
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
            struct ASTNode* pattern;
            struct ASTNode* body;
        } pattern_arm;
        struct {
            struct ASTNode* value;
        } ret;
        struct {
            struct ASTNode* condition;
            struct ASTNode* body;
        } loop;
        struct {
            struct ASTNode* start;
            struct ASTNode* end;
            struct ASTNode* step;
        } range;
        struct {
            char* type_name;
            struct ASTNode** fields;
            char** field_names;
            size_t field_count;
        } struct_lit;
        struct {
            struct ASTNode* object;
            char* member;
        } member;
        struct {
            char* module;
            char* path;
        } at_symbol;
        struct {
            struct ASTNode* value;
        } option_some;
        struct {
            struct ASTNode* target;
            struct ASTNode* value;
        } assignment;
    } data;
} ASTNode;

typedef struct {
    Lexer* lexer;
    size_t current;
    Token* tokens;
    size_t token_count;
} Parser;

// Error reporting
void error(const char* fmt, ...) {
    va_list args;
    va_start(args, fmt);
    fprintf(stderr, "Error: ");
    vfprintf(stderr, fmt, args);
    fprintf(stderr, "\n");
    va_end(args);
}

// Lexer functions
Lexer* lexer_new(const char* source) {
    Lexer* lex = malloc(sizeof(Lexer));
    lex->source = strdup(source);
    lex->pos = 0;
    lex->len = strlen(source);
    lex->line = 1;
    lex->column = 1;
    lex->tokens = malloc(sizeof(Token) * 1000);
    lex->token_count = 0;
    lex->token_capacity = 1000;
    return lex;
}

void lexer_free(Lexer* lex) {
    for (size_t i = 0; i < lex->token_count; i++) {
        if (lex->tokens[i].value) {
            free(lex->tokens[i].value);
        }
    }
    free(lex->tokens);
    free(lex->source);
    free(lex);
}

char peek(Lexer* lex) {
    if (lex->pos >= lex->len) return '\0';
    return lex->source[lex->pos];
}

char peek_next(Lexer* lex) {
    if (lex->pos + 1 >= lex->len) return '\0';
    return lex->source[lex->pos + 1];
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

void skip_comment(Lexer* lex) {
    if (peek(lex) == '/' && peek_next(lex) == '/') {
        while (peek(lex) != '\n' && peek(lex) != '\0') {
            advance(lex);
        }
    }
}

void add_token(Lexer* lex, Token tok) {
    if (lex->token_count >= lex->token_capacity) {
        lex->token_capacity *= 2;
        lex->tokens = realloc(lex->tokens, sizeof(Token) * lex->token_capacity);
    }
    lex->tokens[lex->token_count++] = tok;
}

void tokenize(Lexer* lex) {
    while (peek(lex) != '\0') {
        skip_whitespace(lex);
        skip_comment(lex);
        
        if (peek(lex) == '\0') break;
        
        Token tok = {0};
        tok.line = lex->line;
        tok.column = lex->column;
        
        char c = peek(lex);
        
        // @std or @this
        if (c == '@') {
            advance(lex);
            size_t start = lex->pos;
            while (isalnum(peek(lex)) || peek(lex) == '_' || peek(lex) == '.') {
                advance(lex);
            }
            size_t len = lex->pos - start;
            tok.type = TOK_AT;
            tok.value = malloc(len + 2);
            tok.value[0] = '@';
            strncpy(tok.value + 1, lex->source + start, len);
            tok.value[len + 1] = '\0';
            add_token(lex, tok);
            continue;
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
            add_token(lex, tok);
            continue;
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
            
            // Check for boolean literals and keywords
            if (strcmp(tok.value, "true") == 0) {
                tok.type = TOK_TRUE;
            } else if (strcmp(tok.value, "false") == 0) {
                tok.type = TOK_FALSE;
            } else if (strcmp(tok.value, "loop") == 0 ||
                       strcmp(tok.value, "return") == 0 ||
                       strcmp(tok.value, "break") == 0 ||
                       strcmp(tok.value, "continue") == 0 ||
                       strcmp(tok.value, "Some") == 0 ||
                       strcmp(tok.value, "None") == 0 ||
                       strcmp(tok.value, "void") == 0) {
                tok.type = TOK_KEYWORD;
            } else {
                tok.type = TOK_IDENTIFIER;
            }
            add_token(lex, tok);
            continue;
        }
        
        // Strings
        if (c == '"') {
            advance(lex); // skip opening quote
            tok.type = TOK_STRING;
            
            size_t capacity = 100;
            char* buffer = malloc(capacity);
            size_t buffer_len = 0;
            
            while (peek(lex) != '"' && peek(lex) != '\0') {
                if (peek(lex) == '\\') {
                    advance(lex);
                    if (buffer_len + 2 >= capacity) {
                        capacity *= 2;
                        buffer = realloc(buffer, capacity);
                    }
                    char escaped = advance(lex);
                    switch (escaped) {
                        case 'n': buffer[buffer_len++] = '\n'; break;
                        case 't': buffer[buffer_len++] = '\t'; break;
                        case 'r': buffer[buffer_len++] = '\r'; break;
                        case '\\': buffer[buffer_len++] = '\\'; break;
                        case '"': buffer[buffer_len++] = '"'; break;
                        default: buffer[buffer_len++] = escaped; break;
                    }
                } else {
                    if (buffer_len + 1 >= capacity) {
                        capacity *= 2;
                        buffer = realloc(buffer, capacity);
                    }
                    buffer[buffer_len++] = advance(lex);
                }
            }
            buffer[buffer_len] = '\0';
            tok.value = buffer;
            
            if (peek(lex) == '"') {
                advance(lex); // skip closing quote
            }
            add_token(lex, tok);
            continue;
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
                    } else {
                        tok.type = TOK_COLON_COLON;
                    }
                } else {
                    tok.type = TOK_COLON;
                }
                break;
            case '.':
                if (peek(lex) == '.') {
                    advance(lex);
                    tok.type = TOK_DOUBLE_DOT;
                } else {
                    tok.type = TOK_DOT;
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
            default:
                continue; // Skip unknown characters
        }
        
        add_token(lex, tok);
    }
    
    // Add EOF token
    Token eof = {TOK_EOF, NULL, lex->line, lex->column};
    add_token(lex, eof);
}

// Parser functions
Parser* parser_new(Lexer* lex) {
    Parser* p = malloc(sizeof(Parser));
    p->lexer = lex;
    p->current = 0;
    p->tokens = lex->tokens;
    p->token_count = lex->token_count;
    return p;
}

void parser_free(Parser* p) {
    free(p);
}

Token* current_token(Parser* p) {
    if (p->current >= p->token_count) return &p->tokens[p->token_count - 1];
    return &p->tokens[p->current];
}

Token* peek_token(Parser* p) {
    if (p->current + 1 >= p->token_count) return &p->tokens[p->token_count - 1];
    return &p->tokens[p->current + 1];
}

void advance_token(Parser* p) {
    if (p->current < p->token_count - 1) {
        p->current++;
    }
}

bool match_token(Parser* p, TokenType type) {
    if (current_token(p)->type == type) {
        advance_token(p);
        return true;
    }
    return false;
}

bool expect_token(Parser* p, TokenType type, const char* msg) {
    if (current_token(p)->type != type) {
        error("%s at line %d, column %d", msg, 
              current_token(p)->line, current_token(p)->column);
        return false;
    }
    advance_token(p);
    return true;
}

// Forward declarations
ASTNode* parse_expression(Parser* p);
ASTNode* parse_statement(Parser* p);
ASTNode* parse_block(Parser* p);

// Parse primary expressions
ASTNode* parse_primary(Parser* p) {
    Token* tok = current_token(p);
    
    // Numbers
    if (tok->type == TOK_NUMBER) {
        ASTNode* node = malloc(sizeof(ASTNode));
        node->type = AST_NUMBER;
        node->data.literal.value = strdup(tok->value);
        advance_token(p);
        return node;
    }
    
    // Strings
    if (tok->type == TOK_STRING) {
        ASTNode* node = malloc(sizeof(ASTNode));
        node->type = AST_STRING;
        node->data.literal.value = strdup(tok->value);
        advance_token(p);
        return node;
    }
    
    // Booleans
    if (tok->type == TOK_TRUE || tok->type == TOK_FALSE) {
        ASTNode* node = malloc(sizeof(ASTNode));
        node->type = AST_BOOL;
        node->data.boolean.value = (tok->type == TOK_TRUE);
        advance_token(p);
        return node;
    }
    
    // @std or @this
    if (tok->type == TOK_AT) {
        ASTNode* node = malloc(sizeof(ASTNode));
        node->type = AST_AT_SYMBOL;
        node->data.at_symbol.module = strdup(tok->value);
        advance_token(p);
        
        // Handle function call
        if (match_token(p, TOK_LPAREN)) {
            ASTNode* call = malloc(sizeof(ASTNode));
            call->type = AST_CALL;
            call->data.call.func = node;
            call->data.call.args = malloc(sizeof(ASTNode*) * 10);
            call->data.call.arg_count = 0;
            
            while (!match_token(p, TOK_RPAREN)) {
                if (call->data.call.arg_count > 0) {
                    if (!match_token(p, TOK_COMMA)) {
                        error("Expected ',' between arguments");
                        break;
                    }
                }
                if (current_token(p)->type == TOK_RPAREN) {
                    break;
                }
                call->data.call.args[call->data.call.arg_count++] = parse_expression(p);
            }
            
            return call;
        }
        
        return node;
    }
    
    // Identifiers
    if (tok->type == TOK_IDENTIFIER) {
        ASTNode* node = malloc(sizeof(ASTNode));
        node->type = AST_IDENTIFIER;
        node->data.literal.value = strdup(tok->value);
        advance_token(p);
        
        // Check for function call
        if (match_token(p, TOK_LPAREN)) {
            ASTNode* call = malloc(sizeof(ASTNode));
            call->type = AST_CALL;
            call->data.call.func = node;
            call->data.call.args = malloc(sizeof(ASTNode*) * 10);
            call->data.call.arg_count = 0;
            
            while (!match_token(p, TOK_RPAREN)) {
                if (call->data.call.arg_count > 0) {
                    if (!match_token(p, TOK_COMMA)) {
                        error("Expected ',' between arguments");
                        break;
                    }
                }
                if (current_token(p)->type == TOK_RPAREN) {
                    break;
                }
                call->data.call.args[call->data.call.arg_count++] = parse_expression(p);
            }
            
            return call;
        }
        
        return node;
    }
    
    // Parenthesized expressions
    if (match_token(p, TOK_LPAREN)) {
        ASTNode* expr = parse_expression(p);
        expect_token(p, TOK_RPAREN, "Expected ')' after expression");
        return expr;
    }
    
    // Skip unrecognized tokens to prevent infinite loops
    if (tok->type != TOK_EOF) {
        error("Unexpected token type %d, skipping", tok->type);
        advance_token(p);
        return parse_primary(p);
    }
    
    return NULL;
}

// Parse binary expressions with minimal precedence
ASTNode* parse_binary_expr(Parser* p, int min_prec) {
    ASTNode* left = parse_primary(p);
    if (!left) return NULL;
    
    while (true) {
        Token* op = current_token(p);
        
        // Simple precedence for basic operators
        int prec = 0;
        switch (op->type) {
            case TOK_STAR:
            case TOK_SLASH:
                prec = 10;
                break;
            case TOK_PLUS:
            case TOK_MINUS:
                prec = 9;
                break;
            case TOK_EQUAL:
            case TOK_NOT_EQUAL:
                prec = 6;
                break;
            default:
                return left;
        }
        
        if (prec < min_prec) {
            return left;
        }
        
        advance_token(p);
        ASTNode* right = parse_binary_expr(p, prec + 1);
        if (!right) return left;
        
        ASTNode* binary = malloc(sizeof(ASTNode));
        binary->type = AST_BINARY_OP;
        binary->data.binary.left = left;
        binary->data.binary.right = right;
        
        switch (op->type) {
            case TOK_PLUS: binary->data.binary.op = "+"; break;
            case TOK_MINUS: binary->data.binary.op = "-"; break;
            case TOK_STAR: binary->data.binary.op = "*"; break;
            case TOK_SLASH: binary->data.binary.op = "/"; break;
            case TOK_EQUAL: binary->data.binary.op = "=="; break;
            case TOK_NOT_EQUAL: binary->data.binary.op = "!="; break;
            default: binary->data.binary.op = "?"; break;
        }
        
        left = binary;
    }
}

ASTNode* parse_expression(Parser* p) {
    return parse_binary_expr(p, 0);
}

// Parse statements
ASTNode* parse_statement(Parser* p) {
    Token* tok = current_token(p);
    
    // Handle return statement
    if (tok->type == TOK_KEYWORD && strcmp(tok->value, "return") == 0) {
        advance_token(p);
        ASTNode* ret = malloc(sizeof(ASTNode));
        ret->type = AST_RETURN;
        
        if (current_token(p)->type != TOK_RBRACE && current_token(p)->type != TOK_EOF) {
            ret->data.ret.value = parse_expression(p);
        } else {
            ret->data.ret.value = NULL;
        }
        
        return ret;
    }
    
    // Handle variable declaration or assignment
    if (tok->type == TOK_IDENTIFIER) {
        char* name = strdup(tok->value);
        advance_token(p);
        
        // Function declaration: name = (params) return_type { body }
        if (current_token(p)->type == TOK_ASSIGN && peek_token(p)->type == TOK_LPAREN) {
            advance_token(p); // consume =
            advance_token(p); // consume (
            
            ASTNode* func = malloc(sizeof(ASTNode));
            func->type = AST_FUNCTION;
            func->data.function.name = name;
            func->data.function.params = malloc(sizeof(ASTNode*) * 10);
            func->data.function.param_count = 0;
            
            // Parse parameters (simplified - just skip for now)
            while (!match_token(p, TOK_RPAREN)) {
                if (func->data.function.param_count > 0) {
                    match_token(p, TOK_COMMA);
                }
                // For now, just skip parameter tokens
                if (current_token(p)->type != TOK_RPAREN) {
                    advance_token(p);
                }
            }
            
            // Parse return type
            if (current_token(p)->type == TOK_KEYWORD || current_token(p)->type == TOK_IDENTIFIER) {
                func->data.function.return_type = strdup(current_token(p)->value);
                advance_token(p);
            } else {
                func->data.function.return_type = strdup("void");
            }
            
            // Parse body
            if (!expect_token(p, TOK_LBRACE, "Expected '{' for function body")) {
                free(func);
                return NULL;
            }
            func->data.function.body = parse_block(p);
            
            return func;
        }
        
        // Immutable assignment: x = value
        if (current_token(p)->type == TOK_ASSIGN) {
            advance_token(p); // consume =
            
            ASTNode* var = malloc(sizeof(ASTNode));
            var->type = AST_VAR_DECL;
            var->data.var_decl.name = name;
            var->data.var_decl.is_mutable = false;
            var->data.var_decl.has_type = false;
            var->data.var_decl.value = parse_expression(p);
            
            return var;
        }
        
        // Otherwise, it's just an expression
        p->current--; // backtrack
        free(name);
        return parse_expression(p);
    }
    
    // Default to expression
    return parse_expression(p);
}

// Parse block
ASTNode* parse_block(Parser* p) {
    ASTNode* block = malloc(sizeof(ASTNode));
    block->type = AST_BLOCK;
    block->data.block.statements = malloc(sizeof(ASTNode*) * 100);
    block->data.block.count = 0;
    
    while (current_token(p)->type != TOK_RBRACE && 
           current_token(p)->type != TOK_EOF) {
        ASTNode* stmt = parse_statement(p);
        if (stmt) {
            block->data.block.statements[block->data.block.count++] = stmt;
        }
        
        // Prevent infinite loops
        if (block->data.block.count > 50) {
            error("Block too large, stopping parse");
            break;
        }
    }
    
    expect_token(p, TOK_RBRACE, "Expected '}' at end of block");
    
    return block;
}

// Parse program
ASTNode* parse_program(Parser* p) {
    ASTNode* program = malloc(sizeof(ASTNode));
    program->type = AST_PROGRAM;
    program->data.program.statements = malloc(sizeof(ASTNode*) * 1000);
    program->data.program.count = 0;
    
    while (current_token(p)->type != TOK_EOF) {
        ASTNode* stmt = parse_statement(p);
        if (stmt) {
            program->data.program.statements[program->data.program.count++] = stmt;
        }
        
        // Prevent infinite loops
        if (program->data.program.count > 100) {
            error("Program too large, stopping parse");
            break;
        }
    }
    
    return program;
}

// Code generator
void generate_c_code(ASTNode* node, FILE* out, int indent) {
    if (!node) return;
    
    for (int i = 0; i < indent; i++) fprintf(out, "    ");
    
    switch (node->type) {
        case AST_PROGRAM:
            fprintf(out, "#include <stdio.h>\n");
            fprintf(out, "#include <stdlib.h>\n");
            fprintf(out, "#include <stdbool.h>\n");
            fprintf(out, "#include <string.h>\n\n");
            
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
            
            // Add return 0 for main if not present
            if (strcmp(node->data.function.name, "main") == 0) {
                for (int i = 0; i <= indent; i++) fprintf(out, "    ");
                fprintf(out, "return 0;\n");
            }
            
            for (int i = 0; i < indent; i++) fprintf(out, "    ");
            fprintf(out, "}\n");
            break;
            
        case AST_VAR_DECL:
            // Simple type inference
            const char* qualifier = node->data.var_decl.is_mutable ? "" : "const ";
            
            if (node->data.var_decl.value) {
                if (node->data.var_decl.value->type == AST_NUMBER) {
                    fprintf(out, "%sint %s = ", qualifier, node->data.var_decl.name);
                } else if (node->data.var_decl.value->type == AST_STRING) {
                    fprintf(out, "%schar* %s = ", qualifier, node->data.var_decl.name);
                } else if (node->data.var_decl.value->type == AST_BOOL) {
                    fprintf(out, "%sbool %s = ", qualifier, node->data.var_decl.name);
                } else if (node->data.var_decl.value->type == AST_BINARY_OP) {
                    // Assume arithmetic operations result in int for now
                    fprintf(out, "%sint %s = ", qualifier, node->data.var_decl.name);
                } else if (node->data.var_decl.value->type == AST_IDENTIFIER) {
                    // For variables, assume int for now
                    fprintf(out, "%sint %s = ", qualifier, node->data.var_decl.name);
                } else {
                    fprintf(out, "%svoid* %s = ", qualifier, node->data.var_decl.name);
                }
                generate_c_code(node->data.var_decl.value, out, 0);
            } else {
                // Forward declaration
                fprintf(out, "%sint %s", qualifier, node->data.var_decl.name);
            }
            fprintf(out, ";");
            break;
            
        case AST_CALL:
            // Handle special @std functions
            if (node->data.call.func->type == AST_AT_SYMBOL) {
                const char* func_name = node->data.call.func->data.at_symbol.module;
                
                if (strcmp(func_name, "@std.io.println") == 0) {
                    if (node->data.call.arg_count > 0) {
                        // Check if argument is a string
                        if (node->data.call.args[0]->type == AST_STRING) {
                            fprintf(out, "printf(\"%%s\\n\", ");
                            generate_c_code(node->data.call.args[0], out, 0);
                            fprintf(out, ");");
                        } else {
                            // For numbers, identifiers, and binary operations - treat as int
                            fprintf(out, "printf(\"%%d\\n\", ");
                            generate_c_code(node->data.call.args[0], out, 0);
                            fprintf(out, ");");
                        }
                    } else {
                        fprintf(out, "printf(\"\\n\");");
                    }
                } else {
                    fprintf(out, "/* %s not implemented */", func_name);
                }
            } else {
                // Regular function call
                generate_c_code(node->data.call.func, out, 0);
                fprintf(out, "(");
                for (size_t i = 0; i < node->data.call.arg_count; i++) {
                    if (i > 0) fprintf(out, ", ");
                    generate_c_code(node->data.call.args[i], out, 0);
                }
                fprintf(out, ");");
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
            
        case AST_BOOL:
            fprintf(out, "%s", node->data.boolean.value ? "true" : "false");
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
            
        default:
            fprintf(out, "/* Unknown AST node type %d */", node->type);
            break;
    }
}

int main(int argc, char** argv) {
    if (argc < 2) {
        printf("Usage: zenc3 <input.zen> [-o output.c]\n");
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
    
    // Tokenize
    Lexer* lexer = lexer_new(source);
    tokenize(lexer);
    
    // Parse
    Parser* parser = parser_new(lexer);
    ASTNode* ast = parse_program(parser);
    
    if (!ast) {
        fprintf(stderr, "Parse failed\n");
        parser_free(parser);
        lexer_free(lexer);
        free(source);
        return 1;
    }
    
    // Generate C code
    FILE* output = fopen(output_name, "w");
    if (!output) {
        fprintf(stderr, "Error: Cannot create output file %s\n", output_name);
        parser_free(parser);
        lexer_free(lexer);
        free(source);
        return 1;
    }
    
    generate_c_code(ast, output, 0);
    fclose(output);
    
    printf("Generated %s\n", output_name);
    
    // Compile the C code
    char compile_cmd[512];
    snprintf(compile_cmd, sizeof(compile_cmd), "gcc -std=c99 -o %s.out %s 2>&1", output_name, output_name);
    int result = system(compile_cmd);
    
    if (result == 0) {
        printf("Compiled to %s.out\n", output_name);
    } else {
        printf("Compilation had warnings or errors\n");
    }
    
    // Cleanup
    parser_free(parser);
    lexer_free(lexer);
    free(source);
    
    return 0;
}