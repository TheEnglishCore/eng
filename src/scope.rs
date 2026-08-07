use std::collections::HashMap;

use crate::value::Value;

pub struct ScopeStack {
    globals: HashMap<String, Value>,
    locals: Vec<HashMap<String, Value>>,
}

impl ScopeStack {
    pub fn new() -> Self {
        Self {
            globals: HashMap::new(),
            locals: Vec::new(),
        }
    }

    pub fn push_frame(&mut self) {
        self.locals.push(HashMap::new());
    }

    pub fn pop_frame(&mut self) {
        self.locals.pop();
    }

    pub fn set(&mut self, name: String, value: Value) {
        if let Some(frame) = self.locals.last_mut() {
            frame.insert(name, value);
        } else {
            self.globals.insert(name, value);
        }
    }

    pub fn set_global(&mut self, name: String, value: Value) {
        self.globals.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        for frame in self.locals.iter().rev() {
            if let Some(v) = frame.get(name) {
                return Some(v.clone());
            }
        }
        self.globals.get(name).cloned()
    }

    pub fn globals(&self) -> &HashMap<String, Value> {
        &self.globals
    }

    pub fn locals_depth(&self) -> usize {
        self.locals.len()
    }
}
