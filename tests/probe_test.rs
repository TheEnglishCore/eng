use engling::lexer::Lexer;
use engling::parser::Parser;

#[test]
fn trace_if() {
    let src = "If score is greater than or equal to 60, then\n  Print \"Pass\".\nOtherwise\n  Print \"Fail\".\nEnd.";
    let toks = Lexer::new(src.to_string()).tokenize().unwrap();
    let mut p = Parser::with_source(toks, src.to_string());
    let _ = p.parse();
}