use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::ast::*;
use crate::bytecode::*;
use crate::value::{Function, Value};

static REPEAT_COUNTER: AtomicUsize = AtomicUsize::new(0);

pub struct Compiler {
    chunk: Chunk,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            chunk: Chunk::new(),
        }
    }

    pub fn compile(mut self, program: Program) -> Chunk {
        for statement in program.statements {
            self.statement(statement);
        }
        self.chunk.emit(Instruction::Return);
        self.chunk
    }

    pub fn compile_function(
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
        return_expr: Option<Expression>,
    ) -> Function {
        let mut compiler = Compiler::new();
        let has_return = return_expr.is_some();
        if let Some(expr) = return_expr {
            compiler.expression(expr);
        } else {
            for stmt in body {
                compiler.statement(stmt);
            }
            let nothing_idx = compiler.chunk.add_constant(Value::Nothing);
            compiler.chunk.emit(Instruction::LoadConstant(nothing_idx));
        }
        compiler.chunk.emit(Instruction::Return);
        Function {
            name,
            params,
            chunk: compiler.chunk,
            has_return_expr: has_return,
        }
    }

    fn statement(&mut self, statement: Statement) {
        match statement {
            Statement::Variable { name, value } | Statement::Assignment { name, value } => {
                self.expression(value);
                self.chunk.emit(Instruction::StoreVariable(name));
            }
            Statement::Print { expression } => {
                self.expression(expression);
                self.chunk.emit(Instruction::Print);
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.expression(condition);
                let jump_false = self.chunk.emit(Instruction::JumpIfFalse(0));
                for stmt in then_block {
                    self.statement(stmt);
                }
                if let Some(else_stmts) = else_block {
                    let jump_end = self.chunk.emit(Instruction::Jump(0));
                    let else_start = self.chunk.code.len();
                    self.chunk.patch_jump(jump_false, else_start);
                    for stmt in else_stmts {
                        self.statement(stmt);
                    }
                    let end = self.chunk.code.len();
                    self.chunk.patch_jump(jump_end, end);
                } else {
                    let end = self.chunk.code.len();
                    self.chunk.patch_jump(jump_false, end);
                }
            }
            Statement::Repeat { count, body } => {
                let counter_name = format!(
                    "__repeat_{}",
                    REPEAT_COUNTER.fetch_add(1, Ordering::SeqCst)
                );
                self.expression(count);
                self.chunk.emit(Instruction::StoreVariable(counter_name.clone()));
                let loop_start = self.chunk.code.len();
                self.chunk.emit(Instruction::LoadVariable(counter_name.clone()));
                let zero_idx = self.chunk.add_constant(Value::Number(0.0));
                self.chunk.emit(Instruction::LoadConstant(zero_idx));
                self.chunk.emit(Instruction::Greater);
                let jump_end = self.chunk.emit(Instruction::JumpIfFalse(0));
                for stmt in body {
                    self.statement(stmt);
                }
                self.chunk.emit(Instruction::LoadVariable(counter_name.clone()));
                let one_idx = self.chunk.add_constant(Value::Number(1.0));
                self.chunk.emit(Instruction::LoadConstant(one_idx));
                self.chunk.emit(Instruction::Subtract);
                self.chunk.emit(Instruction::StoreVariable(counter_name));
                self.chunk.emit(Instruction::Jump(loop_start));
                let end = self.chunk.code.len();
                self.chunk.patch_jump(jump_end, end);
            }
            Statement::While { condition, body } => {
                let loop_start = self.chunk.code.len();
                self.expression(condition);
                let jump_end = self.chunk.emit(Instruction::JumpIfFalse(0));
                for stmt in body {
                    self.statement(stmt);
                }
                self.chunk.emit(Instruction::Jump(loop_start));
                let end = self.chunk.code.len();
                self.chunk.patch_jump(jump_end, end);
            }
            Statement::FunctionDef {
                name,
                params,
                body,
                return_expr,
            } => {
                let func = Compiler::compile_function(
                    name.clone(),
                    params,
                    body.unwrap_or_default(),
                    return_expr,
                );
                let index = self.chunk.add_constant(Value::Function(Arc::new(func)));
                self.chunk.emit(Instruction::LoadConstant(index));
                self.chunk.emit(Instruction::StoreVariable(name));
            }
            Statement::Run { name, args } => {
                let argc = args.len();
                for arg in args {
                    self.expression(arg);
                }
                self.chunk.emit(Instruction::Call(name, argc));
                self.chunk.emit(Instruction::Pop);
            }
            Statement::ListDecl { name } => {
                self.chunk.emit(Instruction::ListNew);
                self.chunk.emit(Instruction::StoreVariable(name));
            }
            Statement::ListAdd { name, value } => {
                self.expression(value);
                self.chunk.emit(Instruction::ListPush(name));
            }
            Statement::ListSet { name, index, value } => {
                self.expression(value);
                let idx = self.chunk.add_constant(Value::Number(index as f64));
                self.chunk.emit(Instruction::LoadConstant(idx));
                self.chunk.emit(Instruction::ListSet(name));
            }
            Statement::Import { .. } | Statement::ImportFrom { .. } | Statement::ModuleDecl { .. } => {
                // Handled at runtime, not compiled
            }
            #[cfg(feature = "ui")]
            Statement::WindowDecl { .. }
            | Statement::WidgetDecl { .. }
            | Statement::EventHandler { .. } => {
                // Handled at runtime by UI layer
            }
            #[cfg(feature = "ui")]
            Statement::SetLabelText { label_name, value } => {
                self.expression(value);
                self.chunk.emit(Instruction::SetLabelText(label_name));
            }
        }
    }

    fn expression(&mut self, expression: Expression) {
        match expression {
            Expression::Number(n) => {
                let index = self.chunk.add_constant(Value::Number(n));
                self.chunk.emit(Instruction::LoadConstant(index));
            }
            Expression::String(s) => {
                let index = self.chunk.add_constant(Value::String(s));
                self.chunk.emit(Instruction::LoadConstant(index));
            }
            Expression::Boolean(b) => {
                let index = self.chunk.add_constant(Value::Boolean(b));
                self.chunk.emit(Instruction::LoadConstant(index));
            }
            Expression::Variable(name) => {
                self.chunk.emit(Instruction::LoadVariable(name));
            }
            Expression::Binary { left, operator, right } => {
                self.expression(*left);
                self.expression(*right);
                let instruction = match operator {
                    Operator::Add => Instruction::Add,
                    Operator::Subtract => Instruction::Subtract,
                    Operator::Multiply => Instruction::Multiply,
                    Operator::Divide => Instruction::Divide,
                    Operator::Modulo => Instruction::Modulo,
                    Operator::Equal => Instruction::Equal,
                    Operator::NotEqual => Instruction::NotEqual,
                    Operator::Greater => Instruction::Greater,
                    Operator::Less => Instruction::Less,
                    Operator::GreaterEqual => Instruction::GreaterEqual,
                    Operator::LessEqual => Instruction::LessEqual,
                    Operator::And => Instruction::And,
                    Operator::Or => Instruction::Or,
                };
                self.chunk.emit(instruction);
            }
            Expression::Call { name, args } => {
                let argc = args.len();
                for arg in args {
                    self.expression(arg);
                }
                self.chunk.emit(Instruction::Call(name, argc));
            }
            Expression::ListGet { name, index } => {
                self.chunk.emit(Instruction::LoadVariable(name));
                let idx = self.chunk.add_constant(Value::Number(index as f64));
                self.chunk.emit(Instruction::LoadConstant(idx));
                self.chunk.emit(Instruction::ListGet);
            }
            Expression::ListLength { name } => {
                self.chunk.emit(Instruction::LoadVariable(name));
                self.chunk.emit(Instruction::ListLength);
            }
        }
    }
}
