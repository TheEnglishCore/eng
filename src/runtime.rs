use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::compiler::Compiler;
use crate::error::{EnglingError, Result};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::value::Value;
use crate::vm::VM;

pub struct ModuleLoader {
    base_dir: PathBuf,
    cache: HashMap<String, HashMap<String, Value>>,
    loading: Vec<String>,
}

impl ModuleLoader {
    pub fn new(base_dir: PathBuf) -> Self {
        Self {
            base_dir,
            cache: HashMap::new(),
            loading: Vec::new(),
        }
    }

    pub fn resolve_path(&self, name: &str) -> PathBuf {
        let filename = format!("{name}.eng");
        let local = self.base_dir.join(&filename);
        if local.exists() {
            return local;
        }
        if let Ok(path_var) = std::env::var("ENGLING_PATH") {
            for dir in path_var.split(';').chain(path_var.split(':')) {
                let candidate = PathBuf::from(dir.trim()).join(&filename);
                if candidate.exists() {
                    return candidate;
                }
            }
        }
        local
    }

    pub fn load_exports(&mut self, module_name: &str) -> Result<HashMap<String, Value>> {
        if let Some(exports) = self.cache.get(module_name) {
            return Ok(exports.clone());
        }

        if self.loading.contains(&module_name.to_string()) {
            return Err(EnglingError::Module(format!(
                "Circular import detected: {module_name}"
            )));
        }

        self.loading.push(module_name.to_string());

        let path = self.resolve_path(module_name);
        let source = std::fs::read_to_string(&path).map_err(|_| {
            EnglingError::Module(format!(
                "Could not find module '{module_name}' (looked for {})",
                path.display()
            ))
        })?;

        let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let mut sub_loader = ModuleLoader {
            base_dir,
            cache: self.cache.clone(),
            loading: self.loading.clone(),
        };

        let mut vm = VM::new();
        run_source_with_loader(&source, &mut vm, &mut sub_loader)?;

        let mut exports = HashMap::new();
        for (name, value) in vm.scopes_mut().globals() {
            exports.insert(name.clone(), value.clone());
        }

        self.loading.pop();
        self.cache.insert(module_name.to_string(), exports.clone());
        Ok(exports)
    }
}

pub fn run_source_with_loader(
    source: &str,
    vm: &mut VM,
    loader: &mut ModuleLoader,
) -> Result<()> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::with_source(tokens, source.to_string());
    let program = parser.parse_program()?;

    // Handle imports
    for stmt in &program.statements {
        match stmt {
            crate::ast::Statement::Import { module } => {
                let exports = loader.load_exports(module)?;
                for (name, value) in exports {
                    vm.scopes_mut().set_global(name, value);
                }
            }
            crate::ast::Statement::ImportFrom { module, names } => {
                let exports = loader.load_exports(module)?;
                for name in names {
                    if let Some(value) = exports.get(name) {
                        vm.scopes_mut().set_global(name.clone(), value.clone());
                    } else {
                        return Err(EnglingError::Module(format!(
                            "Module '{module}' does not export '{name}'"
                        )));
                    }
                }
            }
            _ => {}
        }
    }

    // Pre-register UI declarations so they exist before the program runs.
    #[cfg(feature = "ui")]
    {
        if program.statements.iter().any(is_ui_statement) {
            crate::ui::init_ui();
            for stmt in &program.statements {
                if is_ui_statement(stmt) {
                    crate::ui::process_ui_statement(stmt)?;
                }
            }
        }
    }

    let compiler = Compiler::new();
    let chunk = compiler.compile(program);
    vm.run(chunk)
}

#[cfg(feature = "ui")]
fn is_ui_statement(stmt: &crate::ast::Statement) -> bool {
    matches!(
        stmt,
        crate::ast::Statement::WindowDecl { .. }
            | crate::ast::Statement::WidgetDecl { .. }
            | crate::ast::Statement::EventHandler { .. }
    )
}

pub fn execute(source: String, vm: &mut VM) -> Result<()> {
    let mut loader = ModuleLoader::new(PathBuf::from("."));
    run_source_with_loader(&source, vm, &mut loader)
}

pub fn execute_file(path: &Path, vm: &mut VM) -> Result<()> {
    let source = std::fs::read_to_string(path).map_err(|e| {
        EnglingError::runtime(format!("Could not read {}: {e}", path.display()))
    })?;
    let base_dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut loader = ModuleLoader::new(base_dir);
    run_source_with_loader(&source, vm, &mut loader)
}

