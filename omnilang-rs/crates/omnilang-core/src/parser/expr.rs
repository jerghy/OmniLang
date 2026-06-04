use super::*;

/// Pratt parser for OmniLang expressions.
/// Binding powers (bp) determine operator precedence.
/// Higher bp = tighter binding.
///
/// Precedence levels (from lowest to highest):
///   10  ||    (logical or)
///   20  &&    (logical and)
///   30  == != < > <= >=   (comparisons)
///   50  + -   (additive)
///   60  * / % (multiplicative)
///   80  prefix - ! await  (unary)
///   90  () [] . ?.        (postfix)

impl<'s> Parser<'s> {
    /// Entry point for expression parsing
    pub fn parse_expr(&mut self) -> Option<Expr> {
        self.parse_expr_bp(0)
    }

    /// Parse expression with a minimum binding power
    pub fn parse_expr_bp(&mut self, min_bp: u8) -> Option<Expr> {
        let mut lhs = self.parse_prefix()?;

        loop {
            let _op = match self.current_kind() {
                TokenKind::QuesQues => {
                    // Null coalesce: lowest precedence
                    if min_bp > 5 { break; }
                    self.advance();
                    let rhs = self.parse_expr_bp(6)?;
                    lhs = Expr::NullCoalesce(NullCoalesceNode {
                        node_id: new_node_id(),
                        span: lhs.span().clone(),
                        left: Box::new(lhs),
                        right: Box::new(rhs),
                    });
                    continue;
                }
                TokenKind::OrOr => {
                    let (lbp, rbp) = (10, 11);
                    if lbp < min_bp { break; }
                    self.advance();
                    let rhs = self.parse_expr_bp(rbp)?;
                    lhs = Expr::BinaryOp(BinaryOpNode {
                        node_id: new_node_id(),
                        span: lhs.span().clone(),
                        left: Box::new(lhs),
                        op: BinaryOpKind::Or,
                        right: Box::new(rhs),
                    });
                    continue;
                }
                TokenKind::AndAnd => {
                    let (lbp, rbp) = (20, 21);
                    if lbp < min_bp { break; }
                    self.advance();
                    let rhs = self.parse_expr_bp(rbp)?;
                    lhs = Expr::BinaryOp(BinaryOpNode {
                        node_id: new_node_id(),
                        span: lhs.span().clone(),
                        left: Box::new(lhs),
                        op: BinaryOpKind::And,
                        right: Box::new(rhs),
                    });
                    continue;
                }
                TokenKind::EqEq | TokenKind::Ne
                | TokenKind::Lt | TokenKind::Gt
                | TokenKind::Le | TokenKind::Ge => {
                    let (lbp, rbp) = (30, 31);
                    if lbp < min_bp { break; }
                    let op_kind = match self.current_kind() {
                        TokenKind::EqEq => BinaryOpKind::Eq,
                        TokenKind::Ne => BinaryOpKind::Ne,
                        TokenKind::Lt => BinaryOpKind::Lt,
                        TokenKind::Gt => BinaryOpKind::Gt,
                        TokenKind::Le => BinaryOpKind::Le,
                        TokenKind::Ge => BinaryOpKind::Ge,
                        _ => unreachable!(),
                    };
                    self.advance();
                    let rhs = self.parse_expr_bp(rbp)?;
                    lhs = Expr::BinaryOp(BinaryOpNode {
                        node_id: new_node_id(),
                        span: lhs.span().clone(),
                        left: Box::new(lhs),
                        op: op_kind,
                        right: Box::new(rhs),
                    });
                    continue;
                }
                TokenKind::Plus => {
                    let (lbp, rbp) = (50, 51);
                    if lbp < min_bp { break; }
                    self.advance();
                    let rhs = self.parse_expr_bp(rbp)?;
                    lhs = Expr::BinaryOp(BinaryOpNode {
                        node_id: new_node_id(),
                        span: lhs.span().clone(),
                        left: Box::new(lhs),
                        op: BinaryOpKind::Add,
                        right: Box::new(rhs),
                    });
                    continue;
                }
                TokenKind::Minus => {
                    let (lbp, rbp) = (50, 51);
                    if lbp < min_bp { break; }
                    self.advance();
                    let rhs = self.parse_expr_bp(rbp)?;
                    lhs = Expr::BinaryOp(BinaryOpNode {
                        node_id: new_node_id(),
                        span: lhs.span().clone(),
                        left: Box::new(lhs),
                        op: BinaryOpKind::Sub,
                        right: Box::new(rhs),
                    });
                    continue;
                }
                TokenKind::Star => {
                    let (lbp, rbp) = (60, 61);
                    if lbp < min_bp { break; }
                    self.advance();
                    let rhs = self.parse_expr_bp(rbp)?;
                    lhs = Expr::BinaryOp(BinaryOpNode {
                        node_id: new_node_id(),
                        span: lhs.span().clone(),
                        left: Box::new(lhs),
                        op: BinaryOpKind::Mul,
                        right: Box::new(rhs),
                    });
                    continue;
                }
                TokenKind::Slash => {
                    let (lbp, rbp) = (60, 61);
                    if lbp < min_bp { break; }
                    self.advance();
                    let rhs = self.parse_expr_bp(rbp)?;
                    lhs = Expr::BinaryOp(BinaryOpNode {
                        node_id: new_node_id(),
                        span: lhs.span().clone(),
                        left: Box::new(lhs),
                        op: BinaryOpKind::Div,
                        right: Box::new(rhs),
                    });
                    continue;
                }
                TokenKind::Percent => {
                    let (lbp, rbp) = (60, 61);
                    if lbp < min_bp { break; }
                    self.advance();
                    let rhs = self.parse_expr_bp(rbp)?;
                    lhs = Expr::BinaryOp(BinaryOpNode {
                        node_id: new_node_id(),
                        span: lhs.span().clone(),
                        left: Box::new(lhs),
                        op: BinaryOpKind::Mod,
                        right: Box::new(rhs),
                    });
                    continue;
                }
                // Postfix operators: handled in parse_postfix after prefix parsing
                _ => break,
            };
        }

        Some(lhs)
    }

    /// Parse a prefix expression (atom or prefix operator)
    fn parse_prefix(&mut self) -> Option<Expr> {
        let expr = self.parse_atom()?;
        // Apply postfix operators (.method(), [index], (args), ?.field)
        self.parse_postfix(expr)
    }

    /// Parse an atomic expression without postfix handling
    fn parse_atom(&mut self) -> Option<Expr> {
        match self.current_kind() {
            // Literals
            TokenKind::Number(num) => {
                let start = self.current_span();
                let num = num.clone();
                self.advance();
                let (value, kind) = parse_number(&num);
                Some(Expr::Literal(LiteralNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    value,
                    kind: kind.to_string(),
                }))
            }
            TokenKind::StringLit(s) => {
                let start = self.current_span();
                let s = s.clone();
                self.advance();
                Some(Expr::Literal(LiteralNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    value: LiteralValue::Str(s),
                    kind: "String".to_string(),
                }))
            }
            TokenKind::TemplateString(raw) => {
                let start = self.current_span();
                let raw = raw.clone();
                self.advance();
                self.parse_template_string(&raw, &start)
            }
            TokenKind::True => {
                let start = self.current_span();
                self.advance();
                Some(Expr::Literal(LiteralNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    value: LiteralValue::Bool(true),
                    kind: "bool".to_string(),
                }))
            }
            TokenKind::False => {
                let start = self.current_span();
                self.advance();
                Some(Expr::Literal(LiteralNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    value: LiteralValue::Bool(false),
                    kind: "bool".to_string(),
                }))
            }

            // Unary prefix operators
            TokenKind::Minus => {
                let start = self.current_span();
                self.advance();
                let operand = self.parse_expr_bp(80)?;
                Some(Expr::UnaryOp(UnaryOpNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    op: UnaryOpKind::Neg,
                    operand: Box::new(operand),
                }))
            }
            TokenKind::Bang => {
                let start = self.current_span();
                self.advance();
                let operand = self.parse_expr_bp(80)?;
                Some(Expr::UnaryOp(UnaryOpNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    op: UnaryOpKind::Not,
                    operand: Box::new(operand),
                }))
            }
            TokenKind::Await => {
                let start = self.current_span();
                self.advance();
                let expr = self.parse_expr_bp(75)?;
                Some(Expr::AwaitExpr(AwaitExprNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    expr: Box::new(expr),
                }))
            }

            // Spread
            TokenKind::DotDotDot => {
                let start = self.current_span();
                self.advance();
                let expr = self.parse_expr()?;
                Some(Expr::SpreadExpr(SpreadExprNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    expr: Box::new(expr),
                }))
            }

            // Grouping
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.expect(TokenKind::RParen).ok()?;
                // Check for arrow function: (params) => expr/block
                if self.check(&TokenKind::FatArrow) {
                    self.parse_arrow_function(expr)
                } else {
                    Some(expr)
                }
            }

            // Array literal
            TokenKind::LBracket => {
                let start = self.current_span();
                self.advance();
                let mut elements = vec![];
                while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                    elements.push(self.parse_expr()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::RBracket).ok()?;
                Some(Expr::ArrayLiteral(ArrayLiteralNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    elements,
                }))
            }

            // Map literal or block? In expression context, { is always a map literal
            // (blocks are statement-level)
            TokenKind::LBrace => {
                let start = self.current_span();
                self.advance();

                // Check for empty map
                if self.match_token(&TokenKind::RBrace) {
                    return Some(Expr::MapLiteral(MapLiteralNode {
                        node_id: new_node_id(),
                        span: self.span_from(&start),
                        entries: vec![],
                    }));
                }

                // Check for map literal: key: value or string key without colon
                let first = self.parse_expr()?;

                if self.match_token(&TokenKind::Colon) {
                    // Map literal {key: value, ...}
                    let mut entries = vec![(first, self.parse_expr()?)];
                    while self.match_token(&TokenKind::Comma) {
                        if self.check(&TokenKind::RBrace) {
                            break;
                        }
                        let k = self.parse_expr()?;
                        self.expect(TokenKind::Colon).ok()?;
                        let v = self.parse_expr()?;
                        entries.push((k, v));
                    }
                    self.expect(TokenKind::RBrace).ok()?;
                    Some(Expr::MapLiteral(MapLiteralNode {
                        node_id: new_node_id(),
                        span: self.span_from(&start),
                        entries,
                    }))
                } else {
                    self.error("Expected ':' for map entry or '}' for empty map");
                    // Try to recover
                    while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                        self.advance();
                    }
                    self.expect(TokenKind::RBrace).ok();
                    None
                }
            }

            // Closure: |params| body
            TokenKind::Pipe => {
                self.parse_closure_expr()
            }

            // Empty closure: || body
            TokenKind::OrOr => {
                // Treat || at prefix position as empty closure
                self.advance(); // consume the OrOr token
                // || means no params, so just parse the body
                let return_type = if self.match_token(&TokenKind::Arrow) {
                    self.parse_type()
                } else {
                    None
                };
                let body = if self.check(&TokenKind::LBrace) {
                    ClosureBody::Block(self.parse_block()?)
                } else {
                    ClosureBody::Expr(self.parse_expr()?)
                };
                Some(Expr::ClosureExpr(ClosureExprNode {
                    node_id: new_node_id(),
                    span: Span::dummy(),
                    params: vec![],
                    return_type,
                    body: Box::new(body),
                    is_arrow: false,
                }))
            }

            // If expression
            TokenKind::If => {
                let start = self.current_span();
                self.advance();
                let condition = self.parse_expr()?;
                let then_expr = self.parse_block_or_expr()?;
                self.expect(TokenKind::Else).ok()?;
                let else_expr = if self.check(&TokenKind::If) {
                    // Nested if expression
                    self.parse_expr()?
                } else {
                    self.parse_block_or_expr()?
                };
                Some(Expr::IfExpr(IfExprNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    condition: Box::new(condition),
                    then_expr: Box::new(then_expr),
                    else_expr: Box::new(else_expr),
                }))
            }

            // Match expression
            TokenKind::Match => {
                self.parse_match_expr()
            }

            // Identifiers / Function calls / Struct literals
            TokenKind::Ident(name) => {
                let start = self.current_span();
                let name = name.clone();
                self.advance();

                // Check for struct literal: Name { field: value, ... }
                if self.check(&TokenKind::LBrace)
                    && !self.check_peek(&TokenKind::Colon)
                    && !self.check_peek(&TokenKind::RBrace)
                {
                    // This could be a struct literal, but only if the name starts with uppercase
                    if name.chars().next().map_or(false, |c| c.is_uppercase()) {
                        return self.parse_struct_literal_body(&name, &start);
                    }
                }

                Some(Expr::VarRef(VarRefNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    name,
                }))
            }

            _ => {
                self.error(&format!(
                    "Expected expression, got {:?}",
                    self.current_kind()
                ));
                None
            }
        }
    }

    /// Parse postfix operations: .method(), [index], (args), ?.field
    pub fn parse_postfix(&mut self, mut expr: Expr) -> Option<Expr> {
        loop {
            match self.current_kind() {
                TokenKind::LParen => {
                    // Function call
                    let start = expr.span().clone();
                    self.advance();
                    let mut args = vec![];
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        args.push(self.parse_expr()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(TokenKind::RParen).ok()?;
                    expr = Expr::FuncCall(FuncCallNode {
                        node_id: new_node_id(),
                        span: Span::new(
                            start.file.clone(),
                            start.line,
                            start.col,
                            self.tokens[self.pos.saturating_sub(1)].span.end_line,
                            self.tokens[self.pos.saturating_sub(1)].span.end_col,
                        ),
                        func: Box::new(expr),
                        args,
                    });
                }
                TokenKind::Dot => {
                    self.advance();
                    let method = self.take_name()?;

                    if self.check(&TokenKind::LParen) {
                        // Method call: obj.method(args)
                        let start = expr.span().clone();
                        self.advance();
                        let mut args = vec![];
                        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                            args.push(self.parse_expr()?);
                            if !self.match_token(&TokenKind::Comma) {
                                break;
                            }
                        }
                        self.expect(TokenKind::RParen).ok()?;
                        expr = Expr::MethodCall(MethodCallNode {
                            node_id: new_node_id(),
                            span: start,
                            object: Box::new(expr),
                            method,
                            args,
                        });
                    } else {
                        // Field access? Not in OmniLang v0.2, but we handle it anyway
                        // Actually, this could be a struct field access like p.x
                        self.error(&format!(
                            "Expected '(' after method name '{}', or field access not supported",
                            method
                        ));
                    }
                }
                TokenKind::LBracket => {
                    // Index: expr[index]
                    self.advance();
                    let index = self.parse_expr()?;
                    self.expect(TokenKind::RBracket).ok()?;
                    expr = Expr::IndexGet(IndexGetNode {
                        node_id: new_node_id(),
                        span: expr.span().clone(),
                        object: Box::new(expr),
                        index: Box::new(index),
                    });
                }
                TokenKind::Question => {
                    // Optional chain: ?.field or ?[index]
                    // Actually, the lexer emits ? and . separately
                    // We need to check for ?. sequence
                    if self.peek_kind() == &TokenKind::Dot {
                        self.advance(); // consume ?
                        self.advance(); // consume .
                        let name = self.take_name()?;
                        let mut chain = vec![ChainLink {
                            kind: "field".to_string(),
                            name: Some(name),
                            index: None,
                            args: vec![],
                        }];

                        // Continue the optional chain
                        while self.match_token(&TokenKind::Question) {
                            if self.match_token(&TokenKind::Dot) {
                                let n = self.take_name()?;
                                chain.push(ChainLink {
                                    kind: "field".to_string(),
                                    name: Some(n),
                                    index: None,
                                    args: vec![],
                                });
                            } else if self.match_token(&TokenKind::LBracket) {
                                let idx = self.parse_expr()?;
                                self.expect(TokenKind::RBracket).ok()?;
                                chain.push(ChainLink {
                                    kind: "index".to_string(),
                                    name: None,
                                    index: Some(Box::new(idx)),
                                    args: vec![],
                                });
                            } else {
                                break;
                            }
                        }

                        expr = Expr::OptionalChain(OptionalChainNode {
                            node_id: new_node_id(),
                            span: expr.span().clone(),
                            object: Box::new(expr),
                            chain,
                        });
                    } else {
                        // Just ? — nullable type sugar, not used in expressions
                        break;
                    }
                }
                _ => break,
            }
        }
        Some(expr)
    }

    /// Parse a struct literal body: already consumed Name, now parse { field: value, ... }
    pub fn parse_struct_literal_body(&mut self, type_name: &str, start: &Span) -> Option<Expr> {
        self.expect(TokenKind::LBrace).ok()?;
        let mut fields = vec![];
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            if self.match_token(&TokenKind::DotDotDot) {
                // Spread: ...expr
                let expr = self.parse_expr()?;
                fields.push(("...".to_string(), expr));
            } else {
                let field_name = self.take_name()?;
                self.expect(TokenKind::Colon).ok()?;
                let value = self.parse_expr()?;
                fields.push((field_name, value));
            }
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RBrace).ok()?;

        Some(Expr::StructLiteral(StructLiteralNode {
            node_id: new_node_id(),
            span: self.span_from(start),
            type_name: type_name.to_string(),
            fields,
        }))
    }

    /// Parse a closure expression: |params| -> Type { body } or |params| expr
    fn parse_closure_expr(&mut self) -> Option<Expr> {
        let start = self.current_span();
        self.expect(TokenKind::Pipe).ok()?;

        let mut params = vec![];
        while !self.check(&TokenKind::Pipe) && !self.check(&TokenKind::Eof) {
            let pname = self.take_name()?;
            let type_ann = if self.check(&TokenKind::Colon) {
                // Use parse_base_type directly, NOT parse_type() —
                // parse_type() consumes `|` as union type operator,
                // but `|` is the closure's closing delimiter.
                self.expect(TokenKind::Colon).ok()?;
                let ty = self.parse_base_type()?;
                let ty = if self.match_token(&TokenKind::Question) {
                    TypeRepr::union(vec![ty, TypeRepr::named("null")])
                } else {
                    ty
                };
                Some(ty)
            } else {
                None
            };
            params.push(ParamNode {
                node_id: new_node_id(),
                span: Span::dummy(),
                name: pname,
                type_ann,
                is_rest: false,
            });
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::Pipe).ok()?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            self.parse_type()
        } else {
            None
        };

        let body = if self.check(&TokenKind::LBrace) {
            ClosureBody::Block(self.parse_block()?)
        } else {
            ClosureBody::Expr(self.parse_expr()?)
        };

        Some(Expr::ClosureExpr(ClosureExprNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            params,
            return_type,
            body: Box::new(body),
            is_arrow: false,
        }))
    }

    /// Parse an arrow function: (params) => expr or (params) => { block }
    fn parse_arrow_function(&mut self, _first: Expr) -> Option<Expr> {
        // At this point we've consumed `(` then parsed `first` as expr, then found `)`
        // But wait — we already consumed the closing paren in parse_prefix.
        // We actually need to rewind. Let's handle this differently.
        // Arrow functions: (a: i32, b: i32) => a + b
        // Since we already consumed the content between parens via parse_expr,
        // we need a different approach. Let's keep it simple for now.
        self.expect(TokenKind::FatArrow).ok()?;

        let body = if self.check(&TokenKind::LBrace) {
            ClosureBody::Block(self.parse_block()?)
        } else {
            ClosureBody::Expr(self.parse_expr()?)
        };

        // Extract param names from the first expression
        // This is simplified — full arrow function param parsing needs more work
        let params = vec![];

        Some(Expr::ClosureExpr(ClosureExprNode {
            node_id: new_node_id(),
            span: Span::dummy(),
            params,
            return_type: None,
            body: Box::new(body),
            is_arrow: true,
        }))
    }

    /// Parse block_or_expr: { block } or just expression
    fn parse_block_or_expr(&mut self) -> Option<Expr> {
        if self.check(&TokenKind::LBrace) {
            let block = self.parse_block()?;
            // Wrap block as... hmm, blocks aren't expressions in the current AST.
            // For if expression branches, we just use the block's trailing expr.
            if let Some(trailing) = block.trailing_expr {
                Some(*trailing)
            } else {
                // No trailing expr — this would be a void branch
                // Return a placeholder literal
                Some(Expr::Literal(LiteralNode {
                    node_id: new_node_id(),
                    span: block.span,
                    value: LiteralValue::Bool(false),
                    kind: "bool".to_string(),
                }))
            }
        } else {
            self.parse_expr()
        }
    }

    /// Parse match expression
    pub fn parse_match_expr(&mut self) -> Option<Expr> {
        let start = self.current_span();
        self.expect(TokenKind::Match).ok()?;
        let scrutinee = self.parse_expr()?;
        self.expect(TokenKind::LBrace).ok()?;

        let mut arms = vec![];
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let arm_start = self.current_span();
            let pattern = self.parse_match_pattern()?;
            let guard = if self.match_token(&TokenKind::If) {
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            self.expect(TokenKind::FatArrow).ok()?;
            let body = if self.check(&TokenKind::LBrace) {
                MatchArmBody::Block(self.parse_block()?)
            } else {
                MatchArmBody::Expr(self.parse_expr()?)
            };
            arms.push(MatchArmNode {
                node_id: new_node_id(),
                span: self.span_from(&arm_start),
                pattern,
                guard,
                body: Box::new(body),
            });
            // Match arms are comma-separated
            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(TokenKind::RBrace).ok()?;

        Some(Expr::MatchExpr(MatchExprNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            scrutinee: Box::new(scrutinee),
            arms,
        }))
    }

    /// Parse a match pattern
    fn parse_match_pattern(&mut self) -> Option<MatchPattern> {
        match self.current_kind() {
            TokenKind::Underscore => {
                self.advance();
                Some(MatchPattern::Wildcard)
            }
            TokenKind::True => {
                self.advance();
                Some(MatchPattern::Literal(LiteralValue::Bool(true)))
            }
            TokenKind::False => {
                self.advance();
                Some(MatchPattern::Literal(LiteralValue::Bool(false)))
            }
            TokenKind::Number(num) => {
                let (val, _kind) = parse_number(num);
                let _num = num.clone();
                self.advance();
                Some(MatchPattern::Literal(val))
            }
            TokenKind::StringLit(s) => {
                let s = s.clone();
                self.advance();
                Some(MatchPattern::Literal(LiteralValue::Str(s)))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();

                // Check for struct pattern: Name { field: pat, ... }
                if self.check(&TokenKind::LBrace) {
                    self.advance();
                    let mut fields = vec![];
                    while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                        let fname = self.take_name()?;
                        let pat = if self.match_token(&TokenKind::Colon) {
                            self.parse_match_pattern()?
                        } else {
                            MatchPattern::Var(fname.clone())
                        };
                        fields.push((fname, pat));
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    self.expect(TokenKind::RBrace).ok()?;
                    Some(MatchPattern::Struct(StructPatternNode { name, fields }))
                } else {
                    Some(MatchPattern::Var(name))
                }
            }
            _ => {
                self.error(&format!(
                    "Expected match pattern, got {:?}",
                    self.current_kind()
                ));
                None
            }
        }
    }
}

/// Parse a number string into a LiteralValue and kind string.
pub fn parse_number(s: &str) -> (LiteralValue, &'static str) {
    let s = s.replace('_', "");

    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        if let Ok(v) = i32::from_str_radix(hex, 16) {
            return (LiteralValue::Int(v), "i32");
        }
    }
    if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        if let Ok(v) = i32::from_str_radix(oct, 8) {
            return (LiteralValue::Int(v), "i32");
        }
    }
    if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        if let Ok(v) = i32::from_str_radix(bin, 2) {
            return (LiteralValue::Int(v), "i32");
        }
    }

    // Float
    if s.contains('.') || s.contains('e') || s.contains('E') {
        if let Ok(v) = s.parse::<f64>() {
            return (LiteralValue::Float(v), "f64");
        }
    }

    // Integer
    if let Ok(v) = s.parse::<i32>() {
        return (LiteralValue::Int(v), "i32");
    }

    // Fallback
    if let Ok(v) = s.parse::<f64>() {
        (LiteralValue::Float(v), "f64")
    } else {
        (LiteralValue::Int(0), "i32")
    }
}
