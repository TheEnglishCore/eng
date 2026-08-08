use crate::bytecode::{Chunk, Instruction};
use crate::error::{EnglingError, Result};
use crate::scope::ScopeStack;
use crate::value::Value;

struct CallFrame {
    chunk: Chunk,
    ip: usize,
    stack_base: usize,
}

/// Function the VM calls for every `Print` statement. Defaults to
/// `println!`; tests can swap it for a writer that captures output.
pub type PrintFn = Box<dyn FnMut(&Value) + Send>;

/// Function the VM calls for every `Ask` statement. Receives the already
/// evaluated prompt string and must return the line the user typed (with
/// the trailing newline stripped). Defaults to reading from stdin via
/// `std::io::stdin().read_line`; tests can swap it for a closure that
/// returns canned input.
pub type InputFn = Box<dyn FnMut(&str) -> Result<String> + Send>;

pub struct VM {
    stack: Vec<Value>,
    scopes: ScopeStack,
    frames: Vec<CallFrame>,
    print_fn: PrintFn,
    input_fn: InputFn,
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

impl VM {
    pub fn new() -> Self {
        Self::with_printer(Box::new(|v| println!("{v}")))
    }

    pub fn with_printer(print_fn: PrintFn) -> Self {
        Self::with_printer_and_input(print_fn, default_input_reader())
    }

    pub fn with_printer_and_input(print_fn: PrintFn, input_fn: InputFn) -> Self {
        Self {
            stack: Vec::new(),
            scopes: ScopeStack::new(),
            frames: Vec::new(),
            print_fn,
            input_fn,
        }
    }

    pub fn set_printer(&mut self, print_fn: PrintFn) {
        self.print_fn = print_fn;
    }

    pub fn set_input_reader(&mut self, input_fn: InputFn) {
        self.input_fn = input_fn;
    }

    pub fn scopes_mut(&mut self) -> &mut ScopeStack {
        &mut self.scopes
    }

    pub fn run(&mut self, chunk: Chunk) -> Result<()> {
        self.frames.push(CallFrame {
            chunk,
            ip: 0,
            stack_base: 0,
        });
        self.run_frames()
    }

    pub fn call_function(&mut self, name: &str, args: Vec<Value>) -> Result<Value> {
        let func = self
            .scopes
            .get(name)
            .ok_or_else(|| EnglingError::runtime(format!("Function '{name}' is not defined")))?;

        let Value::Function(func) = func else {
            return Err(EnglingError::runtime(format!("'{name}' is not a function")));
        };

        if args.len() != func.params.len() {
            return Err(EnglingError::runtime(format!(
                "Function '{}' expects {} arguments, got {}",
                name,
                func.params.len(),
                args.len()
            )));
        }

        self.scopes.push_frame();
        for (param, arg) in func.params.iter().zip(args) {
            self.scopes.set(param.clone(), arg);
        }

        let stack_base = self.stack.len();
        self.frames.push(CallFrame {
            chunk: func.chunk.clone(),
            ip: 0,
            stack_base,
        });

        self.run_frames()?;

        let result = self.stack.pop().unwrap_or(Value::Nothing);
        Ok(result)
    }

    fn run_frames(&mut self) -> Result<()> {
        while let Some(frame_idx) = self.frames.len().checked_sub(1) {
            let finished = self.run_one_instruction(frame_idx)?;
            if finished {
                self.frames.pop().unwrap();
                if self.frames.is_empty() {
                    break;
                }
                // Return value stays on stack for caller
                if self.scopes.locals_depth() > 0 {
                    self.scopes.pop_frame();
                }
            }
        }
        Ok(())
    }

    fn run_one_instruction(&mut self, frame_idx: usize) -> Result<bool> {
        let ip = self.frames[frame_idx].ip;
        let instruction = self.frames[frame_idx].chunk.code[ip].clone();
        let constants = self.frames[frame_idx].chunk.constants.clone();

        self.frames[frame_idx].ip += 1;

        match instruction {
            Instruction::LoadConstant(index) => {
                self.stack.push(constants[index].clone());
            }
            Instruction::LoadVariable(name) => {
                let value = self.scopes.get(&name).unwrap_or(Value::Nothing);
                self.stack.push(value);
            }
            Instruction::StoreVariable(name) => {
                let value = self.stack.pop().unwrap();
                self.scopes.set(name, value);
            }
            Instruction::Print => {
                let value = self.stack.pop().unwrap();
                (self.print_fn)(&value);
            }
            Instruction::Input(name) => {
                let prompt = self.stack.pop().unwrap();
                let prompt_str = prompt.to_string();
                let line = (self.input_fn)(&prompt_str)?;
                let trimmed = line.trim_end_matches(['\r', '\n']).to_string();
                self.scopes.set(name, Value::String(trimmed));
            }
            Instruction::Pop => {
                self.stack.pop();
            }
            Instruction::Add => self.binary_add()?,
            Instruction::Subtract => self.binary_number(|a, b| a - b)?,
            Instruction::Multiply => self.binary_number(|a, b| a * b)?,
            Instruction::Divide => self.binary_number(|a, b| a / b)?,
            Instruction::Modulo => self.binary_number(|a, b| a % b)?,
            Instruction::Equal => self.compare(Self::values_equal)?,
            Instruction::NotEqual => self.compare(|a, b| !Self::values_equal(a, b))?,
            Instruction::Greater => self.compare_numbers(|a, b| a > b)?,
            Instruction::Less => self.compare_numbers(|a, b| a < b)?,
            Instruction::GreaterEqual => self.compare_numbers(|a, b| a >= b)?,
            Instruction::LessEqual => self.compare_numbers(|a, b| a <= b)?,
            Instruction::And => self.logical(|a, b| a && b),
            Instruction::Or => self.logical(|a, b| a || b),
            Instruction::Jump(offset) => {
                self.frames[frame_idx].ip = offset;
            }
            Instruction::JumpIfFalse(offset) => {
                let value = self.stack.pop().unwrap();
                if !value.is_truthy() {
                    self.frames[frame_idx].ip = offset;
                }
            }
            Instruction::Call(name, argc) => {
                let mut args = Vec::new();
                for _ in 0..argc {
                    args.insert(0, self.stack.pop().unwrap());
                }
                args.reverse();

                let func_val = self.scopes.get(&name).ok_or_else(|| {
                    EnglingError::runtime(format!("Function '{name}' is not defined"))
                })?;

                let Value::Function(func) = func_val else {
                    return Err(EnglingError::runtime(format!("'{name}' is not a function")));
                };

                if args.len() != func.params.len() {
                    return Err(EnglingError::runtime(format!(
                        "Function '{}' expects {} arguments, got {}",
                        name,
                        func.params.len(),
                        args.len()
                    )));
                }

                self.scopes.push_frame();
                for (param, arg) in func.params.iter().zip(args) {
                    self.scopes.set(param.clone(), arg);
                }

                let stack_base = self.stack.len();
                self.frames.push(CallFrame {
                    chunk: func.chunk.clone(),
                    ip: 0,
                    stack_base,
                });
            }
            Instruction::Return => {
                return Ok(true);
            }
            Instruction::ListNew => {
                self.stack.push(Value::List(Vec::new()));
            }
            Instruction::ListPush(name) => {
                let value = self.stack.pop().unwrap();
                let mut list = self.scopes.get(&name).unwrap_or(Value::List(Vec::new()));
                if let Value::List(ref mut items) = list {
                    items.push(value);
                    self.scopes.set(name, list);
                }
            }
            Instruction::ListGet => {
                let index_val = self.stack.pop().unwrap();
                let list_val = self.stack.pop().unwrap();
                let index = match index_val {
                    Value::Number(n) => n as usize,
                    _ => {
                        return Err(EnglingError::runtime("List index must be a number"));
                    }
                };
                if let Value::List(items) = list_val {
                    if index >= items.len() {
                        return Err(EnglingError::runtime(format!(
                            "List index {} is out of range (list has {} items)",
                            index + 1,
                            items.len()
                        )));
                    }
                    self.stack.push(items[index].clone());
                } else {
                    return Err(EnglingError::runtime("Expected a list"));
                }
            }
            Instruction::ListSet(name) => {
                let index_val = self.stack.pop().unwrap();
                let value = self.stack.pop().unwrap();
                let index = match index_val {
                    Value::Number(n) => n as usize,
                    _ => return Err(EnglingError::runtime("List index must be a number")),
                };
                let mut list = self.scopes.get(&name).ok_or_else(|| {
                    EnglingError::runtime(format!("List '{name}' is not defined"))
                })?;
                if let Value::List(ref mut items) = list {
                    if index >= items.len() {
                        return Err(EnglingError::runtime(format!(
                            "List index {} is out of range (list has {} items)",
                            index + 1,
                            items.len()
                        )));
                    }
                    items[index] = value;
                    self.scopes.set(name, list);
                }
            }
            Instruction::ListLength => {
                let list_val = self.stack.pop().unwrap();
                if let Value::List(items) = list_val {
                    self.stack.push(Value::Number(items.len() as f64));
                } else {
                    return Err(EnglingError::runtime("Expected a list"));
                }
            }
            #[cfg(feature = "ui")]
            Instruction::SetLabelText(name) => {
                let value = self.stack.pop().unwrap();
                let text = value.to_string();
                crate::ui::update_label_text(&name, text)?;
            }
        }
        Ok(false)
    }

    fn binary_number(&mut self, op: fn(f64, f64) -> f64) -> Result<()> {
        let right = self.stack.pop().unwrap();
        let left = self.stack.pop().unwrap();
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                self.stack.push(Value::Number(op(a, b)));
            }
            _ => return Err(EnglingError::runtime("Numbers required for this operation")),
        }
        Ok(())
    }

    fn binary_add(&mut self) -> Result<()> {
        let right = self.stack.pop().unwrap();
        let left = self.stack.pop().unwrap();
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                self.stack.push(Value::Number(a + b));
            }
            (Value::String(a), Value::String(b)) => {
                self.stack.push(Value::String(format!("{a}{b}")));
            }
            _ => return Err(EnglingError::runtime("Cannot add these values")),
        }
        Ok(())
    }

    fn compare_numbers(&mut self, op: fn(f64, f64) -> bool) -> Result<()> {
        let right = self.stack.pop().unwrap();
        let left = self.stack.pop().unwrap();
        match (left, right) {
            (Value::Number(a), Value::Number(b)) => {
                self.stack.push(Value::Boolean(op(a, b)));
            }
            _ => return Err(EnglingError::runtime("Numbers required for comparison")),
        }
        Ok(())
    }

    fn compare(&mut self, op: fn(&Value, &Value) -> bool) -> Result<()> {
        let right = self.stack.pop().unwrap();
        let left = self.stack.pop().unwrap();
        self.stack.push(Value::Boolean(op(&left, &right)));
        Ok(())
    }

    fn values_equal(a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Number(x), Value::Number(y)) => x == y,
            (Value::Boolean(x), Value::Boolean(y)) => x == y,
            (Value::String(x), Value::String(y)) => x == y,
            (Value::Nothing, Value::Nothing) => true,
            _ => false,
        }
    }

    fn logical(&mut self, op: fn(bool, bool) -> bool) {
        let right = self.stack.pop().unwrap();
        let left = self.stack.pop().unwrap();
        self.stack
            .push(Value::Boolean(op(left.is_truthy(), right.is_truthy())));
    }
}

/// Default `Input` reader: writes the prompt to stdout (without a
/// trailing newline, so it feels like an interactive prompt) and reads a
/// single line from stdin. We flush stdout because terminals do not
/// line-buffer when piped.
fn default_input_reader() -> InputFn {
    use std::io::{self, Write};
    Box::new(move |prompt: &str| {
        print!("{prompt}");
        io::stdout()
            .flush()
            .map_err(|e| EnglingError::runtime(format!("Could not flush prompt: {e}")))?;
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|e| EnglingError::runtime(format!("Could not read input: {e}")))?;
        Ok(line)
    })
}
