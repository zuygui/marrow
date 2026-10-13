use crate::ast::*;
use crate::diagnostic::{Diagnostic, SourceFile, Span};
use crate::token::{Token, TokenKind};

const BUILTIN_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64", "bool", "rawptr",
];

/// Words that cannot be used as names.
pub const KEYWORDS: &[&str] = &[
    "fn", "struct", "var", "ret", "if", "else", "while", "for", "cast", "true", "false", "null", "va_start",
    "va_arg", "va_end", "break", "continue", "sizeof",
];

fn is_builtin_type_name(s: &str) -> bool {
    BUILTIN_TYPES.contains(&s)
}

fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains(&s)
}

pub struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    allow_struct_lit: bool,
    src: &'a SourceFile,
    file: usize,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, src: &'a SourceFile) -> Self {
        let file = tokens.first().map(|t| t.span.file).unwrap_or(0);
        Parser {
            tokens,
            pos: 0,
            allow_struct_lit: true,
            src,
            file,
        }
    }

    fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| self.tokens.last().unwrap())
    }

    fn peek_kind(&self) -> Option<&TokenKind> {
        Some(&self.current().kind)
    }

    fn peek_kind_at(&self, offset: usize) -> Option<&TokenKind> {
        self.tokens.get(self.pos + offset).map(|t| &t.kind)
    }

    fn cur_span(&self) -> Span {
        self.current().span
    }

    /// Span of the last consumed token.
    fn prev_span(&self) -> Span {
        if self.pos == 0 {
            self.cur_span()
        } else {
            self.tokens[self.pos - 1].span
        }
    }

    /// Span from the start of `start` to the end of the last consumed token.
    fn span_from(&self, start: Span) -> Span {
        Span::new(self.file, start.start, self.prev_span().end)
    }

    fn advance(&mut self) -> Token {
        let t = self.current().clone();
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn is_keyword_at(&self, offset: usize, word: &str) -> bool {
        matches!(self.peek_kind_at(offset), Some(TokenKind::Identifier(s)) if s == word)
    }

    fn at_eof(&self) -> bool {
        self.peek_kind() == Some(&TokenKind::Eof)
    }

    // -- errors ------------------------------------------------------------

    /// `expected X, found Y`, pointing at the unexpected token. For missing
    /// terminators the error points just after the previous token instead,
    /// which is where the terminator should have been written.
    fn error_expected(&self, what: &str, code: &'static str, terminator: bool) -> Diagnostic {
        let found = self.describe_current();
        let mut d = Diagnostic::error(format!("expected {}, found {}", what, found)).code(code);
        let cur = self.cur_span();
        let prev = self.prev_span();
        if self.at_eof() && self.pos > 0 {
            // Point at the end of the last real token, not at an empty last line.
            return d.primary(prev.end_point(), format!("expected {}", what));
        }
        let on_later_line = self.pos > 0 && self.src.line_index(prev.end) != self.src.line_index(cur.start);
        if terminator && on_later_line {
            d = d.primary(prev.end_point(), format!("expected {}", what));
            d = d.secondary(cur, "unexpected token");
        } else {
            d = d.primary(cur, format!("expected {}", what));
        }
        d
    }

    fn expect(&mut self, kind: TokenKind, desc: &str) -> Result<Token, Diagnostic> {
        if self.current().kind == kind {
            Ok(self.advance())
        } else {
            let terminator = matches!(
                kind,
                TokenKind::Semicolon | TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace | TokenKind::Comma
            );
            Err(self.error_expected(desc, "E0201", terminator))
        }
    }

    fn expect_identifier(&mut self, desc: &str) -> Result<(String, Span), Diagnostic> {
        match self.peek_kind().cloned() {
            Some(TokenKind::Identifier(name)) => {
                if is_keyword(&name) {
                    return Err(Diagnostic::error(format!("expected {}, found keyword `{}`", desc, name))
                        .code("E0202")
                        .primary(self.cur_span(), format!("expected {}, found reserved word", desc))
                        .help(format!("`{}` is a reserved word and cannot be used as a name; pick another name", name)));
                }
                let span = self.cur_span();
                self.advance();
                Ok((name, span))
            }
            _ => Err(self.error_expected(desc, "E0201", false)),
        }
    }

    fn describe_current(&self) -> String {
        Self::describe_kind(&self.current().kind)
    }

    fn describe_kind(kind: &TokenKind) -> String {
        match kind {
            TokenKind::Identifier(s) if is_keyword(s) => format!("keyword `{}`", s),
            TokenKind::Identifier(s) => format!("identifier `{}`", s),
            TokenKind::IntLiteral(v) => format!("integer literal `{}`", v),
            TokenKind::FloatLiteral(v) => format!("float literal `{}`", v),
            TokenKind::StringLiteral(s) => {
                let shown: String = s.chars().take(20).collect();
                if s.chars().count() > 20 {
                    format!("string literal \"{}...\"", shown.escape_default())
                } else {
                    format!("string literal \"{}\"", shown.escape_default())
                }
            }
            TokenKind::CharLiteral(c) => format!("character literal `'{}'`", c.escape_default()),
            TokenKind::Eof => "end of file".to_string(),
            other => format!("`{}`", Self::token_symbol(other)),
        }
    }

    fn token_symbol(kind: &TokenKind) -> &'static str {
        use TokenKind::*;
        match kind {
            ColonColon => "::",
            Colon => ":",
            Semicolon => ";",
            Comma => ",",
            Dot => ".",
            DotDot => "..",
            Ellipsis => "...",
            LParen => "(",
            RParen => ")",
            LBrace => "{",
            RBrace => "}",
            LBracket => "[",
            RBracket => "]",
            Arrow => "->",
            FatArrow => "=>",
            At => "@",
            Eq => "=",
            EqEq => "==",
            NotEq => "!=",
            Not => "!",
            Lt => "<",
            LtEq => "<=",
            Gt => ">",
            GtEq => ">=",
            Plus => "+",
            PlusEq => "+=",
            Minus => "-",
            MinusEq => "-=",
            Star => "*",
            StarEq => "*=",
            Slash => "/",
            SlashEq => "/=",
            Pipe => "|",
            Caret => "^",
            Tilde => "~",
            Shl => "<<",
            Shr => ">>",
            AmpEq => "&=",
            PipeEq => "|=",
            CaretEq => "^=",
            ShlEq => "<<=",
            ShrEq => ">>=",
            Percent => "%",
            AmpAmp => "&&",
            PipePipe => "||",
            Amp => "&",
            _ => "?",
        }
    }

    fn unclosed_delimiter(&self, open: Span, delim: &str) -> Diagnostic {
        Diagnostic::error("this file contains an unclosed delimiter")
            .code("E0201")
            .primary(open, format!("unclosed delimiter `{}`", delim))
            .secondary(self.prev_span(), "...the file ends here without closing it")
    }

    fn with_struct_lit_allowed<T>(
        &mut self,
        allowed: bool,
        f: impl FnOnce(&mut Self) -> Result<T, Diagnostic>,
    ) -> Result<T, Diagnostic> {
        let old = self.allow_struct_lit;
        self.allow_struct_lit = allowed;
        let result = f(self);
        self.allow_struct_lit = old;
        result
    }

    // -- items -------------------------------------------------------------

    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let mut items = Vec::new();
        while !self.at_eof() {
            items.push(self.parse_global_item()?);
        }
        Ok(Program { items })
    }

    fn parse_global_item(&mut self) -> Result<GlobalItem, Diagnostic> {
        let start = self.cur_span();
        let mut decorators = Vec::new();
        while self.peek_kind() == Some(&TokenKind::At) {
            decorators.push(self.parse_decorator()?);
        }
        let binding = self.parse_binding_decl()?;
        let span = self.span_from(start);
        Ok(GlobalItem { decorators, binding, span })
    }

    fn parse_decorator(&mut self) -> Result<Decorator, Diagnostic> {
        let start = self.cur_span();
        self.expect(TokenKind::At, "`@`")?;
        let (name, name_ident_span) = self.expect_identifier("decorator name")?;
        let name_span = start.to(name_ident_span);
        let args = if self.peek_kind() == Some(&TokenKind::LParen) {
            self.advance();
            let list = if self.peek_kind() != Some(&TokenKind::RParen) {
                Some(self.with_struct_lit_allowed(true, |p| p.parse_expression_list())?)
            } else {
                None
            };
            self.expect(TokenKind::RParen, "`)`")?;
            list
        } else {
            None
        };
        let span = self.span_from(start);
        Ok(Decorator { name, args, span, name_span })
    }

    fn parse_binding_decl(&mut self) -> Result<BindingDecl, Diagnostic> {
        let start = self.cur_span();
        if self.is_keyword_at(0, "fn") {
            return self.parse_fn_decl(start);
        }
        if self.is_keyword_at(0, "struct") {
            return self.parse_struct_decl(start);
        }
        if self.is_keyword_at(0, "var") {
            return self.parse_global_var_decl(start);
        }
        let mut d = self.error_expected("`fn`, `struct` or `var`", "E0203", false);
        if self.at_eof() && self.pos > 0 {
            d = d.help("decorators such as `@export` or `@import(...)` must be followed by a declaration");
        } else {
            d = d.note("only function, struct and global variable declarations are allowed at the top level");
        }
        Err(d)
    }

    fn parse_fn_decl(&mut self, start: Span) -> Result<BindingDecl, Diagnostic> {
        self.advance(); // 'fn'
        let (name, name_span) = self.expect_identifier("function name")?;
        let (params, variadic) = self.parse_parameter_list_parens()?;
        let ret_type = self.parse_optional_return_type()?;

        let value = if self.peek_kind() == Some(&TokenKind::FatArrow) {
            self.advance();
            let body = self.parse_expression()?;
            BindingValue::ExpressionFunction(ExpressionFunctionDef { params, ret_type, variadic, body: Box::new(body) })
        } else if self.peek_kind() == Some(&TokenKind::LBrace) {
            let lbrace = self.cur_span();
            let body = self.parse_block()?;
            let body_span = Some(self.span_from(lbrace));
            BindingValue::Function(FunctionDef { params, ret_type, variadic, body: Some(body), body_span })
        } else {
            BindingValue::Function(FunctionDef { params, ret_type, variadic, body: None, body_span: None })
        };

        let requires_semicolon = match &value {
            BindingValue::Function(f) => f.body.is_none(),
            _ => true,
        };

        if requires_semicolon {
            self.expect(TokenKind::Semicolon, "`;`")?;
        } else if self.peek_kind() == Some(&TokenKind::Semicolon) {
            self.advance();
        }

        let span = self.span_from(start);
        Ok(BindingDecl { ty: None, name, name_span, value, span })
    }

    fn parse_struct_decl(&mut self, start: Span) -> Result<BindingDecl, Diagnostic> {
        self.advance(); // 'struct'
        let (name, name_span) = self.expect_identifier("struct name")?;
        let sd = self.parse_struct_body()?;
        if self.peek_kind() == Some(&TokenKind::Semicolon) {
            self.advance();
        }
        let span = self.span_from(start);
        Ok(BindingDecl { ty: None, name, name_span, value: BindingValue::Struct(sd), span })
    }

    fn parse_global_var_decl(&mut self, start: Span) -> Result<BindingDecl, Diagnostic> {
        let (ty, name, name_span, value) = self.parse_var_decl_head(false)?;
        self.expect(TokenKind::Semicolon, "`;`")?;
        let span = self.span_from(start);
        Ok(BindingDecl { ty, name, name_span, value: BindingValue::Expr(value), span })
    }

    // `var` name (`:` type)? `=` expr
    fn parse_var_decl_head(&mut self, allow_zero: bool) -> Result<(Option<Type>, String, Span, Expression), Diagnostic> {
        self.advance(); // 'var'
        let (name, name_span) = self.expect_identifier("variable name")?;
        let ty = if self.peek_kind() == Some(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        if allow_zero && ty.is_some() && self.peek_kind() == Some(&TokenKind::Semicolon) {
            // `var x: T;` — zero-initialized.
            let span = self.cur_span();
            return Ok((ty, name, name_span, Expression { kind: ExprKind::Zeroed, span }));
        }
        if self.peek_kind() != Some(&TokenKind::Eq) {
            let mut d = self.error_expected("`=`", "E0201", false);
            d = d.note("variables must be initialized where they are declared: `var name: Type = value;`");
            return Err(d);
        }
        self.advance();
        let value = self.parse_expression()?;
        Ok((ty, name, name_span, value))
    }

    fn parse_parameter_list_parens(&mut self) -> Result<(Vec<Parameter>, bool), Diagnostic> {
        self.expect(TokenKind::LParen, "`(`")?;
        let mut params = Vec::new();
        let mut variadic = false;
        if self.peek_kind() != Some(&TokenKind::RParen) {
            loop {
                if self.peek_kind() == Some(&TokenKind::Ellipsis) {
                    self.advance();
                    variadic = true;
                    break;
                }
                let (name, name_span) = self.expect_identifier("parameter name")?;
                self.expect(TokenKind::Colon, "`:`")?;
                let ty = self.parse_type()?;
                params.push(Parameter { ty, name, name_span });
                if self.peek_kind() == Some(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "`)`")?;
        Ok((params, variadic))
    }

    fn parse_optional_return_type(&mut self) -> Result<Option<Type>, Diagnostic> {
        if self.peek_kind() == Some(&TokenKind::Arrow) {
            self.advance();
            Ok(Some(self.parse_type()?))
        } else {
            Ok(None)
        }
    }

    fn parse_struct_body(&mut self) -> Result<StructDef, Diagnostic> {
        let lbrace = self.cur_span();
        self.expect(TokenKind::LBrace, "`{`")?;
        let mut fields = Vec::new();
        while self.peek_kind() != Some(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(self.unclosed_delimiter(lbrace, "{"));
            }
            let (name, name_span) = self.expect_identifier("field name")?;
            self.expect(TokenKind::Colon, "`:`")?;
            let ty = self.parse_type()?;
            self.expect(TokenKind::Semicolon, "`;`")?;
            fields.push(StructField { ty, name, name_span });
        }
        self.expect(TokenKind::RBrace, "`}`")?;
        Ok(StructDef { fields })
    }

    // -- types -------------------------------------------------------------

    fn parse_type(&mut self) -> Result<Type, Diagnostic> {
        let start = self.cur_span();
        if self.peek_kind() == Some(&TokenKind::Star) {
            self.advance();
            let inner = self.parse_type()?;
            let span = self.span_from(start);
            return Ok(Type { kind: TypeKind::Pointer(Box::new(inner)), span });
        }

        let mut ty = self.parse_primary_type()?;

        loop {
            match self.peek_kind() {
                Some(TokenKind::LBracket) if self.peek_kind_at(1) == Some(&TokenKind::RBracket) => {
                    self.advance();
                    self.advance();
                    let span = self.span_from(start);
                    ty = Type { kind: TypeKind::Slice(Box::new(ty)), span };
                }
                _ => break,
            }
        }

        Ok(ty)
    }

    fn parse_primary_type(&mut self) -> Result<Type, Diagnostic> {
        let start = self.cur_span();
        match self.peek_kind().cloned() {
            Some(TokenKind::LBracket) => {
                self.advance();
                let size_expr = self.with_struct_lit_allowed(true, |p| p.parse_expression())?;
                self.expect(TokenKind::RBracket, "`]`")?;
                let inner = self.parse_type()?;
                let span = self.span_from(start);
                Ok(Type { kind: TypeKind::StaticArray(Box::new(size_expr), Box::new(inner)), span })
            }
            Some(TokenKind::Identifier(name)) if name == "fn" => {
                self.advance();
                self.expect(TokenKind::LParen, "`(`")?;
                let mut params = Vec::new();
                while self.peek_kind() != Some(&TokenKind::RParen) {
                    params.push(self.parse_type()?);
                    if self.peek_kind() == Some(&TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(TokenKind::RParen, "`)`")?;
                let ret = if self.peek_kind() == Some(&TokenKind::Arrow) {
                    self.advance();
                    Some(Box::new(self.parse_type()?))
                } else {
                    None
                };
                let span = self.span_from(start);
                Ok(Type { kind: TypeKind::Func(params, ret), span })
            }
            Some(TokenKind::Identifier(name)) if !is_keyword(&name) => {
                self.advance();
                let kind = if is_builtin_type_name(&name) { TypeKind::Builtin(name) } else { TypeKind::Custom(name) };
                Ok(Type { kind, span: start })
            }
            _ => Err(self.error_expected("a type", "E0204", false)),
        }
    }

    // -- statements --------------------------------------------------------

    fn parse_block(&mut self) -> Result<Block, Diagnostic> {
        let lbrace = self.cur_span();
        self.expect(TokenKind::LBrace, "`{`")?;
        let mut stmts = Vec::new();
        while self.peek_kind() != Some(&TokenKind::RBrace) {
            if self.at_eof() {
                return Err(self.unclosed_delimiter(lbrace, "{"));
            }
            stmts.push(self.parse_statement()?);
        }
        self.expect(TokenKind::RBrace, "`}`")?;
        Ok(stmts)
    }

    fn parse_statement(&mut self) -> Result<Statement, Diagnostic> {
        let start = self.cur_span();

        if self.peek_kind() == Some(&TokenKind::LBrace) {
            let b = self.parse_block()?;
            let span = self.span_from(start);
            return Ok(Statement { kind: StmtKind::Block(b), span });
        }
        if self.is_keyword_at(0, "if") {
            let s = self.parse_if_stmt()?;
            let span = self.span_from(start);
            return Ok(Statement { kind: StmtKind::If(s), span });
        }
        if self.is_keyword_at(0, "while") {
            let s = self.parse_while_stmt()?;
            let span = self.span_from(start);
            return Ok(Statement { kind: StmtKind::While(s), span });
        }
        if self.is_keyword_at(0, "for") {
            let s = self.parse_for_stmt()?;
            let span = self.span_from(start);
            return Ok(Statement { kind: StmtKind::For(s), span });
        }
        for (word, kind) in [("break", StmtKind::Break), ("continue", StmtKind::Continue)] {
            if self.is_keyword_at(0, word) {
                self.advance();
                self.expect(TokenKind::Semicolon, "`;`")?;
                let span = self.span_from(start);
                return Ok(Statement { kind, span });
            }
        }
        if self.is_keyword_at(0, "ret") {
            self.advance();
            let expr = if self.peek_kind() != Some(&TokenKind::Semicolon) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect(TokenKind::Semicolon, "`;`")?;
            let span = self.span_from(start);
            return Ok(Statement { kind: StmtKind::Return(expr), span });
        }

        if self.is_keyword_at(0, "var") {
            let decl = self.parse_local_var_decl()?;
            self.expect(TokenKind::Semicolon, "`;`")?;
            let span = self.span_from(start);
            return Ok(Statement { kind: StmtKind::LocalVarDecl(decl), span });
        }
        if self.is_keyword_at(0, "fn") || self.is_keyword_at(0, "struct") {
            let word = if self.is_keyword_at(0, "fn") { "function" } else { "struct" };
            return Err(Diagnostic::error(format!("{} declarations are only allowed at the top level", word))
                .code("E0203")
                .primary(self.cur_span(), format!("nested {} declaration", word))
                .help("move the declaration out of the function body"));
        }
        let expr = self.parse_expression()?;
        self.expect(TokenKind::Semicolon, "`;`")?;
        let span = self.span_from(start);
        Ok(Statement { kind: StmtKind::Expr(expr), span })
    }

    fn parse_if_stmt(&mut self) -> Result<IfStmt, Diagnostic> {
        self.advance();
        let cond = self.with_struct_lit_allowed(false, |p| p.parse_expression())?;
        let then_block = self.parse_block()?;
        let else_branch = if self.is_keyword_at(0, "else") {
            self.advance();
            if self.is_keyword_at(0, "if") {
                Some(ElseBranch::If(Box::new(self.parse_if_stmt()?)))
            } else {
                Some(ElseBranch::Block(self.parse_block()?))
            }
        } else {
            None
        };
        Ok(IfStmt { cond, then_block, else_branch })
    }

    fn parse_while_stmt(&mut self) -> Result<WhileStmt, Diagnostic> {
        self.advance();
        let cond = self.with_struct_lit_allowed(false, |p| p.parse_expression())?;
        let body = self.parse_block()?;
        Ok(WhileStmt { cond, body })
    }

    fn parse_for_stmt(&mut self) -> Result<ForStmt, Diagnostic> {
        self.advance();
        let init = if self.peek_kind() != Some(&TokenKind::Semicolon) {
            Some(Box::new(self.parse_local_var_decl_required()?))
        } else {
            None
        };
        self.expect(TokenKind::Semicolon, "`;`")?;
        let cond = if self.peek_kind() != Some(&TokenKind::Semicolon) {
            Some(self.with_struct_lit_allowed(false, |p| p.parse_expression())?)
        } else {
            None
        };
        self.expect(TokenKind::Semicolon, "`;`")?;
        let post = if self.peek_kind() != Some(&TokenKind::LBrace) {
            Some(self.with_struct_lit_allowed(false, |p| p.parse_expression())?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(ForStmt { init, cond, post, body })
    }

    fn parse_local_var_decl_required(&mut self) -> Result<LocalVarDecl, Diagnostic> {
        if self.is_keyword_at(0, "var") {
            self.parse_local_var_decl()
        } else {
            Err(self.error_expected("a `var` declaration in the `for` initializer", "E0201", false))
        }
    }

    fn parse_local_var_decl(&mut self) -> Result<LocalVarDecl, Diagnostic> {
        let (ty, name, name_span, value) = self.parse_var_decl_head(true)?;
        Ok(LocalVarDecl::Mutable { ty, name, name_span, value })
    }

    // -- expressions -------------------------------------------------------

    fn parse_expression_list(&mut self) -> Result<Vec<Expression>, Diagnostic> {
        let mut list = vec![self.parse_expression()?];
        while self.peek_kind() == Some(&TokenKind::Comma) {
            self.advance();
            list.push(self.parse_expression()?);
        }
        Ok(list)
    }

    fn parse_expression(&mut self) -> Result<Expression, Diagnostic> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expression, Diagnostic> {
        let left = self.parse_logic_or()?;
        let op = match self.peek_kind() {
            Some(TokenKind::Eq) => "=",
            Some(TokenKind::PlusEq) => "+=",
            Some(TokenKind::MinusEq) => "-=",
            Some(TokenKind::StarEq) => "*=",
            Some(TokenKind::SlashEq) => "/=",
            Some(TokenKind::AmpEq) => "&=",
            Some(TokenKind::PipeEq) => "|=",
            Some(TokenKind::CaretEq) => "^=",
            Some(TokenKind::ShlEq) => "<<=",
            Some(TokenKind::ShrEq) => ">>=",
            _ => return Ok(left),
        };
        let op_span = self.cur_span();
        self.advance();
        let value = self.parse_expression()?;
        let span = left.span.to(value.span);
        Ok(Expression {
            kind: ExprKind::Assign { op: op.into(), op_span, target: Box::new(left), value: Box::new(value) },
            span,
        })
    }

    fn binary(left: Expression, op: &str, op_span: Span, right: Expression) -> Expression {
        let span = left.span.to(right.span);
        Expression {
            kind: ExprKind::Binary { op: op.into(), op_span, left: Box::new(left), right: Box::new(right) },
            span,
        }
    }

    fn parse_logic_or(&mut self) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_logic_and()?;
        while self.peek_kind() == Some(&TokenKind::PipePipe) {
            let op_span = self.cur_span();
            self.advance();
            let right = self.parse_logic_and()?;
            left = Self::binary(left, "||", op_span, right);
        }
        Ok(left)
    }

    fn parse_logic_and(&mut self) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_equality()?;
        while self.peek_kind() == Some(&TokenKind::AmpAmp) {
            let op_span = self.cur_span();
            self.advance();
            let right = self.parse_equality()?;
            left = Self::binary(left, "&&", op_span, right);
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_relational()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::EqEq) => "==",
                Some(TokenKind::NotEq) => "!=",
                _ => break,
            };
            let op_span = self.cur_span();
            self.advance();
            let right = self.parse_relational()?;
            left = Self::binary(left, op, op_span, right);
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_bitor()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Lt) => "<",
                Some(TokenKind::LtEq) => "<=",
                Some(TokenKind::Gt) => ">",
                Some(TokenKind::GtEq) => ">=",
                _ => break,
            };
            let op_span = self.cur_span();
            self.advance();
            let right = self.parse_bitor()?;
            left = Self::binary(left, op, op_span, right);
        }
        Ok(left)
    }

    fn parse_bitor(&mut self) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_bitxor()?;
        while self.peek_kind() == Some(&TokenKind::Pipe) {
            let op_span = self.cur_span();
            self.advance();
            let right = self.parse_bitxor()?;
            left = Self::binary(left, "|", op_span, right);
        }
        Ok(left)
    }

    fn parse_bitxor(&mut self) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_bitand()?;
        while self.peek_kind() == Some(&TokenKind::Caret) {
            let op_span = self.cur_span();
            self.advance();
            let right = self.parse_bitand()?;
            left = Self::binary(left, "^", op_span, right);
        }
        Ok(left)
    }

    fn parse_bitand(&mut self) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_shift()?;
        while self.peek_kind() == Some(&TokenKind::Amp) {
            let op_span = self.cur_span();
            self.advance();
            let right = self.parse_shift()?;
            left = Self::binary(left, "&", op_span, right);
        }
        Ok(left)
    }

    fn parse_shift(&mut self) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Shl) => "<<",
                Some(TokenKind::Shr) => ">>",
                _ => break,
            };
            let op_span = self.cur_span();
            self.advance();
            let right = self.parse_additive()?;
            left = Self::binary(left, op, op_span, right);
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Plus) => "+",
                Some(TokenKind::Minus) => "-",
                _ => break,
            };
            let op_span = self.cur_span();
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Self::binary(left, op, op_span, right);
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, Diagnostic> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Star) => "*",
                Some(TokenKind::Slash) => "/",
                Some(TokenKind::Percent) => "%",
                _ => break,
            };
            let op_span = self.cur_span();
            self.advance();
            let right = self.parse_unary()?;
            left = Self::binary(left, op, op_span, right);
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, Diagnostic> {
        let start = self.cur_span();
        let op = match self.peek_kind() {
            Some(TokenKind::Minus) => Some("-"),
            Some(TokenKind::Not) => Some("!"),
            Some(TokenKind::Tilde) => Some("~"),
            Some(TokenKind::Amp) => Some("&"),
            Some(TokenKind::Star) => Some("*"),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let expr = self.parse_unary()?;
            let span = start.to(expr.span);
            Ok(Expression { kind: ExprKind::Unary { op: op.into(), expr: Box::new(expr) }, span })
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> Result<Expression, Diagnostic> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_kind() {
                Some(TokenKind::LBracket) => {
                    self.advance();
                    if self.peek_kind() == Some(&TokenKind::DotDot) {
                        self.advance();
                        let end = if self.peek_kind() != Some(&TokenKind::RBracket) {
                            Some(Box::new(self.with_struct_lit_allowed(true, |p| p.parse_expression())?))
                        } else {
                            None
                        };
                        self.expect(TokenKind::RBracket, "`]`")?;
                        let span = expr.span.to(self.prev_span());
                        expr = Expression { kind: ExprKind::Slice { base: Box::new(expr), start: None, end }, span };
                    } else {
                        let first = self.with_struct_lit_allowed(true, |p| p.parse_expression())?;
                        if self.peek_kind() == Some(&TokenKind::DotDot) {
                            self.advance();
                            let end = if self.peek_kind() != Some(&TokenKind::RBracket) {
                                Some(Box::new(self.with_struct_lit_allowed(true, |p| p.parse_expression())?))
                            } else {
                                None
                            };
                            self.expect(TokenKind::RBracket, "`]`")?;
                            let span = expr.span.to(self.prev_span());
                            expr = Expression {
                                kind: ExprKind::Slice { base: Box::new(expr), start: Some(Box::new(first)), end },
                                span,
                            };
                        } else {
                            self.expect(TokenKind::RBracket, "`]`")?;
                            let span = expr.span.to(self.prev_span());
                            expr = Expression { kind: ExprKind::Index { base: Box::new(expr), index: Box::new(first) }, span };
                        }
                    }
                }
                Some(TokenKind::Dot) => {
                    self.advance();
                    let (member, member_span) = self.expect_identifier("a member name")?;
                    let span = expr.span.to(member_span);
                    expr = Expression { kind: ExprKind::Member { base: Box::new(expr), member, member_span }, span };
                }
                Some(TokenKind::LParen) => {
                    self.advance();
                    let args = if self.peek_kind() != Some(&TokenKind::RParen) {
                        self.with_struct_lit_allowed(true, |p| p.parse_expression_list())?
                    } else {
                        Vec::new()
                    };
                    self.expect(TokenKind::RParen, "`)`")?;
                    let span = expr.span.to(self.prev_span());
                    expr = Expression { kind: ExprKind::Call { callee: Box::new(expr), args }, span };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, Diagnostic> {
        let start = self.cur_span();
        match self.peek_kind().cloned() {
            Some(TokenKind::Identifier(name)) => {
                if name == "cast" {
                    self.advance();
                    self.expect(TokenKind::LParen, "`(` after `cast`")?;
                    let ty = self.with_struct_lit_allowed(true, |p| p.parse_type())?;
                    self.expect(TokenKind::RParen, "`)`")?;
                    let expr = self.parse_expression()?;
                    let span = start.to(expr.span);
                    return Ok(Expression { kind: ExprKind::Cast { ty, expr: Box::new(expr) }, span });
                }
                if name == "sizeof" {
                    self.advance();
                    self.expect(TokenKind::LParen, "`(` after `sizeof`")?;
                    let ty = self.with_struct_lit_allowed(true, |p| p.parse_type())?;
                    self.expect(TokenKind::RParen, "`)`")?;
                    let span = self.span_from(start);
                    return Ok(Expression { kind: ExprKind::SizeOf(ty), span });
                }
                if name == "va_start" {
                    self.advance();
                    self.expect(TokenKind::LParen, "`(` after `va_start`")?;
                    self.expect(TokenKind::RParen, "`)` (`va_start` takes no arguments)")?;
                    let span = self.span_from(start);
                    return Ok(Expression { kind: ExprKind::VaStart, span });
                }
                if name == "va_arg" {
                    self.advance();
                    self.expect(TokenKind::LParen, "`(` after `va_arg`")?;
                    let list = self.with_struct_lit_allowed(true, |p| p.parse_expression())?;
                    self.expect(TokenKind::Comma, "`,` between the argument list and the type to read")?;
                    let ty = self.with_struct_lit_allowed(true, |p| p.parse_type())?;
                    self.expect(TokenKind::RParen, "`)`")?;
                    let span = self.span_from(start);
                    return Ok(Expression { kind: ExprKind::VaArg { list: Box::new(list), ty }, span });
                }
                if name == "va_end" {
                    self.advance();
                    self.expect(TokenKind::LParen, "`(` after `va_end`")?;
                    let list = self.with_struct_lit_allowed(true, |p| p.parse_expression())?;
                    self.expect(TokenKind::RParen, "`)`")?;
                    let span = self.span_from(start);
                    return Ok(Expression { kind: ExprKind::VaEnd(Box::new(list)), span });
                }
                if name == "true" || name == "false" {
                    self.advance();
                    return Ok(Expression { kind: ExprKind::BoolLiteral(name == "true"), span: start });
                }
                if name == "null" {
                    self.advance();
                    return Ok(Expression { kind: ExprKind::Null, span: start });
                }
                if is_keyword(&name) {
                    return Err(self.error_expected("an expression", "E0205", false));
                }
                self.advance();
                if self.allow_struct_lit && self.peek_kind() == Some(&TokenKind::LBrace) {
                    return self.parse_struct_initializer(name, start);
                }
                Ok(Expression { kind: ExprKind::Identifier(name), span: start })
            }
            Some(TokenKind::IntLiteral(v)) => {
                self.advance();
                Ok(Expression { kind: ExprKind::IntLiteral(v), span: start })
            }
            Some(TokenKind::FloatLiteral(v)) => {
                self.advance();
                Ok(Expression { kind: ExprKind::FloatLiteral(v), span: start })
            }
            Some(TokenKind::StringLiteral(s)) => {
                self.advance();
                Ok(Expression { kind: ExprKind::StringLiteral(s), span: start })
            }
            Some(TokenKind::CharLiteral(c)) => {
                self.advance();
                Ok(Expression { kind: ExprKind::CharLiteral(c), span: start })
            }
            Some(TokenKind::LParen) => {
                self.advance();
                let mut expr = self.with_struct_lit_allowed(true, |p| p.parse_expression())?;
                self.expect(TokenKind::RParen, "`)`")?;
                // Keep the span of the parenthesized expression so that
                // diagnostics underline the parentheses too.
                expr.span = self.span_from(start);
                Ok(expr)
            }
            _ => Err(self.error_expected("an expression", "E0205", false)),
        }
    }

    fn parse_struct_initializer(&mut self, name: String, name_span: Span) -> Result<Expression, Diagnostic> {
        let lbrace = self.cur_span();
        self.expect(TokenKind::LBrace, "`{`")?;
        let old = self.allow_struct_lit;
        self.allow_struct_lit = true;

        let mut fields: Vec<FieldInit> = Vec::new();
        let mut err: Option<Diagnostic> = None;
        if self.peek_kind() != Some(&TokenKind::RBrace) {
            loop {
                match self.parse_field_init() {
                    Ok(f) => fields.push(f),
                    Err(e) => {
                        err = Some(e);
                        break;
                    }
                }
                if self.peek_kind() == Some(&TokenKind::Comma) {
                    self.advance();
                    if self.peek_kind() == Some(&TokenKind::RBrace) {
                        break;
                    }
                } else {
                    break;
                }
            }
        }

        self.allow_struct_lit = old;
        if let Some(e) = err {
            return Err(e);
        }
        if self.at_eof() {
            return Err(self.unclosed_delimiter(lbrace, "{"));
        }
        self.expect(TokenKind::RBrace, "`}`")?;
        let span = self.span_from(name_span);
        Ok(Expression { kind: ExprKind::StructInit { name, name_span, fields }, span })
    }

    fn parse_field_init(&mut self) -> Result<FieldInit, Diagnostic> {
        let (name, name_span) = self.expect_identifier("a field name")?;
        self.expect(TokenKind::Colon, "`:`")?;
        let value = self.parse_expression()?;
        Ok(FieldInit { name, name_span, value })
    }
}
