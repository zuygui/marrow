#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<GlobalItem>,
}

#[derive(Debug, Clone)]
pub struct GlobalItem {
    pub decorators: Vec<Decorator>,
    pub binding: BindingDecl,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct Decorator {
    pub name: String,
    pub args: Option<Vec<Expression>>,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct BindingDecl {
    pub ty: Option<Type>,
    pub name: String,
    pub value: BindingValue,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub enum BindingValue {
    Function(FunctionDef),
    ExpressionFunction(ExpressionFunctionDef),
    Struct(StructDef),
    Expr(Expression),
}

#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub params: Vec<Parameter>,
    pub ret_type: Option<Type>,
    pub variadic: bool,
    pub body: Option<Block>,
}

#[derive(Debug, Clone)]
pub struct ExpressionFunctionDef {
    pub params: Vec<Parameter>,
    pub ret_type: Option<Type>,
    pub variadic: bool,
    pub body: Box<Expression>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub ty: Type,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct StructDef {
    pub fields: Vec<StructField>,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub ty: Type,
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum Type {
    Builtin(String),
    Custom(String),
    StaticArray(Box<Expression>, Box<Type>),
    Pointer(Box<Type>),
    Slice(Box<Type>),
}

pub type Block = Vec<Statement>;

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StmtKind,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub enum StmtKind {
    Block(Block),
    LocalVarDecl(LocalVarDecl),
    If(IfStmt),
    While(WhileStmt),
    For(ForStmt),
    Return(Option<Expression>),
    Expr(Expression),
}

#[derive(Debug, Clone)]
pub enum LocalVarDecl {
    Mutable {
        ty: Option<Type>,
        name: String,
        value: Expression,
    },
    Constant {
        ty: Option<Type>,
        name: String,
        value: BindingValue,
    },
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub cond: Expression,
    pub then_block: Block,
    pub else_branch: Option<ElseBranch>,
}

#[derive(Debug, Clone)]
pub enum ElseBranch {
    Block(Block),
    If(Box<IfStmt>),
}

#[derive(Debug, Clone)]
pub struct WhileStmt {
    pub cond: Expression,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct ForStmt {
    pub init: Option<Box<LocalVarDecl>>,
    pub cond: Option<Expression>,
    pub post: Option<Expression>,
    pub body: Block,
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub kind: ExprKind,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub enum ExprKind {
    Identifier(String),
    IntLiteral(i128),
    FloatLiteral(f64),
    StringLiteral(String),
    CharLiteral(char),
    BoolLiteral(bool),
    Null,
    StructInit {
        name: String,
        fields: Vec<(String, Expression)>,
    },
    Cast {
        ty: Type,
        expr: Box<Expression>,
    },
    Unary {
        op: String,
        expr: Box<Expression>,
    },
    Binary {
        op: String,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Assign {
        op: String,
        target: Box<Expression>,
        value: Box<Expression>,
    },
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    Index {
        base: Box<Expression>,
        index: Box<Expression>,
    },
    Member {
        base: Box<Expression>,
        member: String,
    },
    Slice {
        base: Box<Expression>,
        start: Option<Box<Expression>>,
        end: Option<Box<Expression>>,
    },
    VaStart,
    VaArg {
        list: Box<Expression>,
        ty: Type,
    },
    VaEnd(Box<Expression>),
}