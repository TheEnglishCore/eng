use rustyline::DefaultEditor;

use crate::error::report;
use crate::runtime;
use crate::vm::VM;

/// Track the open block depth so the REPL only submits a program once all
/// `If`/`Otherwise`/`While`/`Repeat`/`Function` blocks have been closed by
/// an `End.` plus a period. The lexer hands the REPL a count of unclosed
/// "block openers" via `block_depth(&str)`.
pub fn block_depth(source: &str) -> i32 {
    let mut depth: i32 = 0;
    let mut chars = source.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '#' {
            // skip line comment
            for c in chars.by_ref() {
                if c == '\n' {
                    break;
                }
            }
            continue;
        }
        if c == '"' {
            // skip string literal
            for c in chars.by_ref() {
                if c == '"' || c == '\n' {
                    break;
                }
            }
            continue;
        }
        if c.is_alphabetic() {
            let mut word = String::from(c);
            while let Some(&nc) = chars.peek() {
                if nc.is_alphanumeric() || nc == '_' {
                    word.push(nc);
                    chars.next();
                } else {
                    break;
                }
            }
            let lower = word.to_lowercase();
            // Only count block openers, not the "of"/"or" that follow them.
            match lower.as_str() {
                "if" => depth += 1,
                "otherwise" => depth += 1,
                "while" => depth += 1,
                "repeat" => {
                    // "Repeat N times" opens one block; the `times` is a separate
                    // token but we only count it once.
                    depth += 1;
                }
                "function" => depth += 1,
                "end" => depth -= 1,
                _ => {}
            }
        }
    }
    depth
}

pub fn starts() {
    let mut vm = VM::new();
    let mut editor = DefaultEditor::new().unwrap();
    let mut buffer = String::new();

    println!("Engling v0.1.0 — type 'exit' to quit");

    loop {
        let prompt = if buffer.is_empty() { "> " } else { "... " };
        let line = match editor.readline(prompt) {
            Ok(input) => input,
            Err(_) => break,
        };

        if buffer.is_empty() && line.trim() == "exit" {
            break;
        }

        if !buffer.is_empty() {
            buffer.push('\n');
        }
        buffer.push_str(&line);

        // Wait for a complete program: the buffer must end with a period
        // that is not inside a string, AND all opened blocks must be closed.
        if !buffer.trim_end().ends_with('.') {
            continue;
        }
        if block_depth(&buffer) > 0 {
            continue;
        }

        match runtime::execute(buffer.clone(), &mut vm) {
            Ok(()) => buffer.clear(),
            Err(e) => {
                report(&e);
                buffer.clear();
            }
        }
    }
}

// Backwards-compatible alias used by cli.rs.
pub fn start() {
    starts();
}
