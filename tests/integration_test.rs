use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use engling::error::EnglingError;
use engling::runtime;
use engling::vm::VM;

/// Run an `.eng` source string in-process. `Print` writes to stdout,
/// which during a test run is captured by the cargo test harness.
fn run(source: &str) -> Result<(), EnglingError> {
    let mut vm = VM::new();
    runtime::execute(source.to_string(), &mut vm)
}

/// Run a source string and capture the lines `Print` emitted. Uses an
/// `Arc<Mutex<Vec<String>>>` shared between the print callback and the
/// caller; we extract the inner value after the VM drops the callback.
fn run_capture(source: &str) -> Result<Vec<String>, EnglingError> {
    let output: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let out_clone = Arc::clone(&output);
    {
        let printer = Box::new(move |v: &engling::value::Value| {
            out_clone.lock().unwrap().push(v.to_string());
        });
        let mut vm = VM::with_printer(printer);
        runtime::execute(source.to_string(), &mut vm)?;
    }
    // VM dropped; the closure (and `out_clone`) dropped. We hold the only
    // remaining `Arc`, so `try_unwrap` returns the inner mutex.
    match Arc::try_unwrap(output) {
        Ok(mutex) => Ok(mutex.into_inner().unwrap()),
        Err(_) => panic!("print callback captured multiple Arc references"),
    }
}

/// Run a fixture file and capture its output.
fn run_file_capture(path: &std::path::Path) -> Result<Vec<String>, EnglingError> {
    let output: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let out_clone = Arc::clone(&output);
    {
        let printer = Box::new(move |v: &engling::value::Value| {
            out_clone.lock().unwrap().push(v.to_string());
        });
        let mut vm = VM::with_printer(printer);
        runtime::execute_file(path, &mut vm)?;
    }
    match Arc::try_unwrap(output) {
        Ok(mutex) => Ok(mutex.into_inner().unwrap()),
        Err(_) => panic!("print callback captured multiple Arc references"),
    }
}

fn assert_lines(actual: Vec<String>, expected: &[&str]) {
    let expected: Vec<String> = expected.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        actual, expected,
        "\n--- expected:\n{expected:#?}\n--- actual:\n{actual:#?}\n"
    );
}

#[test]
fn hello_world_runs() {
    let res = run("Let greeting be \"Hello, world.\".\nPrint greeting.\n");
    assert!(res.is_ok(), "Expected program to run cleanly: {res:?}");
}

#[test]
fn hello_world_prints() {
    let out = run_capture("Let greeting be \"Hello, world.\".\nPrint greeting.\n").unwrap();
    assert_lines(out, &["Hello, world."]);
}

#[test]
fn arithmetic_runs() {
    let src = "
        Let a be 20.
        Let b be 5.
        Let sum be a plus b.
        Let product be a multiplied by b.
        Let quotient be a divided by b.
        Let remainder be a modulo b.
        Print sum.
        Print product.
        Print quotient.
        Print remainder.
    ";
    let res = run(src);
    assert!(res.is_ok(), "Expected arithmetic program to run cleanly");
}

#[test]
fn arithmetic_prints() {
    let src = "
        Let a be 20.
        Let b be 5.
        Print a plus b.
        Print a multiplied by b.
        Print a divided by b.
        Print a modulo b.
    ";
    let out = run_capture(src).unwrap();
    assert_lines(out, &["25", "100", "4", "0"]);
}

#[test]
fn strings_concat() {
    let src = "
        Let p1 be \"Hello, \".
        Let p2 be \"world.\".
        Let m be p1 plus p2.
        Print m.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn strings_concat_prints() {
    let out = run_capture("Print \"Hello, \" plus \"world.\".\n").unwrap();
    assert_lines(out, &["Hello, world."]);
}

#[test]
fn booleans_print() {
    let src = "
        Let yes be true.
        Let no be false.
        Print yes.
        Print no.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn booleans_output() {
    let out = run_capture("Print true.\nPrint false.\n").unwrap();
    assert_lines(out, &["true", "false"]);
}

#[test]
fn comparisons_print() {
    let src = "
        Let age be 21.
        Print age is equal to 21.
        Print age is not equal to 18.
        Print age is greater than 18.
        Print age is less than 30.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn logic_and_or() {
    let src = "
        Let age be 25.
        Let has_id be true.
        If age is greater than 18 and has_id, then
            Print \"Allowed\".
        End.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn if_then_block() {
    let src = "
        Let x be 10.
        If x is greater than 5, then
            Print \"Big\".
        End.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn if_otherwise_block() {
    let src = "
        Let score be 75.
        If score is greater than or equal to 60, then
            Print \"Pass\".
        Otherwise
            Print \"Fail\".
        End.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn if_otherwise_output() {
    let out = run_capture(
        "
        Let score be 75.
        If score is greater than or equal to 60, then
            Print \"Pass\".
        Otherwise
            Print \"Fail\".
        End.
    ",
    )
    .unwrap();
    assert_lines(out, &["Pass"]);
}

#[test]
fn repeat_block() {
    let src = "Repeat 3 times\n    Print \"hi\".\nEnd.\n";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn repeat_output() {
    let out = run_capture("Repeat 3 times\n    Print \"hi\".\nEnd.\n").unwrap();
    assert_lines(out, &["hi", "hi", "hi"]);
}

#[test]
fn while_block() {
    let src = "
        Let count be 0.
        While count is less than 3
            Print count.
            Set count to count plus 1.
        End.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn while_output() {
    let out = run_capture(
        "
        Let count be 0.
        While count is less than 3
            Print count.
            Set count to count plus 1.
        End.
    ",
    )
    .unwrap();
    assert_lines(out, &["0", "1", "2"]);
}

#[test]
fn function_no_return() {
    let src = "
        Define a function called greet that takes name
            Print name.
        End.
        Run greet with \"world\".
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn function_return() {
    let src = "
        Define a function called square that takes x and returns x multiplied by x.
        Print Run square with 5.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn function_return_output() {
    let out = run_capture(
        "
        Define a function called square that takes x and returns x multiplied by x.
        Print Run square with 5.
    ",
    )
    .unwrap();
    assert_lines(out, &["25"]);
}

#[test]
fn function_multi_param() {
    let src = "
        Define a function called add that takes a and b and returns a plus b.
        Print Run add with 3 and 4.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn list_create_and_get() {
    let src = "
        Make a list called scores.
        Add 10 to scores.
        Add 20 to scores.
        Add 30 to scores.
        Print Get the first item of scores.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn list_output() {
    let out = run_capture(
        "
        Make a list called scores.
        Add 10 to scores.
        Add 20 to scores.
        Add 30 to scores.
        Print Get the first item of scores.
        Print Get the second item of scores.
        Print Get the third item of scores.
    ",
    )
    .unwrap();
    assert_lines(out, &["10", "20", "30"]);
}

#[test]
fn list_set_item() {
    let src = "
        Make a list called scores.
        Add 10 to scores.
        Add 20 to scores.
        Add 30 to scores.
        Set the third item of scores to 99.
        Print Get the third item of scores.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn list_length_runs() {
    let src = "
        Make a list called xs.
        Add 1 to xs.
        Add 2 to xs.
        Add 3 to xs.
        Let n be the length of xs.
        Print n.
    ";
    let res = run(src);
    assert!(res.is_ok(), "the length of X should run: {:?}", res);
}

#[test]
fn list_length_output() {
    let out = run_capture(
        "
        Make a list called fruits.
        Add \"apple\" to fruits.
        Add \"banana\" to fruits.
        Add \"cherry\" to fruits.
        Print the length of fruits.

        Make a list called empty.
        Print the length of empty.
    ",
    )
    .unwrap();
    assert_lines(out, &["3", "0"]);
}

#[test]
fn list_length_grows_in_loop() {
    let out = run_capture(
        "
        Make a list called squares.
        Let n be 0.
        While n is less than 5
            Add n multiplied by n to squares.
            Set n to n plus 1.
        End.
        Print the length of squares.
    ",
    )
    .unwrap();
    assert_lines(out, &["5"]);
}

#[test]
fn module_import_all() {
    // Use a sub-directory with a small module file
    let dir = std::env::temp_dir().join("engling_tests_modules_all");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("helpers.eng"),
        "Define a function called double that takes x and returns x multiplied by 2.",
    )
    .unwrap();
    let src = "
        Import helpers.
        Print Run double with 5.
    ";
    let path = dir.join("main.eng");
    std::fs::write(&path, src).unwrap();
    let mut vm = VM::new();
    let res = runtime::execute_file(&path, &mut vm);
    assert!(res.is_ok(), "module import should succeed: {:?}", res);
}

#[test]
fn module_import_all_output() {
    let dir = std::env::temp_dir().join("engling_tests_modules_all_out");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("helpers.eng"),
        "Define a function called double that takes x and returns x multiplied by 2.",
    )
    .unwrap();
    let src = "
        Import helpers.
        Print Run double with 5.
    ";
    let path = dir.join("main.eng");
    std::fs::write(&path, src).unwrap();
    let out = run_file_capture(&path).unwrap();
    assert_lines(out, &["10"]);
}

#[test]
fn module_import_selective() {
    let dir = std::env::temp_dir().join("engling_tests_modules_sel");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("helpers.eng"),
        "Define a function called double that takes x and returns x multiplied by 2.",
    )
    .unwrap();
    let src = "
        From helpers use double.
        Print Run double with 7.
    ";
    let path = dir.join("main.eng");
    std::fs::write(&path, src).unwrap();
    let mut vm = VM::new();
    let res = runtime::execute_file(&path, &mut vm);
    assert!(res.is_ok(), "selective import should succeed: {:?}", res);
}

#[test]
fn nested_scope_runs() {
    let src = "
        Let x be 10.
        If x is greater than 0, then
            Let inner be 99.
            Print inner.
        End.
        Print x.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn nested_scope_output() {
    let out = run_capture(
        "
        Let x be 10.
        If x is greater than 0, then
            Let inner be 99.
            Print inner.
        End.
        Print x.
    ",
    )
    .unwrap();
    assert_lines(out, &["99", "10"]);
}

#[test]
fn fizzbuzz_runs() {
    let src = "
        Let i be 1.
        While i is less than or equal to 5
            If i modulo 3 is equal to 0, then
                Print \"Fizz\".
            Otherwise
                Print i.
            End.
            Set i to i plus 1.
        End.
    ";
    let res = run(src);
    assert!(res.is_ok());
}

#[test]
fn near_miss_errors_cleanly() {
    let src = "Let x be.";
    let mut vm = VM::new();
    let res = runtime::execute(src.to_string(), &mut vm);
    assert!(res.is_err(), "Expected parse error for `Let x be.`");
}

#[test]
fn unknown_word_offers_suggestion() {
    let src = "Pritn 5.";
    let mut vm = VM::new();
    let res = runtime::execute(src.to_string(), &mut vm);
    assert!(res.is_err(), "Expected error for typo `Pritn`");
}

#[test]
fn unknown_word_suggestion_is_print() {
    let src = "Pritn 5.";
    let mut vm = VM::new();
    let err = runtime::execute(src.to_string(), &mut vm).unwrap_err();
    let msg = format!("{err}");
    assert!(
        msg.contains("print"),
        "Expected suggestion to mention 'print', got: {msg}"
    );
}

#[test]
fn all_examples_directory_parses() {
    let examples = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples");
    let mut count = 0;
    for entry in std::fs::read_dir(&examples).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "eng") {
            // Skip module files and UI-required files (just check parse + run)
            let fname = path.file_name().unwrap().to_string_lossy().to_string();
            if fname.starts_with("24_") || fname.starts_with("25_") {
                continue; // UI feature
            }
            if fname.starts_with("27_") {
                continue; // reads from stdin; would block the test harness
            }
            if fname == "math_helpers.eng" {
                continue; // module file, loaded via import
            }
            if fname == "arithmetic.eng" || fname == "bgtest.eng" {
                continue; // legacy hand-written examples, replaced by 01-25
            }
            if fname == "23_error_near_miss.eng" {
                continue; // intentionally invalid; covered by near_miss_errors_cleanly
            }
            let mut vm = VM::new();
            let res = runtime::execute_file(&path, &mut vm);
            assert!(
                res.is_ok(),
                "example `{}` should run cleanly: {:?}",
                fname,
                res
            );
            count += 1;
        }
    }
    assert!(count >= 22, "Expected at least 22 non-UI examples to pass");
}

#[test]
fn repl_block_depth_tracker() {
    // The REPL should not submit a program until all opened blocks close.
    // We test the public helper that powers that logic.
    assert_eq!(
        engling::repl::block_depth("If x, then\n  Print 1.\nEnd."),
        0
    );
    assert_eq!(
        engling::repl::block_depth("Repeat 3 times\n  Print 1.\nEnd."),
        0
    );
    assert_eq!(engling::repl::block_depth("While x\n  Print 1.\nEnd."), 0);
    assert_eq!(
        engling::repl::block_depth("If x, then\n  Print 1.\nEnd.\nWhile y\n  Print 2.\nEnd."),
        0
    );
    assert!(engling::repl::block_depth("If x, then\n  Print 1.") > 0);
    assert!(engling::repl::block_depth("Repeat 3 times\n  Print 1.") > 0);
    assert!(engling::repl::block_depth("While x\n  Print 1.") > 0);
    // Comments and strings shouldn't move the counter.
    assert_eq!(
        engling::repl::block_depth("# if this is a comment\nPrint \"if then\"."),
        0
    );
}

/// Run a source string with a canned input reader that returns the next
/// entry of `inputs` for every `Ask` (with the trailing newline the
/// default `read_line` would leave attached), and capture what was printed.
fn run_with_input(source: &str, inputs: &[&str]) -> Result<Vec<String>, EnglingError> {
    let output: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let prompts: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let out_clone = Arc::clone(&output);
    let prompts_clone = Arc::clone(&prompts);
    let inputs: Vec<String> = inputs.iter().map(|s| s.to_string()).collect();
    let input_count = inputs.len();
    {
        let mut iter = inputs.into_iter();
        let printer = Box::new(move |v: &engling::value::Value| {
            out_clone.lock().unwrap().push(v.to_string());
        });
        let reader = Box::new(move |prompt: &str| -> engling::error::Result<String> {
            prompts_clone.lock().unwrap().push(prompt.to_string());
            Ok(iter.next().unwrap_or_default() + "\n")
        });
        let mut vm = VM::with_printer_and_input(printer, reader);
        runtime::execute(source.to_string(), &mut vm)?;
    }
    let captured_prompts = Arc::try_unwrap(prompts)
        .ok()
        .expect("prompts Arc captured by more than one reference")
        .into_inner()
        .unwrap();
    assert_eq!(
        captured_prompts.len(),
        input_count,
        "expected one captured prompt per supplied input line"
    );
    match Arc::try_unwrap(output) {
        Ok(mutex) => Ok(mutex.into_inner().unwrap()),
        Err(_) => panic!("print callback captured multiple Arc references"),
    }
}

#[test]
fn ask_stores_into_variable() {
    let src = "
        Ask \"Name: \" and put it in name.
        Print \"Hi, \" plus name plus \".\".
    ";
    let out = run_with_input(src, &["Ada"]).unwrap();
    assert_lines(out, &["Hi, Ada."]);
}

#[test]
fn ask_strips_trailing_newline() {
    // run_with_input always appends a newline (mimicking read_line). The
    // stored value should not retain it.
    let src = "
        Ask \"x: \" and put it in x.
        Let y be x plus \"!\".
        Print y.
    ";
    let out = run_with_input(src, &["hello"]).unwrap();
    assert_lines(out, &["hello!"]);
}

#[test]
fn ask_passes_prompt_to_reader() {
    let output: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let prompts: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let out_clone = Arc::clone(&output);
    let prompts_clone = Arc::clone(&prompts);
    {
        let printer = Box::new(move |v: &engling::value::Value| {
            out_clone.lock().unwrap().push(v.to_string());
        });
        let reader = Box::new(move |prompt: &str| -> engling::error::Result<String> {
            prompts_clone.lock().unwrap().push(prompt.to_string());
            Ok(String::from("ok\n"))
        });
        let mut vm = VM::with_printer_and_input(printer, reader);
        runtime::execute(
            "Ask \"Username: \" and put it in name.\nPrint name.\n".to_string(),
            &mut vm,
        )
        .unwrap();
    }
    let captured = prompts.lock().unwrap().clone();
    assert_eq!(captured, vec!["Username: ".to_string()]);
    assert_lines(
        Arc::try_unwrap(output).unwrap().into_inner().unwrap(),
        &["ok"],
    );
}

#[test]
fn ask_with_numeric_arithmetic() {
    // Demonstrates that a number entered via Ask is treated as a string;
    // numeric parsing happens at the user level. Here we just verify the
    // stored value participates in concatenation.
    let src = "
        Ask \"Age: \" and put it in age.
        Print \"You said \" plus age plus \".\".
    ";
    let out = run_with_input(src, &["21"]).unwrap();
    assert_lines(out, &["You said 21."]);
}

#[test]
fn ask_uses_example_file() {
    // Run the shipped example to make sure it parses and runs end-to-end
    // with canned stdin input.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("27_input.eng");
    let output: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let out_clone = Arc::clone(&output);
    let inputs = vec![String::from("Ada\n"), String::from("blue\n")];
    {
        let mut iter = inputs.into_iter();
        let printer = Box::new(move |v: &engling::value::Value| {
            out_clone.lock().unwrap().push(v.to_string());
        });
        let reader = Box::new(move |_prompt: &str| -> engling::error::Result<String> {
            Ok(iter.next().unwrap_or_default())
        });
        let mut vm = VM::with_printer_and_input(printer, reader);
        runtime::execute_file(&path, &mut vm).unwrap();
    }
    let captured = Arc::try_unwrap(output).unwrap().into_inner().unwrap();
    assert_lines(captured, &["Hello, Ada!", "Ada likes blue."]);
}
