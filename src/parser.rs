use crate::ast::*;
use crate::error::CompileError;
use crate::token::{Token, TokenKind};

const BUILTIN_TYPES: &[&str] = &[
    "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64", "bool", "rawptr",
];

fn is_builtin_type_name(s: &str) -> bool {
    BUILTIN_TYPES.contains(&s)
}

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    allow_struct_lit: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            allow_struct_lit: true,
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

    fn current_pos(&self) -> (usize, usize) {
        let t = self.current();
        (t.line, t.col)
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

    fn expect(&mut self, kind: TokenKind, desc: &str) -> Result<Token, CompileError> {
        if self.current().kind == kind {
            Ok(self.advance())
        } else {
            Err(self.error_here(format!("{} attendu, trouvé {}", desc, self.describe_current())))
        }
    }

    fn expect_identifier(&mut self, desc: &str) -> Result<String, CompileError> {
        match self.peek_kind().cloned() {
            Some(TokenKind::Identifier(name)) => {
                self.advance();
                Ok(name)
            }
            _ => Err(self.error_here(format!("{} attendu, trouvé {}", desc, self.describe_current()))),
        }
    }

    fn error_here(&self, message: impl Into<String>) -> CompileError {
        let t = self.current();
        CompileError::new(t.line, t.col, t.len.max(1), message)
    }

    fn describe_current(&self) -> String {
        Self::describe_kind(&self.current().kind)
    }

    fn describe_kind(kind: &TokenKind) -> String {
        match kind {
            TokenKind::Identifier(s) => format!("l'identifiant '{}'", s),
            TokenKind::IntLiteral(v) => format!("l'entier '{}'", v),
            TokenKind::FloatLiteral(v) => format!("le flottant '{}'", v),
            TokenKind::StringLiteral(s) => format!("la chaîne \"{}\"", s),
            TokenKind::CharLiteral(c) => format!("le caractère '{}'", c),
            TokenKind::Eof => "la fin du fichier".to_string(),
            other => format!("'{}'", Self::token_symbol(other)),
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
            Percent => "%",
            AmpAmp => "&&",
            PipePipe => "||",
            Amp => "&",
            _ => "?",
        }
    }

    fn with_struct_lit_allowed<T>(
        &mut self,
        allowed: bool,
        f: impl FnOnce(&mut Self) -> Result<T, CompileError>,
    ) -> Result<T, CompileError> {
        let old = self.allow_struct_lit;
        self.allow_struct_lit = allowed;
        let result = f(self);
        self.allow_struct_lit = old;
        result
    }

    pub fn parse_program(&mut self) -> Result<Program, CompileError> {
        let mut items = Vec::new();
        while self.peek_kind() != Some(&TokenKind::Eof) {
            items.push(self.parse_global_item()?);
        }
        Ok(Program { items })
    }

    fn parse_global_item(&mut self) -> Result<GlobalItem, CompileError> {
        let (line, col) = self.current_pos();
        let mut decorators = Vec::new();
        while self.peek_kind() == Some(&TokenKind::At) {
            decorators.push(self.parse_decorator()?);
        }
        let binding = self.parse_binding_decl()?;
        Ok(GlobalItem { decorators, binding, line, col })
    }

    fn parse_decorator(&mut self) -> Result<Decorator, CompileError> {
        let (line, col) = self.current_pos();
        self.expect(TokenKind::At, "'@'")?;
        let name = self.expect_identifier("nom de décorateur")?;
        let args = if self.peek_kind() == Some(&TokenKind::LParen) {
            self.advance();
            let list = if self.peek_kind() != Some(&TokenKind::RParen) {
                Some(self.with_struct_lit_allowed(true, |p| p.parse_expression_list())?)
            } else {
                None
            };
            self.expect(TokenKind::RParen, "')'")?;
            list
        } else {
            None
        };
        Ok(Decorator { name, args, line, col })
    }

    fn parse_binding_decl(&mut self) -> Result<BindingDecl, CompileError> {
        let (line, col) = self.current_pos();
        if self.is_keyword_at(0, "fn") {
            return self.parse_fn_decl(line, col);
        }
        if self.is_keyword_at(0, "struct") {
            return self.parse_struct_decl(line, col);
        }
        if self.is_keyword_at(0, "var") {
            return self.parse_global_var_decl(line, col);
        }
        Err(self.error_here(format!(
            "déclaration attendue ('fn', 'struct' ou 'var'), trouvé {}",
            self.describe_current()
        )))
    }

    fn parse_fn_decl(&mut self, line: usize, col: usize) -> Result<BindingDecl, CompileError> {
        self.advance(); // 'fn'
        let name = self.expect_identifier("nom de fonction")?;
        let (params, variadic) = self.parse_parameter_list_parens()?;
        let ret_type = self.parse_optional_return_type()?;

        let value = if self.peek_kind() == Some(&TokenKind::FatArrow) {
            self.advance();
            let body = self.parse_expression()?;
            BindingValue::ExpressionFunction(ExpressionFunctionDef { params, ret_type, variadic, body: Box::new(body) })
        } else if self.peek_kind() == Some(&TokenKind::LBrace) {
            let body = self.parse_block()?;
            BindingValue::Function(FunctionDef { params, ret_type, variadic, body: Some(body) })
        } else {
            BindingValue::Function(FunctionDef { params, ret_type, variadic, body: None })
        };

        let requires_semicolon = match &value {
            BindingValue::Function(f) => f.body.is_none(),
            _ => true,
        };

        if requires_semicolon {
            self.expect(TokenKind::Semicolon, "';'")?;
        } else if self.peek_kind() == Some(&TokenKind::Semicolon) {
            self.advance();
        }

        Ok(BindingDecl { ty: None, name, value, line, col })
    }

    fn parse_struct_decl(&mut self, line: usize, col: usize) -> Result<BindingDecl, CompileError> {
        self.advance(); // 'struct'
        let name = self.expect_identifier("nom de structure")?;
        let sd = self.parse_struct_body()?;
        if self.peek_kind() == Some(&TokenKind::Semicolon) {
            self.advance();
        }
        Ok(BindingDecl { ty: None, name, value: BindingValue::Struct(sd), line, col })
    }

    fn parse_global_var_decl(&mut self, line: usize, col: usize) -> Result<BindingDecl, CompileError> {
        let (ty, name, value) = self.parse_var_decl_head()?;
        self.expect(TokenKind::Semicolon, "';'")?;
        Ok(BindingDecl { ty, name, value: BindingValue::Expr(value), line, col })
    }

    // Analyse la tête commune d'une déclaration 'var' : 'var' nom (':' type)? '=' expr
    fn parse_var_decl_head(&mut self) -> Result<(Option<Type>, String, Expression), CompileError> {
        self.advance(); // 'var'
        let name = self.expect_identifier("nom de variable")?;
        let ty = if self.peek_kind() == Some(&TokenKind::Colon) {
            self.advance();
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(TokenKind::Eq, "'='")?;
        let value = self.parse_expression()?;
        Ok((ty, name, value))
    }

    fn parse_parameter_list_parens(&mut self) -> Result<(Vec<Parameter>, bool), CompileError> {
        self.expect(TokenKind::LParen, "'('")?;
        let mut params = Vec::new();
        let mut variadic = false;
        if self.peek_kind() != Some(&TokenKind::RParen) {
            loop {
                if self.peek_kind() == Some(&TokenKind::Ellipsis) {
                    self.advance();
                    variadic = true;
                    break;
                }
                let name = self.expect_identifier("nom de paramètre")?;
                self.expect(TokenKind::Colon, "':'")?;
                let ty = self.parse_type()?;
                params.push(Parameter { ty, name });
                if self.peek_kind() == Some(&TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::RParen, "')'")?;
        Ok((params, variadic))
    }

    fn parse_optional_return_type(&mut self) -> Result<Option<Type>, CompileError> {
        if self.peek_kind() == Some(&TokenKind::Arrow) {
            self.advance();
            Ok(Some(self.parse_type()?))
        } else {
            Ok(None)
        }
    }

    fn parse_struct_body(&mut self) -> Result<StructDef, CompileError> {
        self.expect(TokenKind::LBrace, "'{'")?;
        let mut fields = Vec::new();
        while self.peek_kind() != Some(&TokenKind::RBrace) {
            let name = self.expect_identifier("nom de champ")?;
            self.expect(TokenKind::Colon, "':'")?;
            let ty = self.parse_type()?;
            self.expect(TokenKind::Semicolon, "';'")?;
            fields.push(StructField { ty, name });
        }
        self.expect(TokenKind::RBrace, "'}'")?;
        Ok(StructDef { fields })
    }

    fn parse_type(&mut self) -> Result<Type, CompileError> {
    if match self.peek_kind() {
        Some(TokenKind::Star) => true,
        _ => false,
    } {
        self.advance();
        let inner = self.parse_type()?; 
        return Ok(Type::Pointer(Box::new(inner)));
    }

    let mut ty = self.parse_primary_type()?;

    loop {
        match self.peek_kind() {
            Some(TokenKind::LBracket) if self.peek_kind_at(1) == Some(&TokenKind::RBracket) => {
                self.advance();
                self.advance();
                ty = Type::Slice(Box::new(ty));
            }
            _ => break,
        }
    }

    Ok(ty)
}

    fn parse_primary_type(&mut self) -> Result<Type, CompileError> {
        match self.peek_kind().cloned() {
            Some(TokenKind::LBracket) => {
                self.advance();
                let size_expr = self.with_struct_lit_allowed(true, |p| p.parse_expression())?;
                self.expect(TokenKind::RBracket, "']'")?;
                let inner = self.parse_type()?;
                Ok(Type::StaticArray(Box::new(size_expr), Box::new(inner)))
            }
            Some(TokenKind::Identifier(name)) => {
                self.advance();
                if is_builtin_type_name(&name) {
                    Ok(Type::Builtin(name))
                } else {
                    Ok(Type::Custom(name))
                }
            }
            _ => Err(self.error_here(format!("type attendu, trouvé {}", self.describe_current()))),
        }
    }

    fn parse_block(&mut self) -> Result<Block, CompileError> {
        self.expect(TokenKind::LBrace, "'{'")?;
        let mut stmts = Vec::new();
        while self.peek_kind() != Some(&TokenKind::RBrace) && self.peek_kind() != Some(&TokenKind::Eof) {
            stmts.push(self.parse_statement()?);
        }
        self.expect(TokenKind::RBrace, "'}'")?;
        Ok(stmts)
    }

    fn parse_statement(&mut self) -> Result<Statement, CompileError> {
        let (line, col) = self.current_pos();

        if self.peek_kind() == Some(&TokenKind::LBrace) {
            let b = self.parse_block()?;
            return Ok(Statement { kind: StmtKind::Block(b), line, col });
        }
        if self.is_keyword_at(0, "if") {
            let s = self.parse_if_stmt()?;
            return Ok(Statement { kind: StmtKind::If(s), line, col });
        }
        if self.is_keyword_at(0, "while") {
            let s = self.parse_while_stmt()?;
            return Ok(Statement { kind: StmtKind::While(s), line, col });
        }
        if self.is_keyword_at(0, "for") {
            let s = self.parse_for_stmt()?;
            return Ok(Statement { kind: StmtKind::For(s), line, col });
        }
        if self.is_keyword_at(0, "ret") {
            self.advance();
            let expr = if self.peek_kind() != Some(&TokenKind::Semicolon) {
                Some(self.parse_expression()?)
            } else {
                None
            };
            self.expect(TokenKind::Semicolon, "';'")?;
            return Ok(Statement { kind: StmtKind::Return(expr), line, col });
        }

        if self.is_keyword_at(0, "var") {
            let decl = self.parse_local_var_decl()?;
            self.expect(TokenKind::Semicolon, "';'")?;
            return Ok(Statement { kind: StmtKind::LocalVarDecl(decl), line, col });
        }
        let expr = self.parse_expression()?;
        self.expect(TokenKind::Semicolon, "';'")?;
        Ok(Statement { kind: StmtKind::Expr(expr), line, col })
    }

    fn parse_if_stmt(&mut self) -> Result<IfStmt, CompileError> {
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

    fn parse_while_stmt(&mut self) -> Result<WhileStmt, CompileError> {
        self.advance();
        let cond = self.with_struct_lit_allowed(false, |p| p.parse_expression())?;
        let body = self.parse_block()?;
        Ok(WhileStmt { cond, body })
    }

    fn parse_for_stmt(&mut self) -> Result<ForStmt, CompileError> {
        self.advance(); 
        let init = if self.peek_kind() != Some(&TokenKind::Semicolon) {
            Some(Box::new(self.parse_local_var_decl_required()?))
        } else {
            None
        };
        self.expect(TokenKind::Semicolon, "';'")?;
        let cond = if self.peek_kind() != Some(&TokenKind::Semicolon) {
            Some(self.with_struct_lit_allowed(false, |p| p.parse_expression())?)
        } else {
            None
        };
        self.expect(TokenKind::Semicolon, "';'")?;
        let post = if self.peek_kind() != Some(&TokenKind::LBrace) {
            Some(self.with_struct_lit_allowed(false, |p| p.parse_expression())?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(ForStmt { init, cond, post, body })
    }

    fn parse_local_var_decl_required(&mut self) -> Result<LocalVarDecl, CompileError> {
        if self.is_keyword_at(0, "var") {
            self.parse_local_var_decl()
        } else {
            Err(self.error_here("déclaration de variable ('var ...') attendue dans l'en-tête du 'for'"))
        }
    }

    fn parse_local_var_decl(&mut self) -> Result<LocalVarDecl, CompileError> {
        let (ty, name, value) = self.parse_var_decl_head()?;
        Ok(LocalVarDecl::Mutable { ty, name, value })
    }

    fn parse_expression_list(&mut self) -> Result<Vec<Expression>, CompileError> {
        let mut list = vec![self.parse_expression()?];
        while self.peek_kind() == Some(&TokenKind::Comma) {
            self.advance();
            list.push(self.parse_expression()?);
        }
        Ok(list)
    }

    fn parse_expression(&mut self) -> Result<Expression, CompileError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expression, CompileError> {
        let left = self.parse_logic_or()?;
        let op = match self.peek_kind() {
            Some(TokenKind::Eq) => "=",
            Some(TokenKind::PlusEq) => "+=",
            Some(TokenKind::MinusEq) => "-=",
            Some(TokenKind::StarEq) => "*=",
            Some(TokenKind::SlashEq) => "/=",
            _ => return Ok(left),
        };
        let (line, col) = self.current_pos();
        self.advance();
        let value = self.parse_expression()?;
        Ok(Expression {
            kind: ExprKind::Assign { op: op.into(), target: Box::new(left), value: Box::new(value) },
            line,
            col,
        })
    }

    fn parse_logic_or(&mut self) -> Result<Expression, CompileError> {
        let mut left = self.parse_logic_and()?;
        while self.peek_kind() == Some(&TokenKind::PipePipe) {
            let (line, col) = self.current_pos();
            self.advance();
            let right = self.parse_logic_and()?;
            left = Expression { kind: ExprKind::Binary { op: "||".into(), left: Box::new(left), right: Box::new(right) }, line, col };
        }
        Ok(left)
    }

    fn parse_logic_and(&mut self) -> Result<Expression, CompileError> {
        let mut left = self.parse_equality()?;
        while self.peek_kind() == Some(&TokenKind::AmpAmp) {
            let (line, col) = self.current_pos();
            self.advance();
            let right = self.parse_equality()?;
            left = Expression { kind: ExprKind::Binary { op: "&&".into(), left: Box::new(left), right: Box::new(right) }, line, col };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expression, CompileError> {
        let mut left = self.parse_relational()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::EqEq) => "==",
                Some(TokenKind::NotEq) => "!=",
                _ => break,
            };
            let (line, col) = self.current_pos();
            self.advance();
            let right = self.parse_relational()?;
            left = Expression { kind: ExprKind::Binary { op: op.into(), left: Box::new(left), right: Box::new(right) }, line, col };
        }
        Ok(left)
    }

    fn parse_relational(&mut self) -> Result<Expression, CompileError> {
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Lt) => "<",
                Some(TokenKind::LtEq) => "<=",
                Some(TokenKind::Gt) => ">",
                Some(TokenKind::GtEq) => ">=",
                _ => break,
            };
            let (line, col) = self.current_pos();
            self.advance();
            let right = self.parse_additive()?;
            left = Expression { kind: ExprKind::Binary { op: op.into(), left: Box::new(left), right: Box::new(right) }, line, col };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expression, CompileError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Plus) => "+",
                Some(TokenKind::Minus) => "-",
                _ => break,
            };
            let (line, col) = self.current_pos();
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expression { kind: ExprKind::Binary { op: op.into(), left: Box::new(left), right: Box::new(right) }, line, col };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expression, CompileError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.peek_kind() {
                Some(TokenKind::Star) => "*",
                Some(TokenKind::Slash) => "/",
                Some(TokenKind::Percent) => "%",
                _ => break,
            };
            let (line, col) = self.current_pos();
            self.advance();
            let right = self.parse_unary()?;
            left = Expression { kind: ExprKind::Binary { op: op.into(), left: Box::new(left), right: Box::new(right) }, line, col };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expression, CompileError> {
        let (line, col) = self.current_pos();
        let op = match self.peek_kind() {
            Some(TokenKind::Minus) => Some("-"),
            Some(TokenKind::Not) => Some("!"),
            Some(TokenKind::Amp) => Some("&"),
            Some(TokenKind::Star) => Some("*"),
            _ => None,
        };
        if let Some(op) = op {
            self.advance();
            let expr = self.parse_postfix()?;
            Ok(Expression { kind: ExprKind::Unary { op: op.into(), expr: Box::new(expr) }, line, col })
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> Result<Expression, CompileError> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.peek_kind() {
                Some(TokenKind::LBracket) => {
                    let (line, col) = self.current_pos();
                    self.advance(); 
                    if self.peek_kind() == Some(&TokenKind::DotDot) {
                        self.advance();
                        let end = if self.peek_kind() != Some(&TokenKind::RBracket) {
                            Some(Box::new(self.with_struct_lit_allowed(true, |p| p.parse_expression())?))
                        } else {
                            None
                        };
                        self.expect(TokenKind::RBracket, "']'")?;
                        expr = Expression { kind: ExprKind::Slice { base: Box::new(expr), start: None, end }, line, col };
                    } else {
                        let first = self.with_struct_lit_allowed(true, |p| p.parse_expression())?;
                        if self.peek_kind() == Some(&TokenKind::DotDot) {
                            self.advance();
                            let end = if self.peek_kind() != Some(&TokenKind::RBracket) {
                                Some(Box::new(self.with_struct_lit_allowed(true, |p| p.parse_expression())?))
                            } else {
                                None
                            };
                            self.expect(TokenKind::RBracket, "']'")?;
                            expr = Expression {
                                kind: ExprKind::Slice { base: Box::new(expr), start: Some(Box::new(first)), end },
                                line,
                                col,
                            };
                        } else {
                            self.expect(TokenKind::RBracket, "']'")?;
                            expr = Expression { kind: ExprKind::Index { base: Box::new(expr), index: Box::new(first) }, line, col };
                        }
                    }
                }
                Some(TokenKind::Dot) => {
                    let (line, col) = self.current_pos();
                    self.advance();
                    let member = self.expect_identifier("nom de membre")?;
                    expr = Expression { kind: ExprKind::Member { base: Box::new(expr), member }, line, col };
                }
                Some(TokenKind::LParen) => {
                    let (line, col) = self.current_pos();
                    self.advance();
                    let args = if self.peek_kind() != Some(&TokenKind::RParen) {
                        self.with_struct_lit_allowed(true, |p| p.parse_expression_list())?
                    } else {
                        Vec::new()
                    };
                    self.expect(TokenKind::RParen, "')'")?;
                    expr = Expression { kind: ExprKind::Call { callee: Box::new(expr), args }, line, col };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression, CompileError> {
        let (line, col) = self.current_pos();
        match self.peek_kind().cloned() {
            Some(TokenKind::Identifier(name)) => {
                if name == "cast" {
                    self.advance();
                    self.expect(TokenKind::LParen, "'(' après 'cast'")?;
                    let ty = self.with_struct_lit_allowed(true, |p| p.parse_type())?;
                    self.expect(TokenKind::RParen, "')'")?;
                    let expr = self.parse_expression()?;
                    return Ok(Expression { kind: ExprKind::Cast { ty, expr: Box::new(expr) }, line, col });
                }
                if name == "va_start" {
                    self.advance();
                    self.expect(TokenKind::LParen, "'(' après 'va_start'")?;
                    self.expect(TokenKind::RParen, "')' (« va_start » ne prend pas d'argument)")?;
                    return Ok(Expression { kind: ExprKind::VaStart, line, col });
                }
                if name == "va_arg" {
                    self.advance();
                    self.expect(TokenKind::LParen, "'(' après 'va_arg'")?;
                    let list = self.with_struct_lit_allowed(true, |p| p.parse_expression())?;
                    self.expect(TokenKind::Comma, "',' entre la liste d'arguments et le type à lire")?;
                    let ty = self.with_struct_lit_allowed(true, |p| p.parse_type())?;
                    self.expect(TokenKind::RParen, "')'")?;
                    return Ok(Expression { kind: ExprKind::VaArg { list: Box::new(list), ty }, line, col });
                }
                if name == "va_end" {
                    self.advance();
                    self.expect(TokenKind::LParen, "'(' après 'va_end'")?;
                    let list = self.with_struct_lit_allowed(true, |p| p.parse_expression())?;
                    self.expect(TokenKind::RParen, "')'")?;
                    return Ok(Expression { kind: ExprKind::VaEnd(Box::new(list)), line, col });
                }
                if name == "true" || name == "false" {
                    self.advance();
                    return Ok(Expression { kind: ExprKind::BoolLiteral(name == "true"), line, col });
                }
                if name == "null" {
                    self.advance();
                    return Ok(Expression { kind: ExprKind::Null, line, col });
                }
                self.advance();
                if self.allow_struct_lit && self.peek_kind() == Some(&TokenKind::LBrace) {
                    return self.parse_struct_initializer(name, line, col);
                }
                Ok(Expression { kind: ExprKind::Identifier(name), line, col })
            }
            Some(TokenKind::IntLiteral(v)) => {
                self.advance();
                Ok(Expression { kind: ExprKind::IntLiteral(v), line, col })
            }
            Some(TokenKind::FloatLiteral(v)) => {
                self.advance();
                Ok(Expression { kind: ExprKind::FloatLiteral(v), line, col })
            }
            Some(TokenKind::StringLiteral(s)) => {
                self.advance();
                Ok(Expression { kind: ExprKind::StringLiteral(s), line, col })
            }
            Some(TokenKind::CharLiteral(c)) => {
                self.advance();
                Ok(Expression { kind: ExprKind::CharLiteral(c), line, col })
            }
            Some(TokenKind::LParen) => {
                self.advance();
                let expr = self.with_struct_lit_allowed(true, |p| p.parse_expression())?;
                self.expect(TokenKind::RParen, "')'")?;
                Ok(expr)
            }
            _ => Err(self.error_here(format!("expression attendue, trouvé {}", self.describe_current()))),
        }
    }

    fn parse_struct_initializer(&mut self, name: String, line: usize, col: usize) -> Result<Expression, CompileError> {
        self.expect(TokenKind::LBrace, "'{'")?;
        let old = self.allow_struct_lit;
        self.allow_struct_lit = true;

        let mut fields: Vec<(String, Expression)> = Vec::new();
        let mut err: Option<CompileError> = None;
        if self.peek_kind() != Some(&TokenKind::RBrace) {
            loop {
                match self.parse_field_init() {
                    Ok(pair) => fields.push(pair),
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
        self.expect(TokenKind::RBrace, "'}'")?;
        Ok(Expression { kind: ExprKind::StructInit { name, fields }, line, col })
    }

    fn parse_field_init(&mut self) -> Result<(String, Expression), CompileError> {
        let fname = self.expect_identifier("nom de champ")?;
        self.expect(TokenKind::Colon, "':'")?;
        let fexpr = self.parse_expression()?;
        Ok((fname, fexpr))
    }
}