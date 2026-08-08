#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Assignment
    Let,
    Set,
    Make,
    Be,
    To,

    // I/O
    Print,
    Show,
    Display,
    Ask,
    Put,
    It,
    In,

    // Literals / booleans
    True,
    False,

    // Control flow
    If,
    Otherwise,
    End,
    Repeat,
    Times,
    While,
    Then,
    Comma,

    // Functions
    Define,
    Function,
    Called,
    That,
    Takes,
    Returns,
    Run,
    Call,
    With,
    A,
    Nothing,

    // Lists
    List,
    Add,
    Get,
    The,
    Item,
    Of,
    Length,
    First,
    Second,
    Third,
    Fourth,
    Fifth,
    St,
    Nd,
    Rd,
    Th,

    // Modules
    Import,
    From,
    Use,
    Module,
    Create,

    // UI
    Window,
    Titled,
    Button,
    Label,
    Text,
    Field,
    When,
    Clicked,
    Labeled,

    // Operators
    Plus,
    Minus,
    Multiplied,
    Divided,
    By,
    Modulo,
    Is,
    Equal,
    Not,
    Greater,
    Less,
    Than,
    And,
    Or,

    Identifier(String),
    Number(f64),
    String(String),
    Period,
    EOF,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
    /// Byte offset of the first character of this token in the source.
    pub byte_offset: usize,
    /// Length of the token in bytes (utf-8).
    pub byte_len: usize,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Self {
            kind,
            line,
            column,
            byte_offset: 0,
            byte_len: 0,
        }
    }

    pub fn with_span(
        kind: TokenKind,
        line: usize,
        column: usize,
        byte_offset: usize,
        byte_len: usize,
    ) -> Self {
        Self {
            kind,
            line,
            column,
            byte_offset,
            byte_len,
        }
    }
}
