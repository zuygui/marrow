use crate::error::CompileError;
use crate::token::{Token, TokenKind};

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), CompileError> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.advance();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    let (start_line, start_col) = (self.line, self.col);
                    self.advance();
                    self.advance();
                    let mut closed = false;
                    while let Some(c) = self.peek() {
                        if c == '*' && self.peek_at(1) == Some('/') {
                            self.advance();
                            self.advance();
                            closed = true;
                            break;
                        }
                        self.advance();
                    }
                    if !closed {
                        return Err(CompileError::new(
                            start_line,
                            start_col,
                            2,
                            "commentaire de bloc '/*' non terminé",
                        ));
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, CompileError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments()?;
            let (line, col) = (self.line, self.col);
            let c = match self.peek() {
                None => {
                    tokens.push(Token {
                        kind: TokenKind::Eof,
                        line,
                        col,
                        len: 0,
                    });
                    break;
                }
                Some(c) => c,
            };

            if c.is_ascii_digit() {
                tokens.push(self.lex_number()?);
                continue;
            }
            if c == '_' || c.is_alphabetic() {
                tokens.push(self.lex_identifier());
                continue;
            }
            if c == '"' {
                tokens.push(self.lex_string()?);
                continue;
            }
            if c == '\'' {
                tokens.push(self.lex_char()?);
                continue;
            }

            tokens.push(self.lex_operator()?);
        }
        Ok(tokens)
    }

    fn lex_identifier(&mut self) -> Token {
        let (line, col) = (self.line, self.col);
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == '_' || c.is_alphanumeric() {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        let len = s.chars().count();
        Token {
            kind: TokenKind::Identifier(s),
            line,
            col,
            len,
        }
    }

    fn lex_number(&mut self) -> Result<Token, CompileError> {
        let (line, col) = (self.line, self.col);

        if self.peek() == Some('0') && matches!(self.peek_at(1), Some('x') | Some('X')) {
            self.advance(); 
            self.advance(); 
            let mut hex = String::new();
            while let Some(c) = self.peek() {
                if c.is_ascii_hexdigit() {
                    hex.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            let len = 2 + hex.chars().count();
            if hex.is_empty() {
                return Err(CompileError::new(
                    line,
                    col,
                    len,
                    "littéral hexadécimal invalide : aucun chiffre après '0x'",
                ));
            }
            let value = i128::from_str_radix(&hex, 16)
                .map_err(|_| CompileError::new(line, col, len, "littéral hexadécimal invalide"))?;
            return Ok(Token {
                kind: TokenKind::IntLiteral(value),
                line,
                col,
                len,
            });
        }

        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }

        if self.peek() == Some('.') && self.peek_at(1).map_or(false, |c| c.is_ascii_digit()) {
            s.push('.');
            self.advance();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    s.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
            let len = s.chars().count();
            let value: f64 = s
                .parse()
                .map_err(|_| CompileError::new(line, col, len, format!("littéral flottant invalide : '{}'", s)))?;
            return Ok(Token {
                kind: TokenKind::FloatLiteral(value),
                line,
                col,
                len,
            });
        }

        let len = s.chars().count();
        let value: i128 = s
            .parse()
            .map_err(|_| CompileError::new(line, col, len, format!("littéral entier invalide : '{}'", s)))?;
        Ok(Token {
            kind: TokenKind::IntLiteral(value),
            line,
            col,
            len,
        })
    }

    fn lex_string(&mut self) -> Result<Token, CompileError> {
        let (line, col) = (self.line, self.col);
        self.advance(); 
        let mut s = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(CompileError::new(line, col, 1, "chaîne de caractères non terminée"));
                }
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    self.advance();
                    let (esc_line, esc_col) = (self.line, self.col);
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('0') => s.push('\0'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some('\'') => s.push('\''),
                        Some(other) => {
                            return Err(CompileError::new(
                                esc_line,
                                esc_col,
                                1,
                                format!("séquence d'échappement inconnue : '\\{}'", other),
                            ));
                        }
                        None => {
                            return Err(CompileError::new(esc_line, esc_col, 1, "séquence d'échappement incomplète"));
                        }
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.advance();
                }
            }
        }
        let len = self.col.saturating_sub(col).max(1);
        Ok(Token {
            kind: TokenKind::StringLiteral(s),
            line,
            col,
            len,
        })
    }

    fn lex_char(&mut self) -> Result<Token, CompileError> {
        let (line, col) = (self.line, self.col);
        self.advance();
        let ch = match self.peek() {
            Some('\\') => {
                self.advance();
                match self.advance() {
                    Some('n') => '\n',
                    Some('t') => '\t',
                    Some('r') => '\r',
                    Some('0') => '\0',
                    Some('\\') => '\\',
                    Some('\'') => '\'',
                    Some('"') => '"',
                    Some(other) => {
                        return Err(CompileError::new(
                            line,
                            col,
                            1,
                            format!("séquence d'échappement inconnue : '\\{}'", other),
                        ));
                    }
                    None => return Err(CompileError::new(line, col, 1, "littéral caractère non terminé")),
                }
            }
            Some(c) => {
                self.advance();
                c
            }
            None => return Err(CompileError::new(line, col, 1, "littéral caractère non terminé")),
        };
        match self.peek() {
            Some('\'') => {
                self.advance();
            }
            _ => {
                return Err(CompileError::new(
                    line,
                    col,
                    1,
                    "littéral caractère non terminé : guillemet simple fermant attendu",
                ));
            }
        }
        let len = self.col.saturating_sub(col).max(1);
        Ok(Token {
            kind: TokenKind::CharLiteral(ch),
            line,
            col,
            len,
        })
    }

    fn lex_operator(&mut self) -> Result<Token, CompileError> {
        let (line, col) = (self.line, self.col);
        let c = self.advance().unwrap();

        macro_rules! two {
            ($next:expr, $two_kind:expr, $one_kind:expr) => {{
                if self.peek() == Some($next) {
                    self.advance();
                    Token { kind: $two_kind, line, col, len: 2 }
                } else {
                    Token { kind: $one_kind, line, col, len: 1 }
                }
            }};
        }

        let tok = match c {
            ':' => two!(':', TokenKind::ColonColon, TokenKind::Colon),
            ';' => Token { kind: TokenKind::Semicolon, line, col, len: 1 },
            ',' => Token { kind: TokenKind::Comma, line, col, len: 1 },
            '.' => {
                if self.peek() == Some('.') {
                    self.advance();
                    if self.peek() == Some('.') {
                        self.advance();
                        Token { kind: TokenKind::Ellipsis, line, col, len: 3 }
                    } else {
                        Token { kind: TokenKind::DotDot, line, col, len: 2 }
                    }
                } else {
                    Token { kind: TokenKind::Dot, line, col, len: 1 }
                }
            }
            '(' => Token { kind: TokenKind::LParen, line, col, len: 1 },
            ')' => Token { kind: TokenKind::RParen, line, col, len: 1 },
            '{' => Token { kind: TokenKind::LBrace, line, col, len: 1 },
            '}' => Token { kind: TokenKind::RBrace, line, col, len: 1 },
            '[' => Token { kind: TokenKind::LBracket, line, col, len: 1 },
            ']' => Token { kind: TokenKind::RBracket, line, col, len: 1 },
            '@' => Token { kind: TokenKind::At, line, col, len: 1 },
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    Token { kind: TokenKind::Arrow, line, col, len: 2 }
                } else if self.peek() == Some('=') {
                    self.advance();
                    Token { kind: TokenKind::MinusEq, line, col, len: 2 }
                } else {
                    Token { kind: TokenKind::Minus, line, col, len: 1 }
                }
            }
            '=' => {
                if self.peek() == Some('>') {
                    self.advance();
                    Token { kind: TokenKind::FatArrow, line, col, len: 2 }
                } else if self.peek() == Some('=') {
                    self.advance();
                    Token { kind: TokenKind::EqEq, line, col, len: 2 }
                } else {
                    Token { kind: TokenKind::Eq, line, col, len: 1 }
                }
            }
            '!' => two!('=', TokenKind::NotEq, TokenKind::Not),
            '<' => two!('=', TokenKind::LtEq, TokenKind::Lt),
            '>' => two!('=', TokenKind::GtEq, TokenKind::Gt),
            '+' => two!('=', TokenKind::PlusEq, TokenKind::Plus),
            '*' => two!('=', TokenKind::StarEq, TokenKind::Star),
            '/' => two!('=', TokenKind::SlashEq, TokenKind::Slash),
            '%' => Token { kind: TokenKind::Percent, line, col, len: 1 },
            '&' => two!('&', TokenKind::AmpAmp, TokenKind::Amp),
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    Token { kind: TokenKind::PipePipe, line, col, len: 2 }
                } else {
                    return Err(CompileError::new(line, col, 1, "caractère inattendu : '|' (vouliez-vous '||' ?)"));
                }
            }
            other => {
                return Err(CompileError::new(line, col, 1, format!("caractère inattendu : '{}'", other)));
            }
        };
        Ok(tok)
    }
}