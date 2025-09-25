use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Identifier(String),
    Integer(String),
    Float(String),
    StringLiteral(String),
    Symbol(char),
    Operator(String),
    Question,           // ?
    Pipe,               // |
    Underscore,         // _ (for wildcard patterns)
    AtStd,              // @std
    AtThis,             // @this
    AtMeta,             // @meta (for compile-time metaprogramming)
    #[allow(dead_code)]
    InterpolationStart, // Start of ${...}
    #[allow(dead_code)]
    InterpolationEnd,   // End of ${...}
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenWithSpan {
    pub token: Token,
    pub span: Span,
}

#[derive(Clone)]
pub struct Lexer<'a> {
    pub input: &'a str,
    pub position: usize,
    pub read_position: usize,
    pub current_char: Option<char>,
    pub line: usize,
    pub column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input,
            position: 0,
            read_position: 0,
            current_char: None,
            line: 1,
            column: 0, // Start at 0 for 0-based column tracking
        };
        lexer.read_char();
        lexer
    }

    fn read_char(&mut self) {
        if self.read_position >= self.input.len() {
            self.current_char = None;
            self.position = self.read_position;
        } else if let Some((_idx, ch)) = self.input[self.read_position..].char_indices().next() {
            self.current_char = Some(ch);
            self.position = self.read_position;
            self.read_position += ch.len_utf8();

            // Update line and column tracking
            if ch == '\n' {
                self.line += 1;
                self.column = 0; // Reset to 0 for 0-based columns
            } else {
                self.column += 1;
            }
        } else {
            self.current_char = None;
            self.position = self.read_position;
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.next_token_with_span().token
    }

    pub fn next_token_with_span(&mut self) -> TokenWithSpan {
        self.skip_whitespace_and_comments();

        // Record position AFTER skipping whitespace/comments
        let start_pos = self.position;
        let start_line = self.line;
        // Column is already 0-based now
        let start_column = self.column;

        let token = match self.current_char {
            Some('@') => {
                // Handle @std, @this, and @meta special symbols
                let start = self.position;
                self.read_char(); // consume '@'

                // Read the identifier part after @
                let ident_start = self.position;
                while let Some(c) = self.current_char {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        self.read_char();
                    } else {
                        break;
                    }
                }
                let ident = self.input[ident_start..self.position].to_string();

                // Check for special @ tokens
                // Support @std, @this, and @meta with optional dot-notation
                if ident == "std" {
                    Token::AtStd
                } else if ident == "this" {
                    Token::AtThis
                } else if ident == "meta" {
                    Token::AtMeta
                } else {
                    // For other @ identifiers, return as regular identifier with @
                    Token::Identifier(self.input[start..self.position].to_string())
                }
            }
            Some('_') => {
                // Check if it's just underscore (wildcard) or part of identifier
                let start = self.position;
                self.read_char();
                if let Some(c) = self.current_char {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        // Part of identifier, continue reading
                        while let Some(ch) = self.current_char {
                            if ch.is_ascii_alphanumeric() || ch == '_' {
                                self.read_char();
                            } else {
                                break;
                            }
                        }
                        Token::Identifier(self.input[start..self.position].to_string())
                    } else {
                        // Just underscore (wildcard)
                        Token::Underscore
                    }
                } else {
                    Token::Underscore
                }
            }
            Some(c) if c.is_ascii_alphabetic() => {
                let ident = self.read_identifier();
                // No keywords in Zen - all are identifiers
                Token::Identifier(ident)
            }
            Some(c) if c.is_ascii_digit() => {
                let number = self.read_number();
                if number.contains('.') {
                    Token::Float(number)
                } else {
                    Token::Integer(number)
                }
            }
            Some('"') => {
                let string = self.read_string();
                Token::StringLiteral(string)
            }
            Some(':') => {
                // Check for multi-character operators (:=, ::, ::=)
                if let Some(next) = self.peek_char() {
                    if next == '=' {
                        self.read_char(); // consume ':'
                        self.read_char(); // consume '='
                        return TokenWithSpan {
                            token: Token::Operator(":=".to_string()),
                            span: Span {
                                start: start_pos,
                                end: self.position,
                                line: start_line,
                                column: start_column,
                            },
                        };
                    } else if next == ':' {
                        self.read_char(); // consume ':'
                        if let Some(next2) = self.peek_char() {
                            if next2 == '=' {
                                self.read_char(); // consume second ':'
                                self.read_char(); // consume '='
                                return TokenWithSpan {
                                    token: Token::Operator("::=".to_string()),
                                    span: Span {
                                        start: start_pos,
                                        end: self.position,
                                        line: start_line,
                                        column: start_column,
                                    },
                                };
                            }
                        }
                        self.read_char(); // consume second ':'
                        return TokenWithSpan {
                            token: Token::Operator("::".to_string()),
                            span: Span {
                                start: start_pos,
                                end: self.position,
                                line: start_line,
                                column: start_column,
                            },
                        };
                    }
                }
                self.read_char();
                Token::Symbol(':')
            }
            Some('.') => {
                // Check for range operators (.., ..=) and varargs (...)
                if let Some(next) = self.peek_char() {
                    if next == '.' {
                        self.read_char(); // consume first '.'
                        if let Some(next2) = self.peek_char() {
                            if next2 == '.' {
                                self.read_char(); // consume second '.'
                                self.read_char(); // consume third '.'
                                return TokenWithSpan {
                                    token: Token::Operator("...".to_string()),
                                    span: Span {
                                        start: start_pos,
                                        end: self.position,
                                        line: start_line,
                                        column: start_column,
                                    },
                                };
                            } else if next2 == '=' {
                                self.read_char(); // consume second '.'
                                self.read_char(); // consume '='
                                return TokenWithSpan {
                                    token: Token::Operator("..=".to_string()),
                                    span: Span {
                                        start: start_pos,
                                        end: self.position,
                                        line: start_line,
                                        column: start_column,
                                    },
                                };
                            }
                        }
                        self.read_char(); // consume second '.'
                        return TokenWithSpan {
                            token: Token::Operator("..".to_string()),
                            span: Span {
                                start: start_pos,
                                end: self.position,
                                line: start_line,
                                column: start_column,
                            },
                        };
                    }
                }
                self.read_char();
                Token::Symbol('.')
            }
            Some('?') => {
                self.read_char();
                Token::Question
            }
            Some('|') => {
                // Check for '||' operator
                if let Some(next) = self.peek_char() {
                    if next == '|' {
                        self.read_char(); // consume '|'
                        self.read_char(); // consume second '|'
                        return TokenWithSpan {
                            token: Token::Operator("||".to_string()),
                            span: Span {
                                start: start_pos,
                                end: self.position,
                                line: start_line,
                                column: start_column,
                            },
                        };
                    }
                }
                // Single pipe for pattern matching
                self.read_char();
                Token::Pipe
            }
            Some('&') => {
                // Check for '&&' operator
                if let Some(next) = self.peek_char() {
                    if next == '&' {
                        self.read_char(); // consume '&'
                        self.read_char(); // consume second '&'
                        return TokenWithSpan {
                            token: Token::Operator("&&".to_string()),
                            span: Span {
                                start: start_pos,
                                end: self.position,
                                line: start_line,
                                column: start_column,
                            },
                        };
                    }
                }
                self.read_char();
                Token::Symbol('&')
            }
            Some('<') => {
                self.read_char();
                if self.current_char == Some('=') {
                    self.read_char();
                    Token::Operator("<=".to_string())
                } else {
                    Token::Operator("<".to_string())
                }
            }
            Some('>') => {
                self.read_char();
                if self.current_char == Some('=') {
                    self.read_char();
                    Token::Operator(">=".to_string())
                } else {
                    Token::Operator(">".to_string())
                }
            }
            Some('!') => {
                self.read_char();
                if self.current_char == Some('=') {
                    self.read_char();
                    Token::Operator("!=".to_string())
                } else {
                    Token::Symbol('!')
                }
            }
            Some(c) if self.is_symbol(c) => {
                self.read_char();
                Token::Symbol(c)
            }
            Some('-') => {
                self.read_char();
                if self.current_char == Some('>') {
                    self.read_char();
                    Token::Operator("->".to_string())
                } else {
                    Token::Operator("-".to_string())
                }
            }
            Some('=') => {
                self.read_char();
                if self.current_char == Some('=') {
                    self.read_char();
                    Token::Operator("==".to_string())
                } else if self.current_char == Some('>') {
                    self.read_char();
                    Token::Operator("=>".to_string())
                } else {
                    Token::Operator("=".to_string())
                }
            }
            Some(c) if self.is_operator_start(c) => {
                let op = self.read_operator();
                Token::Operator(op)
            }
            None => Token::Eof,
            _ => {
                self.read_char();
                return self.next_token_with_span();
            }
        };

        TokenWithSpan {
            token,
            span: Span {
                start: start_pos,
                end: self.position,
                line: start_line,
                column: start_column,
            },
        }
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(c) = self.current_char {
                if c.is_whitespace() {
                    self.read_char();
                } else {
                    break;
                }
            }
            // Skip single-line comments
            if self.current_char == Some('/') && self.peek_char() == Some('/') {
                // Skip '//'
                self.read_char();
                self.read_char();
                // Skip until end of line or input
                while let Some(c) = self.current_char {
                    if c == '\n' {
                        break;
                    }
                    self.read_char();
                }
                continue;
            }
            // Skip multi-line comments
            if self.current_char == Some('/') && self.peek_char() == Some('*') {
                // Skip '/*'
                self.read_char();
                self.read_char();
                // Skip until '*/' or end of input
                while let Some(c) = self.current_char {
                    if c == '*' && self.peek_char() == Some('/') {
                        self.read_char(); // skip '*'
                        self.read_char(); // skip '/'
                        break;
                    }
                    self.read_char();
                }
                continue;
            }
            break;
        }
    }

    fn read_identifier(&mut self) -> String {
        let start = self.position;
        while let Some(c) = self.current_char {
            if c.is_ascii_alphanumeric() || c == '_' || c == '@' {
                self.read_char();
            } else {
                break;
            }
        }
        // When we exit the loop, self.position might be pointing to the last character
        // we want to include, not past it. The issue is that after the last read_char(),
        // self.position is updated to the current position, not past the character.
        // Actually, self.position should be correct since read_char() updates it.
        self.input[start..self.position].to_string()
    }

    fn read_number(&mut self) -> String {
        let start = self.position;
        let mut has_dot = false;

        while let Some(c) = self.current_char {
            if c.is_ascii_digit() {
                self.read_char();
            } else if c == '.' && !has_dot {
                // Only consume '.' if it's followed by a digit (for floats like 3.14)
                // This prevents consuming '..' as part of a number
                if let Some(next_c) = self.peek_char() {
                    if next_c.is_ascii_digit() {
                        has_dot = true;
                        self.read_char();
                    } else {
                        // Don't consume '.' if not followed by digit
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        self.input[start..self.position].to_string()
    }

    fn is_symbol(&self, c: char) -> bool {
        matches!(c, '{' | '}' | '(' | ')' | '[' | ']' | ';' | ',')
    }

    fn is_operator_start(&self, c: char) -> bool {
        matches!(c, '+' | '*' | '/' | '%' | '!' | '<' | '>' | '&' | '|' | ':')
    }

    fn read_string(&mut self) -> String {
        self.read_char(); // consume opening quote
        let mut result = String::new();
        let mut escape_next = false;

        while let Some(c) = self.current_char {
            if escape_next {
                // Handle escape sequences
                match c {
                    'n' => result.push('\n'),
                    't' => result.push('\t'),
                    'r' => result.push('\r'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    '$' => result.push('$'), // Allow escaping $ for literal $
                    _ => {
                        result.push('\\');
                        result.push(c);
                    }
                }
                escape_next = false;
                self.read_char();
            } else if c == '\\' {
                escape_next = true;
                self.read_char();
            } else if c == '"' {
                break;
            } else if c == '$' && self.peek_char() == Some('{') {
                // Mark string interpolation point
                result.push('\x01'); // Special marker for interpolation
                self.read_char(); // consume $
                self.read_char(); // consume {

                // Read the interpolated expression until we find '}'
                let mut depth = 1;
                while let Some(ch) = self.current_char {
                    if ch == '{' {
                        depth += 1;
                    } else if ch == '}' {
                        depth -= 1;
                        if depth == 0 {
                            self.read_char(); // consume final }
                            break;
                        }
                    }
                    result.push(ch);
                    self.read_char();
                }
                result.push('\x02'); // End marker for interpolation
            } else {
                result.push(c);
                self.read_char();
            }
        }
        self.read_char(); // consume closing quote
        result
    }

    fn read_operator(&mut self) -> String {
        let _start = self.position;
        let first_char = self.current_char.unwrap();
        self.read_char();

        // Handle three-character operators first
        if let Some(second_char) = self.current_char {
            if let Some(third_char) = self.peek_char() {
                let three_char_op = format!("{}{}{}", first_char, second_char, third_char);
                if matches!(three_char_op.as_str(), "::=" | "..=") {
                    self.read_char(); // consume second char
                    self.read_char(); // consume third char
                    return three_char_op;
                }
            }
        }

        // Handle two-character operators
        if let Some(second_char) = self.current_char {
            let two_char_op = format!("{}{}", first_char, second_char);
            if matches!(
                two_char_op.as_str(),
                "==" | "!=" | "<=" | ">=" | "&&" | "||" | ":=" | "::" | ".." | "..="
            ) {
                self.read_char();
                return two_char_op;
            }
        }

        // Single character operator
        first_char.to_string()
    }

    fn peek_char(&self) -> Option<char> {
        if self.read_position >= self.input.len() {
            None
        } else {
            self.input[self.read_position..].chars().next()
        }
    }
}
