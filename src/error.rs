use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Where a positioned error occurred. Pairs a span in the source with a
/// copy of the source text so miette can render a labeled snippet.
#[derive(Debug, Clone)]
pub struct SourceLocation {
    pub source: String,
    pub span: SourceSpan,
}

#[derive(Debug, Clone, Error, Diagnostic)]
pub enum EnglingError {
    #[error("Lexer error at line {line}, column {column}: {message}")]
    #[diagnostic(code(engling::lex), help("Check the spelling of this word."))]
    Lex {
        line: usize,
        column: usize,
        message: String,
        #[source_code]
        src: String,
        #[label("here")]
        span: SourceSpan,
    },

    #[error("Parse error at line {line}, column {column}: {message}")]
    #[diagnostic(
        code(engling::parse),
        help("Review the sentence template in docs/GRAMMAR.md.")
    )]
    Parse {
        line: usize,
        column: usize,
        message: String,
        #[source_code]
        src: String,
        #[label("here")]
        span: SourceSpan,
    },

    #[error("Runtime error: {0}")]
    #[diagnostic(code(engling::runtime))]
    Runtime(String),

    #[error("Module error: {0}")]
    #[diagnostic(code(engling::module))]
    Module(String),

    #[error("Package error: {0}")]
    #[diagnostic(code(engling::package))]
    Package(String),
}

pub type Result<T> = std::result::Result<T, EnglingError>;

impl EnglingError {
    /// Construct a parse error attached to a specific (line, column) in the
    /// given source. The span defaults to a 1-character label at that
    /// position; callers can refine it via [`EnglingError::parse_with_span`].
    pub fn parse(line: usize, column: usize, message: impl Into<String>) -> Self {
        Self::parse_with_span(line, column, message, "", 0, 0)
    }

    pub fn parse_with_span(
        line: usize,
        column: usize,
        message: impl Into<String>,
        source: impl Into<String>,
        byte_offset: usize,
        byte_len: usize,
    ) -> Self {
        EnglingError::Parse {
            line,
            column,
            message: message.into(),
            src: source.into(),
            span: SourceSpan::new(byte_offset.into(), byte_len.max(1)),
        }
    }

    /// Construct a lex error with a source span pointing at the offending
    /// word. `byte_offset` is the 0-based byte offset into `source`,
    /// `byte_len` is the length of the offending token.
    pub fn lex(
        line: usize,
        column: usize,
        message: impl Into<String>,
        source: impl Into<String>,
        byte_offset: usize,
        byte_len: usize,
    ) -> Self {
        EnglingError::Lex {
            line,
            column,
            message: message.into(),
            src: source.into(),
            span: SourceSpan::new(byte_offset.into(), byte_len.max(1)),
        }
    }

    pub fn runtime(message: impl Into<String>) -> Self {
        EnglingError::Runtime(message.into())
    }

    pub fn module(message: impl Into<String>) -> Self {
        EnglingError::Module(message.into())
    }

    pub fn package(message: impl Into<String>) -> Self {
        EnglingError::Package(message.into())
    }
}

pub fn suggest_keyword(word: &str) -> Option<&'static str> {
    const KEYWORDS: &[&str] = &[
        "let",
        "set",
        "make",
        "be",
        "to",
        "print",
        "true",
        "false",
        "if",
        "otherwise",
        "end",
        "repeat",
        "times",
        "while",
        "then",
        "define",
        "function",
        "called",
        "that",
        "takes",
        "returns",
        "run",
        "with",
        "plus",
        "minus",
        "and",
        "or",
        "import",
        "from",
        "use",
        "add",
        "get",
        "the",
        "item",
        "of",
        "length",
        "first",
        "second",
        "third",
    ];
    let w = word.to_lowercase();
    // Only suggest when the word is at least 4 chars long. This prevents
    // short identifiers like `x` from being mis-flagged as typos for `be`.
    if w.len() < 4 {
        return None;
    }
    KEYWORDS
        .iter()
        .find(|k| {
            if k.len() < 4 {
                return false;
            }
            let d = levenshtein(&w, k);
            if d > 2 {
                return false;
            }
            // Don't suggest a keyword that is much shorter than the word.
            if k.len() + 1 < w.len() {
                return false;
            }
            // Require a shared prefix of at least 2 characters so that
            // words like `name` (which happen to be 2 edits from `make`)
            // don't get false-positive suggestions.
            prefix_match(&w, k) >= 2
        })
        .copied()
}

fn prefix_match(a: &str, b: &str) -> usize {
    let mut count = 0;
    for (ca, cb) in a.chars().zip(b.chars()) {
        if ca == cb {
            count += 1;
        } else {
            break;
        }
    }
    count
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut dp = vec![vec![0; b.len() + 1]; a.len() + 1];
    for (i, row) in dp.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, val) in dp[0].iter_mut().enumerate().skip(1) {
        *val = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            dp[i][j] = (dp[i - 1][j] + 1)
                .min(dp[i][j - 1] + 1)
                .min(dp[i - 1][j - 1] + cost);
        }
    }
    dp[a.len()][b.len()]
}

pub fn report(err: &EnglingError) {
    let report = miette::Report::new(err.clone());
    eprintln!("{report:?}");
}

/// Convert a (line, column) pair (1-based) into a byte offset within `source`.
pub fn line_col_to_offset(source: &str, line: usize, column: usize) -> usize {
    let mut byte_offset = 0usize;
    let mut current_line = 1usize;
    for ch in source.chars() {
        if current_line == line {
            // Count columns: column 1 is the first character on the line.
            // We need to land at byte offset of the (column-1)th char.
            let mut col = 1usize;
            for c in source[byte_offset..].chars() {
                if col == column {
                    return byte_offset;
                }
                byte_offset += c.len_utf8();
                if c == '\n' {
                    current_line += 1;
                    if current_line == line + 1 {
                        // Overshot — caller passed column past EOL.
                        return byte_offset.saturating_sub(c.len_utf8());
                    }
                }
                col += 1;
            }
            return byte_offset;
        }
        if ch == '\n' {
            current_line += 1;
        }
        byte_offset += ch.len_utf8();
    }
    byte_offset
}

/// Helper for tests: returns the byte length of the label that should be
/// drawn for a positioned error. Useful when callers want to default the
/// label length to "until end of word".
pub fn word_at(source: &str, byte_offset: usize) -> usize {
    let bytes = source.as_bytes();
    let mut end = byte_offset;
    while end < bytes.len() {
        let b = bytes[end];
        if b.is_ascii_alphanumeric() || b == b'_' {
            end += 1;
        } else {
            break;
        }
    }
    end.saturating_sub(byte_offset).max(1)
}

/// Convenience: render a positioned error with source context attached.
pub fn parse_error_at(
    source: &str,
    line: usize,
    column: usize,
    message: impl Into<String>,
) -> EnglingError {
    let offset = line_col_to_offset(source, line, column);
    let len = word_at(source, offset);
    EnglingError::parse_with_span(line, column, message, source.to_string(), offset, len)
}

pub fn lex_error_at(
    source: &str,
    line: usize,
    column: usize,
    word: &str,
    message: impl Into<String>,
) -> EnglingError {
    let offset = line_col_to_offset(source, line, column);
    let len = word.len().max(1);
    EnglingError::lex(line, column, message, source.to_string(), offset, len)
}

// (Intentionally no dead-code stubs — keep the file lean.)
