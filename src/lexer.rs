use crate::error::{lex_error_at, suggest_keyword, EnglingError, Result};
use crate::token::{Token, TokenKind};

pub struct Lexer {
    source: String,
    chars: Vec<char>,
    /// Char-based position in `chars`.
    position: usize,
    line: usize,
    column: usize,
    /// Byte offset of `position` (where the next char would start).
    byte_offset: usize,
}

impl Lexer {
    pub fn new(source: String) -> Self {
        let chars: Vec<char> = source.chars().collect();
        Self {
            source,
            chars,
            position: 0,
            line: 1,
            column: 1,
            byte_offset: 0,
        }
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token()?;
            let end = token.kind == TokenKind::EOF;
            tokens.push(token);
            if end {
                break;
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace();
        let line = self.line;
        let column = self.column;
        let byte_offset = self.byte_offset;

        if self.position >= self.chars.len() {
            return Ok(Token::with_span(
                TokenKind::EOF,
                line,
                column,
                byte_offset,
                0,
            ));
        }

        let ch = self.advance();

        match ch {
            '.' => Ok(Token::with_span(
                TokenKind::Period,
                line,
                column,
                byte_offset,
                1,
            )),
            ',' => Ok(Token::with_span(
                TokenKind::Comma,
                line,
                column,
                byte_offset,
                1,
            )),
            '"' => {
                let start_byte = self.byte_offset;
                let value = self.read_string()?;
                let len = self.byte_offset - start_byte - 1; // exclude trailing quote
                Ok(Token::with_span(
                    TokenKind::String(value),
                    line,
                    column,
                    start_byte,
                    len.max(1),
                ))
            }
            '#' => {
                self.skip_comment();
                self.next_token()
            }
            c if c.is_ascii_digit() => self.read_number(c, line, column, byte_offset),
            c if c.is_alphabetic() || c == '_' => self.read_word(c, line, column, byte_offset),
            _ => self.next_token(),
        }
    }

    fn read_word(
        &mut self,
        first: char,
        line: usize,
        column: usize,
        byte_offset: usize,
    ) -> Result<Token> {
        let mut word = first.to_string();
        let start_byte = byte_offset;
        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                word.push(self.advance());
            } else {
                break;
            }
        }
        let len = self.byte_offset - start_byte;

        let lower = word.to_lowercase();
        let kind = match lower.as_str() {
            "let" => TokenKind::Let,
            "set" => TokenKind::Set,
            "make" => TokenKind::Make,
            "be" => TokenKind::Be,
            "to" => TokenKind::To,
            "print" | "show" | "display" => TokenKind::Print,
            "ask" => TokenKind::Ask,
            "put" => TokenKind::Put,
            "it" => TokenKind::It,
            "in" => TokenKind::In,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "if" => TokenKind::If,
            "otherwise" => TokenKind::Otherwise,
            "end" => TokenKind::End,
            "repeat" => TokenKind::Repeat,
            "times" => TokenKind::Times,
            "while" => TokenKind::While,
            "then" => TokenKind::Then,
            "define" => TokenKind::Define,
            "function" => TokenKind::Function,
            "called" => TokenKind::Called,
            "that" => TokenKind::That,
            "takes" => TokenKind::Takes,
            "returns" => TokenKind::Returns,
            "run" | "call" => TokenKind::Run,
            "with" => TokenKind::With,
            "a" | "an" => TokenKind::A,
            "nothing" => TokenKind::Nothing,
            "list" => TokenKind::List,
            "add" => TokenKind::Add,
            "get" => TokenKind::Get,
            "the" => TokenKind::The,
            "item" => TokenKind::Item,
            "of" => TokenKind::Of,
            "length" => TokenKind::Length,
            "first" => TokenKind::First,
            "second" => TokenKind::Second,
            "third" => TokenKind::Third,
            "fourth" => TokenKind::Fourth,
            "fifth" => TokenKind::Fifth,
            "st" => TokenKind::St,
            "nd" => TokenKind::Nd,
            "rd" => TokenKind::Rd,
            "th" => TokenKind::Th,
            "import" => TokenKind::Import,
            "from" => TokenKind::From,
            "use" => TokenKind::Use,
            "module" => TokenKind::Module,
            "create" => TokenKind::Create,
            "window" => TokenKind::Window,
            "titled" => TokenKind::Titled,
            "button" => TokenKind::Button,
            "label" => TokenKind::Label,
            "text" => TokenKind::Text,
            "field" => TokenKind::Field,
            "when" => TokenKind::When,
            "clicked" => TokenKind::Clicked,
            "labeled" => TokenKind::Labeled,
            "plus" => TokenKind::Plus,
            "minus" => TokenKind::Minus,
            "multiplied" => TokenKind::Multiplied,
            "divided" => TokenKind::Divided,
            "by" => TokenKind::By,
            "modulo" => TokenKind::Modulo,
            "is" => TokenKind::Is,
            "equal" => TokenKind::Equal,
            "not" => TokenKind::Not,
            "greater" => TokenKind::Greater,
            "less" => TokenKind::Less,
            "than" => TokenKind::Than,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            _ => {
                if let Some(suggestion) = suggest_keyword(&word) {
                    return Err(lex_error_at(
                        &self.source,
                        line,
                        column,
                        &word,
                        format!("Unknown word '{word}'. Did you mean '{suggestion}'?"),
                    ));
                }
                TokenKind::Identifier(word)
            }
        };

        Ok(Token::with_span(kind, line, column, start_byte, len))
    }

    fn read_number(
        &mut self,
        first: char,
        line: usize,
        column: usize,
        byte_offset: usize,
    ) -> Result<Token> {
        let mut number = first.to_string();
        let mut is_float = false;
        let start_byte = byte_offset;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                number.push(self.advance());
            } else if c == '.' && !is_float {
                if let Some(next) = self.chars.get(self.position + 1) {
                    if next.is_ascii_digit() {
                        is_float = true;
                        number.push(self.advance());
                        continue;
                    }
                }
                break;
            } else {
                break;
            }
        }
        let len = self.byte_offset - start_byte;

        let value: f64 = number.parse().map_err(|_| {
            EnglingError::lex(
                line,
                column,
                format!("Invalid number '{number}'"),
                self.source.clone(),
                start_byte,
                len.max(1),
            )
        })?;

        Ok(Token::with_span(
            TokenKind::Number(value),
            line,
            column,
            start_byte,
            len,
        ))
    }

    fn read_string(&mut self) -> Result<String> {
        let mut result = String::new();
        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance();
                break;
            }
            if c == '\n' {
                return Err(EnglingError::lex(
                    self.line,
                    self.column,
                    "Unterminated string",
                    self.source.clone(),
                    self.byte_offset,
                    1,
                ));
            }
            result.push(self.advance());
        }
        Ok(result)
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        while let Some(c) = self.peek() {
            self.advance();
            if c == '\n' {
                break;
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.position).copied()
    }

    fn advance(&mut self) -> char {
        let c = self.chars[self.position];
        let c_len = c.len_utf8();
        self.position += 1;
        self.byte_offset += c_len;
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        c
    }
}
