#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Variable {
        name: String,
        value: Expression,
    },
    Assignment {
        name: String,
        value: Expression,
    },
    Print {
        expression: Expression,
    },
    Input {
        prompt: Expression,
        variable: String,
    },
    If {
        condition: Expression,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
    },
    Repeat {
        count: Expression,
        body: Vec<Statement>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    FunctionDef {
        name: String,
        params: Vec<String>,
        body: Option<Vec<Statement>>,
        return_expr: Option<Expression>,
    },
    Run {
        name: String,
        args: Vec<Expression>,
    },
    ListDecl {
        name: String,
    },
    ListAdd {
        name: String,
        value: Expression,
    },
    ListSet {
        name: String,
        index: usize,
        value: Expression,
    },
    Import {
        module: String,
    },
    ImportFrom {
        module: String,
        names: Vec<String>,
    },
    ModuleDecl {
        name: String,
    },
    #[cfg(feature = "ui")]
    WindowDecl {
        name: String,
        title: String,
    },
    #[cfg(feature = "ui")]
    WidgetDecl {
        window: String,
        kind: WidgetKind,
        label: String,
    },
    #[cfg(feature = "ui")]
    EventHandler {
        button_label: String,
        function: String,
    },
    #[cfg(feature = "ui")]
    SetLabelText {
        label_name: String,
        value: Expression,
    },
}

#[derive(Debug, Clone)]
#[cfg(feature = "ui")]
pub enum WidgetKind {
    Button,
    Label,
    TextField,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Number(f64),
    String(String),
    Boolean(bool),
    Variable(String),
    Binary {
        left: Box<Expression>,
        operator: Operator,
        right: Box<Expression>,
    },
    Call {
        name: String,
        args: Vec<Expression>,
    },
    ListGet {
        name: String,
        index: usize,
    },
    ListLength {
        name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    And,
    Or,
}
