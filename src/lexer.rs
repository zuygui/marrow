use crate::diagnostic::{Diagnostic, Span};
use crate::token::{Token, TokenKind};

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    file: usize,
}

const VALID_ESCAPES: &str = "valid escapes are `\\n`, `\\t`, `\\r`, `\\0`, `\\\\`, `\\\"` and `\\'`";

impl Lexer {
    pub fn new(source: &str, file: usize) -> Self {
        Lexer {
            chars: source.chars().collect(),
            pos: 0,
            file,
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
        Some(c)
    }

    fn span(&self, start: usize) -> Span {
        Span::new(self.file, start, self.pos)
    }

    fn token(&self, kind: TokenKind, start: usize) -> Token {
        Token { kind, span: self.span(start) }
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), Diagnostic> {
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
                    let start = self.pos;
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
                        return Err(Diagnostic::error("unterminated block comment")
                            .code("E0101")
                            .primary(Span::new(self.file, start, start + 2), "this block comment is never closed")
                            .help("add `*/` to close the comment"));
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments()?;
            let c = match self.peek() {
                None => {
                    let end = self.chars.len();
                    tokens.push(Token { kind: TokenKind::Eof, span: Span::new(self.file, end, end) });
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
        let start = self.pos;
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == '_' || c.is_alphanumeric() {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        self.token(TokenKind::Identifier(s), start)
    }

    /// Reports `10abc` / `5u8` style literals, which would otherwise be
    /// silently split into two tokens.
    fn check_number_suffix(&mut self, start: usize) -> Result<(), Diagnostic> {
        match self.peek() {
            Some(c) if c == '_' || c.is_alphabetic() => {
                let sfx_start = self.pos;
                let mut sfx = String::new();
                while let Some(c) = self.peek() {
                    if c == '_' || c.is_alphanumeric() {
                        sfx.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                let literal: String = self.chars[start..sfx_start].iter().collect();
                let mut d = Diagnostic::error(format!("invalid suffix `{}` for number literal", sfx))
                    .code("E0105")
                    .primary(Span::new(self.file, start, self.pos), "invalid suffix")
                    .help("number literals have no suffix: write the literal alone and give the variable a type instead");
                if matches!(sfx.as_str(), "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64") {
                    d = d.suggest(
                        format!("to give the literal a type, use an explicit cast"),
                        Span::new(self.file, start, self.pos),
                        format!("cast({}) {}", sfx, literal),
                    );
                }
                Err(d)
            }
            _ => Ok(()),
        }
    }

    fn lex_number(&mut self) -> Result<Token, Diagnostic> {
        let start = self.pos;

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
            if hex.is_empty() {
                return Err(Diagnostic::error("no valid digits found for number")
                    .code("E0105")
                    .primary(self.span(start), "expected at least one hexadecimal digit after `0x`"));
            }
            self.check_number_suffix(start)?;
            let value = i128::from_str_radix(&hex, 16).map_err(|_| {
                Diagnostic::error("hexadecimal literal is too large")
                    .code("E0105")
                    .primary(self.span(start), "this literal does not fit in 128 bits")
            })?;
            return Ok(self.token(TokenKind::IntLiteral(value), start));
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
            self.check_number_suffix(start)?;
            let value: f64 = s.parse().map_err(|_| {
                Diagnostic::error(format!("invalid float literal `{}`", s))
                    .code("E0105")
                    .primary(self.span(start), "not a valid floating point number")
            })?;
            return Ok(self.token(TokenKind::FloatLiteral(value), start));
        }

        self.check_number_suffix(start)?;
        let value: i128 = s.parse().map_err(|_| {
            Diagnostic::error("integer literal is too large")
                .code("E0105")
                .primary(self.span(start), "this literal does not fit in 128 bits")
        })?;
        Ok(self.token(TokenKind::IntLiteral(value), start))
    }

    fn lex_string(&mut self) -> Result<Token, Diagnostic> {
        let start = self.pos;
        self.advance();
        let mut s = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(Diagnostic::error("unterminated string literal")
                        .code("E0102")
                        .primary(Span::new(self.file, start, self.pos), "this string literal is never closed")
                        .help("add a closing `\"`"));
                }
                Some('"') => {
                    self.advance();
                    break;
                }
                Some('\\') => {
                    let esc_start = self.pos;
                    self.advance();
                    match self.advance() {
                        Some('n') => s.push('\n'),
                        Some('t') => s.push('\t'),
                        Some('r') => s.push('\r'),
                        Some('0') => s.push('\0'),
                        Some('\\') => s.push('\\'),
                        Some('"') => s.push('"'),
                        Some('\'') => s.push('\''),
                        Some(other) => {
                            return Err(Diagnostic::error(format!("unknown escape sequence `\\{}`", other))
                                .code("E0104")
                                .primary(self.span(esc_start), "unknown escape sequence")
                                .note(VALID_ESCAPES));
                        }
                        None => {
                            return Err(Diagnostic::error("unterminated string literal")
                                .code("E0102")
                                .primary(Span::new(self.file, start, self.pos), "this string literal is never closed")
                                .help("add a closing `\"`"));
                        }
                    }
                }
                Some(c) => {
                    s.push(c);
                    self.advance();
                }
            }
        }
        Ok(self.token(TokenKind::StringLiteral(s), start))
    }

    fn lex_char(&mut self) -> Result<Token, Diagnostic> {
        let start = self.pos;
        self.advance();
        let ch = match self.peek() {
            Some('\'') => {
                self.advance();
                return Err(Diagnostic::error("empty character literal")
                    .code("E0103")
                    .primary(self.span(start), "a character literal must contain exactly one character"));
            }
            Some('\\') => {
                let esc_start = self.pos;
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
                        return Err(Diagnostic::error(format!("unknown escape sequence `\\{}`", other))
                            .code("E0104")
                            .primary(self.span(esc_start), "unknown escape sequence")
                            .note(VALID_ESCAPES));
                    }
                    None => {
                        return Err(Diagnostic::error("unterminated character literal")
                            .code("E0103")
                            .primary(Span::new(self.file, start, self.pos), "this character literal is never closed"));
                    }
                }
            }
            Some('\n') | None => {
                return Err(Diagnostic::error("unterminated character literal")
                    .code("E0103")
                    .primary(Span::new(self.file, start, self.pos), "this character literal is never closed"));
            }
            Some(c) => {
                self.advance();
                c
            }
        };
        match self.peek() {
            Some('\'') => {
                self.advance();
            }
            _ => {
                // Find where the literal would end, to underline all of it.
                let mut end = self.pos;
                while let Some(c) = self.chars.get(end) {
                    if *c == '\'' || *c == '\n' {
                        break;
                    }
                    end += 1;
                }
                let closed = self.chars.get(end) == Some(&'\'');
                if closed {
                    let span = Span::new(self.file, start, end + 1);
                    return Err(Diagnostic::error("character literal may only contain one character")
                        .code("E0103")
                        .primary(span, "too many characters")
                        .help("if you meant to write a string, use double quotes: `\"...\"`"));
                }
                return Err(Diagnostic::error("unterminated character literal")
                    .code("E0103")
                    .primary(self.span(start), "this character literal is never closed")
                    .help("add a closing `'`"));
            }
        }
        Ok(self.token(TokenKind::CharLiteral(ch), start))
    }

    fn lex_operator(&mut self) -> Result<Token, Diagnostic> {
        let start = self.pos;
        let c = self.advance().unwrap();

        macro_rules! two {
            ($next:expr, $two_kind:expr, $one_kind:expr) => {{
                if self.peek() == Some($next) {
                    self.advance();
                    self.token($two_kind, start)
                } else {
                    self.token($one_kind, start)
                }
            }};
        }

        let tok = match c {
            ':' => two!(':', TokenKind::ColonColon, TokenKind::Colon),
            ';' => self.token(TokenKind::Semicolon, start),
            ',' => self.token(TokenKind::Comma, start),
            '.' => {
                if self.peek() == Some('.') {
                    self.advance();
                    if self.peek() == Some('.') {
                        self.advance();
                        self.token(TokenKind::Ellipsis, start)
                    } else {
                        self.token(TokenKind::DotDot, start)
                    }
                } else {
                    self.token(TokenKind::Dot, start)
                }
            }
            '(' => self.token(TokenKind::LParen, start),
            ')' => self.token(TokenKind::RParen, start),
            '{' => self.token(TokenKind::LBrace, start),
            '}' => self.token(TokenKind::RBrace, start),
            '[' => self.token(TokenKind::LBracket, start),
            ']' => self.token(TokenKind::RBracket, start),
            '@' => self.token(TokenKind::At, start),
            '-' => {
                if self.peek() == Some('>') {
                    self.advance();
                    self.token(TokenKind::Arrow, start)
                } else if self.peek() == Some('=') {
                    self.advance();
                    self.token(TokenKind::MinusEq, start)
                } else {
                    self.token(TokenKind::Minus, start)
                }
            }
            '=' => {
                if self.peek() == Some('>') {
                    self.advance();
                    self.token(TokenKind::FatArrow, start)
                } else if self.peek() == Some('=') {
                    self.advance();
                    self.token(TokenKind::EqEq, start)
                } else {
                    self.token(TokenKind::Eq, start)
                }
            }
            '!' => two!('=', TokenKind::NotEq, TokenKind::Not),
            '<' => two!('=', TokenKind::LtEq, TokenKind::Lt),
            '>' => two!('=', TokenKind::GtEq, TokenKind::Gt),
            '+' => two!('=', TokenKind::PlusEq, TokenKind::Plus),
            '*' => two!('=', TokenKind::StarEq, TokenKind::Star),
            '/' => two!('=', TokenKind::SlashEq, TokenKind::Slash),
            '%' => self.token(TokenKind::Percent, start),
            '&' => two!('&', TokenKind::AmpAmp, TokenKind::Amp),
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    self.token(TokenKind::PipePipe, start)
                } else {
                    return Err(Diagnostic::error("unexpected character `|`")
                        .code("E0106")
                        .primary(self.span(start), "not a valid operator on its own")
                        .help("did you mean the logical or operator `||`?")
                        .note("bitwise operators are not supported yet"));
                }
            }
            other => {
                return Err(Diagnostic::error(format!("unknown start of token: `{}`", other))
                    .code("E0106")
                    .primary(self.span(start), "this character is not valid in Marrow source code"));
            }
        };
        Ok(tok)
    }
}
