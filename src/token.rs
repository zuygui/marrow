#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    IntLiteral(i128),
    FloatLiteral(f64),
    StringLiteral(String),
    CharLiteral(char),

    ColonColon, // ::
    Colon,      // :
    Semicolon,  // ;
    Comma,      // ,
    Dot,        // .
    DotDot,     // ..
    Ellipsis,   // ...
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    LBracket,   // [
    RBracket,   // ]
    Arrow,      // ->
    FatArrow,   // =>
    At,         // @

    Eq,      // =
    EqEq,    // ==
    NotEq,   // !=
    Not,     // !
    Lt,      // <
    LtEq,    // <=
    Gt,      // >
    GtEq,    // >=
    Plus,    // +
    PlusEq,  // +=
    Minus,   // -
    MinusEq, // -=
    Star,    // *
    StarEq,  // *=
    Slash,   // /
    SlashEq, // /=
    Percent, // %
    AmpAmp,  // &&
    PipePipe,// ||
    Amp,     // &

    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
    pub len: usize,
}