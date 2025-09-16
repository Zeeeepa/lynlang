// Zen Compiler v2 - Implements LANGUAGE_SPEC.zen features
// This is an enhanced C implementation to compile Zen programs per the spec

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
            while (isalnum(peek(lex)) || peek(lex) == '_') {
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
        
        // Strings with interpolation support
        if (c == '"') {
            advance(lex); // skip opening quote
            tok.type = TOK_STRING;
            
            // Build string value, checking for interpolation
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
                } else if (peek(lex) == '$' && peek_next(lex) == '{') {
                    // Mark as string interpolation
                    tok.type = TOK_STRING_INTERP_START;
                    // TODO: Handle string interpolation properly
                    advance(lex);
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
        
        // Handle @std.io.println etc
        while (match_token(p, TOK_DOT)) {
            if (current_token(p)->type == TOK_IDENTIFIER) {
                size_t len = strlen(node->data.at_symbol.module) + 
                            strlen(current_token(p)->value) + 2;
                char* new_path = malloc(len);
                snprintf(new_path, len, "%s.%s", 
                        node->data.at_symbol.module, 
                        current_token(p)->value);
                free(node->data.at_symbol.module);
                node->data.at_symbol.module = new_path;
                advance_token(p);
            }
        }
        
        // Handle function call
        if (match_token(p, TOK_LPAREN)) {
            ASTNode* call = malloc(sizeof(ASTNode));
            call->type = AST_CALL;
            call->data.call.func = node;
            call->data.call.args = malloc(sizeof(ASTNode*) * 10);
            call->data.call.arg_count = 0;
            
            while (!match_token(p, TOK_RPAREN)) {
                if (call->data.call.arg_count > 0) {
                    expect_token(p, TOK_COMMA, "Expected ',' between arguments");
                }
                call->data.call.args[call->data.call.arg_count++] = parse_expression(p);
                
                if (current_token(p)->type == TOK_RPAREN) {
                    advance_token(p);
                    break;
                }
            }
            
            return call;
        }
        
        return node;
    }
    
    // Identifiers
    if (tok->type == TOK_IDENTIFIER) {
        ASTNode* node = malloc(sizeof(ASTNode));
        
        // Check for Some(value) or None
        if (strcmp(tok->value, "Some") == 0 && peek_token(p)->type == TOK_LPAREN) {
            node->type = AST_OPTION_SOME;
            advance_token(p);
            expect_token(p, TOK_LPAREN, "Expected '(' after Some");
            node->data.option_some.value = parse_expression(p);
            expect_token(p, TOK_RPAREN, "Expected ')' after Some value");
            return node;
        }
        
        if (strcmp(tok->value, "None") == 0) {
            node->type = AST_OPTION_NONE;
            advance_token(p);
            return node;
        }
        
        node->type = AST_IDENTIFIER;
        node->data.literal.value = strdup(tok->value);
        advance_token(p);
        
        // Check for struct literal
        if (current_token(p)->type == TOK_LBRACE) {
            ASTNode* struct_lit = malloc(sizeof(ASTNode));
            struct_lit->type = AST_STRUCT_LITERAL;
            struct_lit->data.struct_lit.type_name = node->data.literal.value;
            struct_lit->data.struct_lit.fields = malloc(sizeof(ASTNode*) * 20);
            struct_lit->data.struct_lit.field_names = malloc(sizeof(char*) * 20);
            struct_lit->data.struct_lit.field_count = 0;
            
            advance_token(p); // consume {
            
            while (!match_token(p, TOK_RBRACE)) {
                if (struct_lit->data.struct_lit.field_count > 0) {
                    expect_token(p, TOK_COMMA, "Expected ',' between struct fields");
                }
                
                // Field name
                if (current_token(p)->type != TOK_IDENTIFIER) {
                    error("Expected field name in struct literal");
                    break;
                }
                struct_lit->data.struct_lit.field_names[struct_lit->data.struct_lit.field_count] = 
                    strdup(current_token(p)->value);
                advance_token(p);
                
                expect_token(p, TOK_COLON, "Expected ':' after field name");
                
                // Field value
                struct_lit->data.struct_lit.fields[struct_lit->data.struct_lit.field_count] = 
                    parse_expression(p);
                struct_lit->data.struct_lit.field_count++;
                
                if (current_token(p)->type == TOK_RBRACE) {
                    advance_token(p);
                    break;
                }
            }
            
            free(node);
            return struct_lit;
        }
        
        // Check for function call
        if (match_token(p, TOK_LPAREN)) {
            ASTNode* call = malloc(sizeof(ASTNode));
            call->type = AST_CALL;
            call->data.call.func = node;
            call->data.call.args = malloc(sizeof(ASTNode*) * 10);
            call->data.call.arg_count = 0;
            
            while (!match_token(p, TOK_RPAREN)) {
                if (call->data.call.arg_count > 0) {
                    expect_token(p, TOK_COMMA, "Expected ',' between arguments");
                }
                call->data.call.args[call->data.call.arg_count++] = parse_expression(p);
                
                if (current_token(p)->type == TOK_RPAREN) {
                    advance_token(p);
                    break;
                }
            }
            
            return call;
        }
        
        // Check for member access
        while (match_token(p, TOK_DOT)) {
            ASTNode* member = malloc(sizeof(ASTNode));
            member->type = AST_MEMBER_ACCESS;
            member->data.member.object = node;
            
            if (current_token(p)->type != TOK_IDENTIFIER && 
                strcmp(current_token(p)->value, "loop") != 0) {
                error("Expected member name after '.'");
                break;
            }
            member->data.member.member = strdup(current_token(p)->value);
            advance_token(p);
            
            // Check for method call like .loop()
            if (match_token(p, TOK_LPAREN)) {
                ASTNode* call = malloc(sizeof(ASTNode));
                call->type = AST_CALL;
                call->data.call.func = member;
                call->data.call.args = malloc(sizeof(ASTNode*) * 10);
                call->data.call.arg_count = 0;
                
                while (!match_token(p, TOK_RPAREN)) {
                    if (call->data.call.arg_count > 0) {
                        expect_token(p, TOK_COMMA, "Expected ',' between arguments");
                    }
                    call->data.call.args[call->data.call.arg_count++] = parse_expression(p);
                    
                    if (current_token(p)->type == TOK_RPAREN) {
                        advance_token(p);
                        break;
                    }
                }
                
                node = call;
            } else {
                node = member;
            }
        }
        
        return node;
    }
    
    // Parenthesized expressions or ranges
    if (match_token(p, TOK_LPAREN)) {
        ASTNode* expr = parse_expression(p);
        
        // Check for range syntax (start..end)
        if (match_token(p, TOK_DOUBLE_DOT)) {
            ASTNode* range = malloc(sizeof(ASTNode));
            range->type = AST_RANGE;
            range->data.range.start = expr;
            range->data.range.end = parse_expression(p);
            range->data.range.step = NULL;
            
            expect_token(p, TOK_RPAREN, "Expected ')' after range");
            
            // Check for .step() or .loop()
            if (match_token(p, TOK_DOT)) {
                if (current_token(p)->type == TOK_IDENTIFIER) {
                    if (strcmp(current_token(p)->value, "loop") == 0) {
                        advance_token(p);
                        expect_token(p, TOK_LPAREN, "Expected '(' after loop");
                        
                        // Parse loop body (closure)
                        ASTNode* loop = malloc(sizeof(ASTNode));
                        loop->type = AST_LOOP;
                        loop->data.loop.condition = range;
                        loop->data.loop.body = parse_expression(p);
                        
                        expect_token(p, TOK_RPAREN, "Expected ')' after loop body");
                        return loop;
                    }
                }
            }
            
            return range;
        }
        
        expect_token(p, TOK_RPAREN, "Expected ')' after expression");
        return expr;
    }
    
    // loop() syntax
    if (current_token(p)->type == TOK_KEYWORD && 
        strcmp(current_token(p)->value, "loop") == 0) {
        advance_token(p);
        expect_token(p, TOK_LPAREN, "Expected '(' after loop");
        
        ASTNode* loop = malloc(sizeof(ASTNode));
        loop->type = AST_LOOP;
        loop->data.loop.condition = NULL;
        loop->data.loop.body = parse_expression(p);
        
        expect_token(p, TOK_RPAREN, "Expected ')' after loop body");
        return loop;
    }
    
    error("Unexpected token in primary expression: %d", tok->type);
    return NULL;
}

// Parse binary expressions
ASTNode* parse_binary_expr(Parser* p, int min_prec) {
    ASTNode* left = parse_primary(p);
    
    while (true) {
        Token* op = current_token(p);
        
        // Check for operators
        int prec = 0;
        switch (op->type) {
            case TOK_STAR:
            case TOK_SLASH:
            case TOK_PERCENT:
                prec = 10;
                break;
            case TOK_PLUS:
            case TOK_MINUS:
                prec = 9;
                break;
            case TOK_LESS:
            case TOK_GREATER:
            case TOK_LESS_EQUAL:
            case TOK_GREATER_EQUAL:
                prec = 7;
                break;
            case TOK_EQUAL:
            case TOK_NOT_EQUAL:
                prec = 6;
                break;
            case TOK_QUESTION:
                prec = 3;
                break;
            default:
                return left;
        }
        
        if (prec < min_prec) {
            return left;
        }
        
        // Handle pattern matching with ?
        if (op->type == TOK_QUESTION) {
            advance_token(p);
            
            ASTNode* pattern_match = malloc(sizeof(ASTNode));
            pattern_match->type = AST_PATTERN_MATCH;
            pattern_match->data.pattern_match.expr = left;
            pattern_match->data.pattern_match.arms = malloc(sizeof(ASTNode*) * 10);
            pattern_match->data.pattern_match.arm_count = 0;
            
            // Check for simple boolean pattern or block
            if (current_token(p)->type == TOK_LBRACE) {
                // Simple boolean true branch
                ASTNode* arm = malloc(sizeof(ASTNode));
                arm->type = AST_PATTERN_ARM;
                
                // Create a "true" pattern
                ASTNode* true_pat = malloc(sizeof(ASTNode));
                true_pat->type = AST_BOOL;
                true_pat->data.boolean.value = true;
                arm->data.pattern_arm.pattern = true_pat;
                
                advance_token(p); // consume {
                arm->data.pattern_arm.body = parse_block(p);
                
                pattern_match->data.pattern_match.arms[pattern_match->data.pattern_match.arm_count++] = arm;
            } else {
                // Full pattern matching with pipes
                while (true) {
                    // Expect pipe for each arm (except maybe first)
                    if (pattern_match->data.pattern_match.arm_count > 0 || 
                        current_token(p)->type == TOK_PIPE) {
                        if (!match_token(p, TOK_PIPE)) {
                            break;
                        }
                    }
                    
                    ASTNode* arm = malloc(sizeof(ASTNode));
                    arm->type = AST_PATTERN_ARM;
                    
                    // Parse pattern
                    arm->data.pattern_arm.pattern = parse_expression(p);
                    
                    // Expect block
                    if (current_token(p)->type != TOK_LBRACE) {
                        error("Expected '{' after pattern");
                        break;
                    }
                    advance_token(p);
                    arm->data.pattern_arm.body = parse_block(p);
                    
                    pattern_match->data.pattern_match.arms[pattern_match->data.pattern_match.arm_count++] = arm;
                    
                    // Check if there are more arms
                    if (current_token(p)->type != TOK_PIPE) {
                        break;
                    }
                }
            }
            
            return pattern_match;
        }
        
        // Regular binary operators
        advance_token(p);
        
        ASTNode* right = parse_binary_expr(p, prec + 1);
        
        ASTNode* binary = malloc(sizeof(ASTNode));
        binary->type = AST_BINARY_OP;
        binary->data.binary.left = left;
        binary->data.binary.right = right;
        
        switch (op->type) {
            case TOK_PLUS: binary->data.binary.op = "+"; break;
            case TOK_MINUS: binary->data.binary.op = "-"; break;
            case TOK_STAR: binary->data.binary.op = "*"; break;
            case TOK_SLASH: binary->data.binary.op = "/"; break;
            case TOK_PERCENT: binary->data.binary.op = "%"; break;
            case TOK_LESS: binary->data.binary.op = "<"; break;
            case TOK_GREATER: binary->data.binary.op = ">"; break;
            case TOK_LESS_EQUAL: binary->data.binary.op = "<="; break;
            case TOK_GREATER_EQUAL: binary->data.binary.op = ">="; break;
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
        
        if (current_token(p)->type != TOK_RBRACE) {
            ret->data.ret.value = parse_expression(p);
        } else {
            ret->data.ret.value = NULL;
        }
        
        return ret;
    }
    
    // Handle break statement
    if (tok->type == TOK_KEYWORD && strcmp(tok->value, "break") == 0) {
        advance_token(p);
        ASTNode* brk = malloc(sizeof(ASTNode));
        brk->type = AST_BREAK;
        return brk;
    }
    
    // Handle continue statement
    if (tok->type == TOK_KEYWORD && strcmp(tok->value, "continue") == 0) {
        advance_token(p);
        ASTNode* cont = malloc(sizeof(ASTNode));
        cont->type = AST_CONTINUE;
        return cont;
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
            
            // Parse parameters
            while (!match_token(p, TOK_RPAREN)) {
                if (func->data.function.param_count > 0) {
                    expect_token(p, TOK_COMMA, "Expected ',' between parameters");
                }
                // TODO: Parse parameter properly
                advance_token(p);
            }
            
            // Parse return type
            if (current_token(p)->type == TOK_IDENTIFIER) {
                func->data.function.return_type = strdup(current_token(p)->value);
                advance_token(p);
            }
            
            // Parse body
            expect_token(p, TOK_LBRACE, "Expected '{' for function body");
            func->data.function.body = parse_block(p);
            
            return func;
        }
        
        // Variable declaration with type: x: type = value
        if (current_token(p)->type == TOK_COLON) {
            advance_token(p); // consume :
            
            ASTNode* var = malloc(sizeof(ASTNode));
            var->type = AST_VAR_DECL;
            var->data.var_decl.name = name;
            var->data.var_decl.is_mutable = false;
            var->data.var_decl.has_type = true;
            
            // Parse type
            if (current_token(p)->type == TOK_IDENTIFIER) {
                var->data.var_decl.type_name = strdup(current_token(p)->value);
                advance_token(p);
            }
            
            // Optional assignment
            if (match_token(p, TOK_ASSIGN)) {
                var->data.var_decl.value = parse_expression(p);
            } else {
                var->data.var_decl.value = NULL;
            }
            
            return var;
        }
        
        // Mutable variable with type: x :: type = value or x :: type
        if (current_token(p)->type == TOK_COLON_COLON) {
            advance_token(p); // consume ::
            
            ASTNode* var = malloc(sizeof(ASTNode));
            var->type = AST_VAR_DECL;
            var->data.var_decl.name = name;
            var->data.var_decl.is_mutable = true;
            var->data.var_decl.has_type = false;
            
            // Check if type is specified
            if (current_token(p)->type == TOK_IDENTIFIER) {
                var->data.var_decl.has_type = true;
                var->data.var_decl.type_name = strdup(current_token(p)->value);
                advance_token(p);
                
                // Optional assignment
                if (match_token(p, TOK_ASSIGN)) {
                    var->data.var_decl.value = parse_expression(p);
                } else {
                    var->data.var_decl.value = NULL;
                }
            } else {
                // Just forward declaration: x ::
                var->data.var_decl.value = NULL;
            }
            
            return var;
        }
        
        // Mutable assignment without type: x ::= value
        if (current_token(p)->type == TOK_COLON_COLON_ASSIGN) {
            advance_token(p); // consume ::=
            
            ASTNode* var = malloc(sizeof(ASTNode));
            var->type = AST_VAR_DECL;
            var->data.var_decl.name = name;
            var->data.var_decl.is_mutable = true;
            var->data.var_decl.has_type = false;
            var->data.var_decl.value = parse_expression(p);
            
            return var;
        }
        
        // Immutable assignment without type: x = value
        if (current_token(p)->type == TOK_ASSIGN) {
            advance_token(p); // consume =
            
            // Check if it's an assignment to existing variable or new declaration
            // For now, treat as new declaration
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
        block->data.block.statements[block->data.block.count++] = parse_statement(p);
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
        program->data.program.statements[program->data.program.count++] = parse_statement(p);
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
            
            // Generate Option type helpers
            fprintf(out, "// Option type helpers\n");
            fprintf(out, "typedef struct { bool is_some; void* value; } Option;\n");
            fprintf(out, "Option Some(void* v) { Option o; o.is_some = true; o.value = v; return o; }\n");
            fprintf(out, "Option None() { Option o; o.is_some = false; o.value = NULL; return o; }\n\n");
            
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
            if (node->data.var_decl.has_type) {
                // Map Zen types to C types
                const char* c_type = "int"; // default
                if (node->data.var_decl.type_name) {
                    if (strcmp(node->data.var_decl.type_name, "i32") == 0) c_type = "int";
                    else if (strcmp(node->data.var_decl.type_name, "i64") == 0) c_type = "long long";
                    else if (strcmp(node->data.var_decl.type_name, "f32") == 0) c_type = "float";
                    else if (strcmp(node->data.var_decl.type_name, "f64") == 0) c_type = "double";
                    else if (strcmp(node->data.var_decl.type_name, "bool") == 0) c_type = "bool";
                    else if (strcmp(node->data.var_decl.type_name, "string") == 0) c_type = "const char*";
                }
                
                if (node->data.var_decl.is_mutable) {
                    fprintf(out, "%s %s", c_type, node->data.var_decl.name);
                } else {
                    fprintf(out, "const %s %s", c_type, node->data.var_decl.name);
                }
                
                if (node->data.var_decl.value) {
                    fprintf(out, " = ");
                    generate_c_code(node->data.var_decl.value, out, 0);
                }
            } else {
                // Infer type from value
                const char* qualifier = node->data.var_decl.is_mutable ? "" : "const ";
                
                if (node->data.var_decl.value) {
                    // Simple type inference
                    if (node->data.var_decl.value->type == AST_NUMBER) {
                        fprintf(out, "%sint %s = ", qualifier, node->data.var_decl.name);
                    } else if (node->data.var_decl.value->type == AST_STRING) {
                        fprintf(out, "%schar* %s = ", qualifier, node->data.var_decl.name);
                    } else if (node->data.var_decl.value->type == AST_BOOL) {
                        fprintf(out, "%sbool %s = ", qualifier, node->data.var_decl.name);
                    } else {
                        fprintf(out, "%svoid* %s = ", qualifier, node->data.var_decl.name);
                    }
                    generate_c_code(node->data.var_decl.value, out, 0);
                } else {
                    // Forward declaration
                    fprintf(out, "%sint %s", qualifier, node->data.var_decl.name);
                }
            }
            fprintf(out, ";");
            break;
            
        case AST_ASSIGNMENT:
            fprintf(out, "%s = ", node->data.assignment.target->data.literal.value);
            generate_c_code(node->data.assignment.value, out, 0);
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
                            fprintf(out, ")");
                        } else {
                            fprintf(out, "printf(\"%%d\\n\", ");
                            generate_c_code(node->data.call.args[0], out, 0);
                            fprintf(out, ")");
                        }
                    } else {
                        fprintf(out, "printf(\"\\n\")");
                    }
                } else if (strcmp(func_name, "@std.io.print") == 0) {
                    if (node->data.call.arg_count > 0) {
                        fprintf(out, "printf(\"%%s\", ");
                        generate_c_code(node->data.call.args[0], out, 0);
                        fprintf(out, ")");
                    }
                } else {
                    fprintf(out, "/* %s not implemented */", func_name);
                }
            } else if (node->data.call.func->type == AST_MEMBER_ACCESS) {
                // Handle .loop() calls
                if (strcmp(node->data.call.func->data.member.member, "loop") == 0) {
                    fprintf(out, "/* .loop() not yet implemented */");
                } else {
                    // Regular method call
                    generate_c_code(node->data.call.func->data.member.object, out, 0);
                    fprintf(out, "_%s(", node->data.call.func->data.member.member);
                    for (size_t i = 0; i < node->data.call.arg_count; i++) {
                        if (i > 0) fprintf(out, ", ");
                        generate_c_code(node->data.call.args[i], out, 0);
                    }
                    fprintf(out, ")");
                }
            } else {
                // Regular function call
                generate_c_code(node->data.call.func, out, 0);
                fprintf(out, "(");
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
            
        case AST_BREAK:
            fprintf(out, "break;");
            break;
            
        case AST_CONTINUE:
            fprintf(out, "continue;");
            break;
            
        case AST_PATTERN_MATCH:
            // Generate if-else chain for pattern matching
            for (size_t i = 0; i < node->data.pattern_match.arm_count; i++) {
                ASTNode* arm = node->data.pattern_match.arms[i];
                
                if (i == 0) {
                    fprintf(out, "if (");
                } else {
                    fprintf(out, " else if (");
                }
                
                // Generate condition
                if (arm->data.pattern_arm.pattern->type == AST_BOOL) {
                    // For boolean patterns, check against the expression
                    generate_c_code(node->data.pattern_match.expr, out, 0);
                    if (arm->data.pattern_arm.pattern->data.boolean.value) {
                        // true pattern
                    } else {
                        fprintf(out, " == false");
                    }
                } else if (arm->data.pattern_arm.pattern->type == AST_IDENTIFIER) {
                    if (strcmp(arm->data.pattern_arm.pattern->data.literal.value, "true") == 0) {
                        generate_c_code(node->data.pattern_match.expr, out, 0);
                    } else if (strcmp(arm->data.pattern_arm.pattern->data.literal.value, "false") == 0) {
                        fprintf(out, "!");
                        generate_c_code(node->data.pattern_match.expr, out, 0);
                    } else if (strcmp(arm->data.pattern_arm.pattern->data.literal.value, "_") == 0) {
                        fprintf(out, "true"); // Default case
                    } else {
                        // Match against specific value
                        generate_c_code(node->data.pattern_match.expr, out, 0);
                        fprintf(out, " == ");
                        generate_c_code(arm->data.pattern_arm.pattern, out, 0);
                    }
                } else {
                    // Compare expression with pattern
                    generate_c_code(node->data.pattern_match.expr, out, 0);
                    fprintf(out, " == ");
                    generate_c_code(arm->data.pattern_arm.pattern, out, 0);
                }
                
                fprintf(out, ") {\n");
                generate_c_code(arm->data.pattern_arm.body, out, indent + 1);
                for (int j = 0; j < indent; j++) fprintf(out, "    ");
                fprintf(out, "}");
            }
            break;
            
        case AST_LOOP:
            if (node->data.loop.condition && node->data.loop.condition->type == AST_RANGE) {
                // For range loops
                ASTNode* range = node->data.loop.condition;
                fprintf(out, "for (int _i = ");
                generate_c_code(range->data.range.start, out, 0);
                fprintf(out, "; _i < ");
                generate_c_code(range->data.range.end, out, 0);
                fprintf(out, "; _i++) {\n");
                generate_c_code(node->data.loop.body, out, indent + 1);
                for (int j = 0; j < indent; j++) fprintf(out, "    ");
                fprintf(out, "}");
            } else {
                // Infinite loop
                fprintf(out, "while (true) {\n");
                generate_c_code(node->data.loop.body, out, indent + 1);
                for (int j = 0; j < indent; j++) fprintf(out, "    ");
                fprintf(out, "}");
            }
            break;
            
        case AST_RANGE:
            fprintf(out, "/* Range not directly supported in C */");
            break;
            
        case AST_STRUCT_LITERAL:
            fprintf(out, "/* Struct literals not yet implemented */");
            break;
            
        case AST_OPTION_SOME:
            fprintf(out, "Some(");
            generate_c_code(node->data.option_some.value, out, 0);
            fprintf(out, ")");
            break;
            
        case AST_OPTION_NONE:
            fprintf(out, "None()");
            break;
            
        default:
            fprintf(out, "/* Unknown AST node type %d */", node->type);
            break;
    }
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
    
    // Tokenize
    Lexer* lexer = lexer_new(source);
    tokenize(lexer);
    
    // Parse
    Parser* parser = parser_new(lexer);
    ASTNode* ast = parse_program(parser);
    
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
    char compile_cmd[256];
    snprintf(compile_cmd, sizeof(compile_cmd), "gcc -o %s.out %s 2>&1", output_name, output_name);
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