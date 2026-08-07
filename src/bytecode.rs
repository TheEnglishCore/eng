use crate::value::Value;

#[derive(Debug, Clone)]
pub enum Instruction {
    LoadConstant(usize),
    LoadVariable(String),
    StoreVariable(String),
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Print,
    Pop,
    Equal,
    NotEqual,
    Greater,
    Less,
    GreaterEqual,
    LessEqual,
    And,
    Or,
    Jump(usize),
    JumpIfFalse(usize),
    Call(String, usize),
    Return,
    ListNew,
    ListPush(String),
    ListGet,
    ListSet(String),
    ListLength,
    #[cfg(feature = "ui")]
    SetLabelText(String),
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub constants: Vec<Value>,
    pub code: Vec<Instruction>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            code: Vec::new(),
        }
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn emit(&mut self, instruction: Instruction) -> usize {
        self.code.push(instruction);
        self.code.len() - 1
    }

    pub fn patch_jump(&mut self, index: usize, target: usize) {
        match &mut self.code[index] {
            Instruction::Jump(ref mut offset) | Instruction::JumpIfFalse(ref mut offset) => {
                *offset = target;
            }
            _ => panic!("Not a jump instruction at index {index}"),
        }
    }
}
