use super::*;

impl<'s> Parser<'s> {
    /// Parse a template string from its raw content (without backticks).
    /// Template strings contain literal text and ${expr} interpolations.
    pub fn parse_template_string(&mut self, raw: &str, span: &Span) -> Option<Expr> {
        let mut parts = vec![];
        let mut i = 0;
        let chars: Vec<char> = raw.chars().collect();
        let mut literal = String::new();

        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                // Escape sequence
                i += 1;
                match chars[i] {
                    'n' => literal.push('\n'),
                    't' => literal.push('\t'),
                    '\\' => literal.push('\\'),
                    '$' => literal.push('$'),
                    '`' => literal.push('`'),
                    c => {
                        literal.push('\\');
                        literal.push(c);
                    }
                }
                i += 1;
            } else if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                // Interpolation: ${expr}
                if !literal.is_empty() {
                    parts.push(TemplatePart {
                        kind: "literal".to_string(),
                        value: TemplatePartValue::Literal(std::mem::take(&mut literal)),
                    });
                }
                i += 2; // skip ${
                let mut depth = 1;
                let mut expr_text = String::new();
                while i < chars.len() && depth > 0 {
                    match chars[i] {
                        '{' => {
                            depth += 1;
                            expr_text.push('{');
                        }
                        '}' => {
                            depth -= 1;
                            if depth > 0 {
                                expr_text.push('}');
                            }
                        }
                        c => expr_text.push(c),
                    }
                    i += 1;
                }
                // Parse the expression
                let expr = self.parse_inline_expr(&expr_text)?;
                parts.push(TemplatePart {
                    kind: "expr".to_string(),
                    value: TemplatePartValue::Expr(Box::new(expr)),
                });
            } else {
                literal.push(chars[i]);
                i += 1;
            }
        }

        if !literal.is_empty() {
            parts.push(TemplatePart {
                kind: "literal".to_string(),
                value: TemplatePartValue::Literal(literal),
            });
        }

        Some(Expr::TemplateString(TemplateStringNode {
            node_id: new_node_id(),
            span: span.clone(),
            parts,
        }))
    }

    /// Parse an inline expression (e.g., inside ${} in template strings).
    /// Creates a separate sub-lexer and sub-parser for the expression text.
    pub fn parse_inline_expr(&mut self, text: &str) -> Option<Expr> {
        let mut lexer = Lexer::new(text, &self.filename);
        let tokens = match lexer.tokenize() {
            Ok(t) => t,
            Err(lex_errors) => {
                for e in lex_errors {
                    self.error(&e);
                }
                return None;
            }
        };

        // Create a completely separate sub-parser with its own error collector.
        let mut sub_errors = ErrorCollector::new();
        let mut sub_parser = Parser::new(text, tokens, &self.filename, &mut sub_errors);
        let expr = sub_parser.parse_expr();

        // Transfer any sub-errors to the parent parser's error collector
        if sub_errors.has_errors() {
            for e in sub_errors.errors {
                self.errors.add(e.message, e.span, e.filename);
            }
        }

        expr
    }
}
