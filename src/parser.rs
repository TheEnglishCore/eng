use crate::ast::*;
use crate::error::{EnglingError, Result, line_col_to_offset};
use crate::token::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    /// Original source, used to attach miette spans to errors.
    source: String,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
            source: String::new(),
        }
    }

    pub fn with_source(tokens: Vec<Token>, source: String) -> Self {
        Self {
            tokens,
            current: 0,
            source,
        }
    }

    /// Helper: convert a (line, column) token position into a miette span.
    fn span_at(&self, line: usize, column: usize, byte_len: usize) -> (usize, usize) {
        let offset = line_col_to_offset(&self.source, line, column);
        (offset, byte_len.max(1))
    }

    fn span_for_token(&self, tok: &Token) -> (usize, usize) {
        (tok.byte_offset, tok.byte_len.max(1))
    }

    fn err_at(&self, tok: &Token, message: impl Into<String>) -> EnglingError {
        let (off, len) = self.span_for_token(tok);
        EnglingError::parse_with_span(
            tok.line,
            tok.column,
            message,
            self.source.clone(),
            off,
            len,
        )
    }

    fn err_pos(&self, line: usize, column: usize, message: impl Into<String>) -> EnglingError {
        let (off, len) = self.span_at(line, column, 1);
        EnglingError::parse_with_span(line, column, message, self.source.clone(), off, len)
    }

    pub fn parse(&mut self) -> Result<Program> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.statement()?);
        }
        Ok(Program { statements })
    }

    fn statement(&mut self) -> Result<Statement> {
        match &self.peek().kind {
            TokenKind::Let => self.variable_decl(),
            TokenKind::Set => self.assignment(),
            TokenKind::Print => self.print_stmt(),
            TokenKind::If => self.if_stmt(),
            TokenKind::Repeat => self.repeat_stmt(),
            TokenKind::While => self.while_stmt(),
            TokenKind::Define => self.function_def(),
            TokenKind::Run | TokenKind::Call => self.run_stmt(),
            TokenKind::Make => self.make_stmt(),
            TokenKind::Add => self.list_add(),
            TokenKind::Import => self.import_stmt(),
            TokenKind::From => self.import_from(),
            TokenKind::Create => self.module_decl(),
            #[cfg(feature = "ui")]
            TokenKind::When => self.event_handler(),
            kind => Err(self.err_pos(
                self.peek().line,
                self.peek().column,
                format!("Unexpected statement starting with {kind:?}"),
            )),
        }
    }

    fn make_stmt(&mut self) -> Result<Statement> {
        self.advance(); // Make
        if self.check(&TokenKind::A) {
            self.advance();
        }
        match &self.peek().kind {
            TokenKind::List => {
                self.advance();
                self.expect(TokenKind::Called)?;
                let name = self.read_identifier()?;
                self.expect(TokenKind::Period)?;
                Ok(Statement::ListDecl { name })
            }
            #[cfg(feature = "ui")]
            TokenKind::Window => {
                self.advance();
                self.expect(TokenKind::Called)?;
                let name = self.read_identifier()?;
                self.expect(TokenKind::Titled)?;
                let title = self.read_string_lit()?;
                self.expect(TokenKind::Period)?;
                Ok(Statement::WindowDecl { name, title })
            }
            _ => {
                // Make x be expr. (synonym for Let)
                let name = self.read_identifier()?;
                self.expect(TokenKind::Be)?;
                let value = self.expression()?;
                self.expect(TokenKind::Period)?;
                Ok(Statement::Variable { name, value })
            }
        }
    }

    #[cfg(feature = "ui")]
    fn make_ui_widget(&mut self) -> Result<Statement> {
        unreachable!()
    }

    fn variable_decl(&mut self) -> Result<Statement> {
        self.advance();
        let name = self.read_identifier()?;
        self.expect(TokenKind::Be)?;
        let value = self.expression()?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::Variable { name, value })
    }

    fn assignment(&mut self) -> Result<Statement> {
        self.advance();
        // Set the label text of count_label to expr.
        if self.check(&TokenKind::The) {
            self.advance();
            if self.check(&TokenKind::Label) || self.check(&TokenKind::Text) {
                #[cfg(feature = "ui")]
                {
                    self.advance(); // label/text
                    if self.check(&TokenKind::Text) {
                        self.advance(); // text in "label text"
                    }
                    self.expect(TokenKind::Of)?;
                    // Allow either an identifier or a string literal as the label name
                    let label_name = if let TokenKind::String(s) = &self.peek().kind {
                        let s = s.clone();
                        self.advance();
                        s
                    } else {
                        self.read_identifier()?
                    };
                    self.expect(TokenKind::To)?;
                    let value = self.expression()?;
                    self.expect(TokenKind::Period)?;
                    return Ok(Statement::SetLabelText { label_name, value });
                }
            }
            // Set the Nth item of list to expr.
            if self.check_ordinal() {
                let index = self.read_ordinal()?;
                self.expect(TokenKind::Item)?;
                self.expect(TokenKind::Of)?;
                let name = self.read_identifier()?;
                self.expect(TokenKind::To)?;
                let value = self.expression()?;
                self.expect(TokenKind::Period)?;
                return Ok(Statement::ListSet { name, index, value });
            }
        }
        let name = self.read_identifier()?;
        self.expect(TokenKind::To)?;
        let value = self.expression()?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::Assignment { name, value })
    }

    fn print_stmt(&mut self) -> Result<Statement> {
        self.advance();
        let expression = self.expression()?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::Print { expression })
    }

    fn if_stmt(&mut self) -> Result<Statement> {
        self.advance();
        let condition = self.expression()?;
        self.expect(TokenKind::Comma)?;
        self.expect(TokenKind::Then)?;
        let then_block = self.block_until_otherwise_or_end()?;
        let else_block = if self.check(&TokenKind::Otherwise) {
            self.advance();
            Some(self.block_until_end()?)
        } else {
            None
        };
        self.expect(TokenKind::End)?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::If {
            condition,
            then_block,
            else_block,
        })
    }

    fn repeat_stmt(&mut self) -> Result<Statement> {
        self.advance();
        let count = self.expression()?;
        self.expect(TokenKind::Times)?;
        let body = self.block_until_end()?;
        self.expect(TokenKind::End)?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::Repeat { count, body })
    }

    fn while_stmt(&mut self) -> Result<Statement> {
        self.advance();
        let condition = self.expression()?;
        let body = self.block_until_end()?;
        self.expect(TokenKind::End)?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::While { condition, body })
    }

    fn function_def(&mut self) -> Result<Statement> {
        self.advance(); // Define
        if self.check(&TokenKind::A) {
            self.advance();
        }
        self.expect(TokenKind::Function)?;
        self.expect(TokenKind::Called)?;
        let name = self.read_identifier()?;
        // Optional "that" between `called X` and `takes`.
        if self.check(&TokenKind::That) {
            self.advance();
        }
        self.expect(TokenKind::Takes)?;
        let params = self.read_param_list()?;

        if self.check(&TokenKind::Returns) {
            self.advance();
            let return_expr = self.expression()?;
            self.expect(TokenKind::Period)?;
            return Ok(Statement::FunctionDef {
                name,
                params,
                body: None,
                return_expr: Some(return_expr),
            });
        }

        // body block ending with End.
        let body = self.block_until_end()?;
        self.expect(TokenKind::End)?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::FunctionDef {
            name,
            params,
            body: Some(body),
            return_expr: None,
        })
    }

    fn run_stmt(&mut self) -> Result<Statement> {
        self.advance();
        let name = self.read_identifier()?;
        let args = if self.check(&TokenKind::With) {
            self.advance();
            self.read_arg_list()?
        } else {
            Vec::new()
        };
        self.expect(TokenKind::Period)?;
        Ok(Statement::Run { name, args })
    }

    fn list_add(&mut self) -> Result<Statement> {
        self.advance();
        let value = self.expression()?;
        self.expect(TokenKind::To)?;
        let name = self.read_identifier()?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::ListAdd { name, value })
    }

    fn import_stmt(&mut self) -> Result<Statement> {
        self.advance();
        let module = self.read_identifier()?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::Import { module })
    }

    fn import_from(&mut self) -> Result<Statement> {
        self.advance();
        let module = self.read_identifier()?;
        self.expect(TokenKind::Use)?;
        let names = self.read_name_list()?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::ImportFrom { module, names })
    }

    fn module_decl(&mut self) -> Result<Statement> {
        self.advance();
        if self.check(&TokenKind::A) {
            self.advance();
        }
        self.expect(TokenKind::Module)?;
        self.expect(TokenKind::Called)?;
        let name = self.read_identifier()?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::ModuleDecl { name })
    }

    #[cfg(feature = "ui")]
    fn event_handler(&mut self) -> Result<Statement> {
        self.advance(); // When
        self.expect(TokenKind::The)?;
        let button_label = self.read_string_lit()?;
        self.expect(TokenKind::Button)?;
        self.expect(TokenKind::Is)?;
        self.expect(TokenKind::Clicked)?;
        self.expect(TokenKind::Comma)?;
        self.expect(TokenKind::Run)?;
        let function = self.read_identifier()?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::EventHandler {
            button_label,
            function,
        })
    }

    fn block_until_otherwise_or_end(&mut self) -> Result<Vec<Statement>> {
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::Otherwise)
            && !self.check(&TokenKind::End)
            && !self.is_at_end()
        {
            stmts.push(self.statement_with_ui()?);
        }
        Ok(stmts)
    }

    fn block_until_end(&mut self) -> Result<Vec<Statement>> {
        let mut stmts = Vec::new();
        while !self.check(&TokenKind::End) && !self.is_at_end() {
            stmts.push(self.statement_with_ui()?);
        }
        Ok(stmts)
    }

    fn read_param_list(&mut self) -> Result<Vec<String>> {
        if self.check(&TokenKind::Nothing) {
            self.advance();
            return Ok(Vec::new());
        }
        let mut params = vec![self.read_identifier()?];
        // Continue reading params as long as the next separator is `and` or
        // `,` followed by an identifier. Stop (and consume the separator)
        // when the next token is `returns` (the start of a return-expression)
        // or a body terminator — `and` here is part of the bridge
        // `takes x and returns y` and must be eaten so the caller sees
        // `returns` next.
        while self.check(&TokenKind::And) || self.check(&TokenKind::Comma) {
            // Peek ahead: if the next token after `and`/`and` is `returns`
            // or a body terminator, consume the separator and stop.
            let next = self.current + 1;
            if next < self.tokens.len() {
                let next_kind = &self.tokens[next].kind;
                if matches!(next_kind, TokenKind::Returns | TokenKind::End) {
                    self.advance(); // consume the `and` / `,`
                    break;
                }
            }
            self.advance(); // and / ,
            params.push(self.read_identifier()?);
        }
        Ok(params)
    }

    fn read_arg_list(&mut self) -> Result<Vec<Expression>> {
        let mut args = Vec::new();
        loop {
            if self.check(&TokenKind::Period) || self.is_at_end() {
                break;
            }
            // "with name being expr" or just expr
            if let TokenKind::Identifier(ref param) = self.peek().kind {
                let param_name = param.clone();
                let next = self.current + 1;
                if next < self.tokens.len() {
                    if matches!(self.tokens[next].kind, TokenKind::Be | TokenKind::To) {
                        self.advance(); // param name
                        self.advance(); // be/to
                        args.push(self.expression()?);
                        continue;
                    }
                }
                let _ = param_name;
            }
            // Use `comparison` rather than `expression` so that `and` /
            // `or` are treated as argument separators (`Run f with 3 and
            // 4` -> args = [3, 4]) rather than as boolean operators.
            args.push(self.comparison()?);
            if self.check(&TokenKind::And) || self.check(&TokenKind::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(args)
    }

    fn read_name_list(&mut self) -> Result<Vec<String>> {
        let mut names = vec![self.read_identifier()?];
        while self.check(&TokenKind::Comma) || self.check(&TokenKind::And) {
            self.advance();
            names.push(self.read_identifier()?);
        }
        Ok(names)
    }

    fn expression(&mut self) -> Result<Expression> {
        self.logic_or()
    }

    fn logic_or(&mut self) -> Result<Expression> {
        let mut expr = self.logic_and()?;
        while self.check(&TokenKind::Or) {
            self.advance();
            let right = self.logic_and()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator: Operator::Or,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn logic_and(&mut self) -> Result<Expression> {
        let mut expr = self.comparison()?;
        while self.check(&TokenKind::And) {
            self.advance();
            let right = self.comparison()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator: Operator::And,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expression> {
        let mut expr = self.addition()?;
        loop {
            if !self.check(&TokenKind::Is) {
                break;
            }
            self.advance();
            let operator = match self.peek().kind {
                TokenKind::Equal => {
                    self.advance();
                    self.expect(TokenKind::To)?;
                    Operator::Equal
                }
                TokenKind::Not => {
                    self.advance();
                    self.expect(TokenKind::Equal)?;
                    self.expect(TokenKind::To)?;
                    Operator::NotEqual
                }
                TokenKind::Greater => {
                    self.advance();
                    if self.check(&TokenKind::Than) {
                        self.advance();
                        if self.check(&TokenKind::Or) {
                            self.advance();
                            self.expect(TokenKind::Equal)?;
                            self.expect(TokenKind::To)?;
                            Operator::GreaterEqual
                        } else {
                            Operator::Greater
                        }
                    } else {
                        return Err(self.err_pos(
                            self.peek().line,
                            self.peek().column,
                            "Expected 'than' after 'greater'",
                        ));
                    }
                }
                TokenKind::Less => {
                    self.advance();
                    if self.check(&TokenKind::Than) {
                        self.advance();
                        if self.check(&TokenKind::Or) {
                            self.advance();
                            self.expect(TokenKind::Equal)?;
                            self.expect(TokenKind::To)?;
                            Operator::LessEqual
                        } else {
                            Operator::Less
                        }
                    } else {
                        return Err(self.err_pos(
                            self.peek().line,
                            self.peek().column,
                            "Expected 'than' after 'less'",
                        ));
                    }
                }
                _ => {
                    return Err(self.err_pos(
                        self.peek().line,
                        self.peek().column,
                        "Expected comparison after 'is'",
                    ));
                }
            };
            let right = self.addition()?;
            expr = Expression::Binary {
                left: Box::new(expr),
                operator,
                right: Box::new(right),
            };
        }
        Ok(expr)
    }

    fn addition(&mut self) -> Result<Expression> {
        let mut expr = self.multiplication()?;
        loop {
            match self.peek().kind {
                TokenKind::Plus => {
                    self.advance();
                    let right = self.multiplication()?;
                    expr = Expression::Binary {
                        left: Box::new(expr),
                        operator: Operator::Add,
                        right: Box::new(right),
                    };
                }
                TokenKind::Minus => {
                    self.advance();
                    let right = self.multiplication()?;
                    expr = Expression::Binary {
                        left: Box::new(expr),
                        operator: Operator::Subtract,
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn multiplication(&mut self) -> Result<Expression> {
        let mut expr = self.unary()?;
        loop {
            match self.peek().kind {
                TokenKind::Multiplied => {
                    self.advance();
                    self.expect(TokenKind::By)?;
                    let right = self.unary()?;
                    expr = Expression::Binary {
                        left: Box::new(expr),
                        operator: Operator::Multiply,
                        right: Box::new(right),
                    };
                }
                TokenKind::Divided => {
                    self.advance();
                    self.expect(TokenKind::By)?;
                    let right = self.unary()?;
                    expr = Expression::Binary {
                        left: Box::new(expr),
                        operator: Operator::Divide,
                        right: Box::new(right),
                    };
                }
                TokenKind::Modulo => {
                    self.advance();
                    let right = self.unary()?;
                    expr = Expression::Binary {
                        left: Box::new(expr),
                        operator: Operator::Modulo,
                        right: Box::new(right),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expression> {
        if self.check(&TokenKind::Get) {
            return self.list_get_expr();
        }
        if self.check(&TokenKind::The) {
            // Peek ahead to see if this is `the length of IDENT`.
            let next = self.current + 1;
            if next < self.tokens.len()
                && matches!(self.tokens[next].kind, TokenKind::Length)
            {
                return self.list_length_expr();
            }
        }
        self.primary()
    }

    fn list_length_expr(&mut self) -> Result<Expression> {
        self.advance(); // The
        self.expect(TokenKind::Length)?;
        self.expect(TokenKind::Of)?;
        let name = self.read_identifier()?;
        Ok(Expression::ListLength { name })
    }

    fn list_get_expr(&mut self) -> Result<Expression> {
        self.advance(); // Get
        self.expect(TokenKind::The)?;
        let index = self.read_ordinal()?;
        self.expect(TokenKind::Item)?;
        self.expect(TokenKind::Of)?;
        let name = self.read_identifier()?;
        Ok(Expression::ListGet { name, index })
    }

    fn primary(&mut self) -> Result<Expression> {
        // Run func as expression (for return value)
        if self.check(&TokenKind::Run) || self.check(&TokenKind::Call) {
            self.advance();
            let name = self.read_identifier()?;
            let args = if self.check(&TokenKind::With) {
                self.advance();
                self.read_arg_list()?
            } else {
                Vec::new()
            };
            return Ok(Expression::Call { name, args });
        }

        match self.advance().kind {
            TokenKind::Number(n) => Ok(Expression::Number(n)),
            TokenKind::String(s) => Ok(Expression::String(s)),
            TokenKind::True => Ok(Expression::Boolean(true)),
            TokenKind::False => Ok(Expression::Boolean(false)),
            TokenKind::Identifier(name) => Ok(Expression::Variable(name)),
            // Same contextual-keyword fallbacks as `read_identifier`: a
            // short English word that the lexer flagged as a keyword but
            // that the parser is happy to treat as a variable reference in
            // an expression.
            TokenKind::A => Ok(Expression::Variable("a".to_string())),
            TokenKind::Add => Ok(Expression::Variable("add".to_string())),
            TokenKind::By => Ok(Expression::Variable("by".to_string())),
            TokenKind::To => Ok(Expression::Variable("to".to_string())),
            TokenKind::Of => Ok(Expression::Variable("of".to_string())),
            TokenKind::Or => Ok(Expression::Variable("or".to_string())),
            TokenKind::And => Ok(Expression::Variable("and".to_string())),
            TokenKind::Is => Ok(Expression::Variable("is".to_string())),
            TokenKind::Be => Ok(Expression::Variable("be".to_string())),
            kind => Err(self.err_at(self.previous(), format!("Invalid expression: {kind:?}"))),
        }
    }

    fn check_ordinal(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::First
                | TokenKind::Second
                | TokenKind::Third
                | TokenKind::Fourth
                | TokenKind::Fifth
                | TokenKind::Number(_)
        )
    }

    fn read_ordinal(&mut self) -> Result<usize> {
        let index = match self.advance().kind {
            TokenKind::First => 0,
            TokenKind::Second => 1,
            TokenKind::Third => 2,
            TokenKind::Fourth => 3,
            TokenKind::Fifth => 4,
            TokenKind::Number(n) => {
                let n = n as usize;
                if self.check(&TokenKind::St)
                    || self.check(&TokenKind::Nd)
                    || self.check(&TokenKind::Rd)
                    || self.check(&TokenKind::Th)
                {
                    self.advance();
                }
                if n == 0 {
                    return Err(self.err_at(
                        self.previous(),
                        "List indices start at 1, not 0",
                    ));
                }
                n - 1
            }
            kind => {
                return Err(self.err_pos(
                    self.peek().line,
                    self.peek().column,
                    format!("Expected ordinal, got {kind:?}"),
                ));
            }
        };
        Ok(index)
    }

    fn read_identifier(&mut self) -> Result<String> {
        let tok = self.advance();
        let name = match tok.kind {
            TokenKind::Identifier(name) => name,
            // Some short English words are reserved as keywords (`a`, `add`,
            // ...) but are also perfectly natural identifier names. When
            // they appear in an identifier position, treat them as
            // identifiers with the corresponding word.
            TokenKind::A => "a".to_string(),
            TokenKind::Add => "add".to_string(),
            TokenKind::By => "by".to_string(),
            TokenKind::To => "to".to_string(),
            TokenKind::Of => "of".to_string(),
            TokenKind::Or => "or".to_string(),
            TokenKind::And => "and".to_string(),
            TokenKind::Is => "is".to_string(),
            TokenKind::Be => "be".to_string(),
            kind => {
                return Err(self.err_at(
                    self.previous(),
                    format!("Expected identifier, got {kind:?}"),
                ))
            }
        };
        Ok(name)
    }

    fn read_string_lit(&mut self) -> Result<String> {
        match self.advance().kind {
            TokenKind::String(s) => Ok(s),
            kind => Err(self.err_at(self.previous(), format!("Expected string, got {kind:?}"))),
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<()> {
        if self.check(&kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.err_pos(
                self.peek().line,
                self.peek().column,
                format!("Expected {kind:?}, found {:?}", self.peek().kind),
            ))
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn advance(&mut self) -> Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.tokens[self.current - 1].clone()
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::EOF
    }
}

// Handle UI widget declarations via Make path - need to add to make_stmt
impl Parser {
    #[cfg(feature = "ui")]
    pub fn parse_widget_after_make(&mut self) -> Result<Statement> {
        // A has already been consumed by statement_with_ui
        match &self.peek().kind {
            TokenKind::Button | TokenKind::Label | TokenKind::Text => {}
            _ => {
                return Err(self.err_pos(
                    self.peek().line,
                    self.peek().column,
                    "Expected button, label, or text field",
                ));
            }
        }
        let kind = match self.advance().kind {
            TokenKind::Button => WidgetKind::Button,
            TokenKind::Label => WidgetKind::Label,
            TokenKind::Text => {
                self.expect(TokenKind::Field)?;
                WidgetKind::TextField
            }
            _ => unreachable!(),
        };
        self.expect(TokenKind::To)?;
        let window = self.read_identifier()?;
        self.expect(TokenKind::Labeled)?;
        let label = self.read_string_lit()?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::WidgetDecl {
            window,
            kind,
            label,
        })
    }
}

// Patch make_stmt to handle Add a button to window
impl Parser {
    pub fn statement_with_ui(&mut self) -> Result<Statement> {
        if self.check(&TokenKind::Make) {
            let saved = self.current;
            self.advance();
            if self.check(&TokenKind::A) {
                self.advance();
                #[cfg(feature = "ui")]
                if matches!(
                    self.peek().kind,
                    TokenKind::Button | TokenKind::Label | TokenKind::Text
                ) {
                    return self.parse_widget_after_make_from_peek();
                }
            }
            self.current = saved;
        }
        if self.check(&TokenKind::Add) {
            let saved = self.current;
            self.advance();
            if self.check(&TokenKind::A) {
                self.advance();
                #[cfg(feature = "ui")]
                if matches!(
                    self.peek().kind,
                    TokenKind::Button | TokenKind::Label | TokenKind::Text
                ) {
                    return self.parse_widget_after_add();
                }
            }
            self.current = saved;
        }
        self.statement()
    }

    #[cfg(feature = "ui")]
    fn parse_widget_after_make_from_peek(&mut self) -> Result<Statement> {
        self.parse_widget_after_make()
    }

    #[cfg(feature = "ui")]
    fn parse_widget_after_add(&mut self) -> Result<Statement> {
        let kind = match self.advance().kind {
            TokenKind::Button => WidgetKind::Button,
            TokenKind::Label => WidgetKind::Label,
            TokenKind::Text => {
                self.expect(TokenKind::Field)?;
                WidgetKind::TextField
            }
            kind => {
                return Err(self.err_at(self.previous(), format!("Expected widget type, got {kind:?}")));
            }
        };
        self.expect(TokenKind::To)?;
        let window = self.read_identifier()?;
        self.expect(TokenKind::Labeled)?;
        let label = self.read_string_lit()?;
        self.expect(TokenKind::Period)?;
        Ok(Statement::WidgetDecl {
            window,
            kind,
            label,
        })
    }
}

// Override parse to use statement_with_ui
impl Parser {
    pub fn parse_program(&mut self) -> Result<Program> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.statement_with_ui()?);
        }
        Ok(Program { statements })
    }
}
