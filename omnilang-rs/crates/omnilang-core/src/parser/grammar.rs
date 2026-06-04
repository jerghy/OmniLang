use super::*;

impl<'s> Parser<'s> {
    /// Parse the entire program.
    pub fn parse_program(&mut self) -> Program {
        let mut program = Program::new();

        while !self.check(&TokenKind::Eof) {
            // Skip stray semicolons
            if self.match_token(&TokenKind::Semicolon) {
                continue;
            }

            match self.current_kind() {
                // Top-level declarations
                TokenKind::Fn | TokenKind::Async => {
                    if let Some(func) = self.parse_function_def() {
                        program.functions.push(func);
                    }
                }
                TokenKind::Struct | TokenKind::Nominal => {
                    if let Some(s) = self.parse_struct_def() {
                        program.structs.push(s);
                    }
                }
                TokenKind::Type => {
                    if let Some(tu) = self.parse_tagged_union_def() {
                        program.tagged_unions.push(tu);
                    }
                }
                TokenKind::Class => {
                    if let Some(c) = self.parse_class_def() {
                        program.classes.push(c);
                    }
                }
                TokenKind::Interface => {
                    if let Some(i) = self.parse_interface_def() {
                        program.interfaces.push(i);
                    }
                }
                TokenKind::Impl => {
                    if let Some(imp) = self.parse_impl_block() {
                        program.impl_blocks.push(imp);
                    }
                }
                TokenKind::Import => {
                    if let Some(imp) = self.parse_import_stmt(false) {
                        program.imports.push(imp);
                    }
                }
                TokenKind::Export => {
                    if let Some(exp) = self.parse_export_stmt() {
                        program.exports.push(exp);
                    }
                }
                TokenKind::Print | TokenKind::Println => {
                    // Allow print at top level (fixture convenience)
                    self.parse_print_stmt();
                }
                TokenKind::At => {
                    // attribute (decorator, suppress)
                    self.parse_attribute();
                }
                _ => {
                    self.error(&format!(
                        "Unexpected token at top level: {:?}",
                        self.current_kind()
                    ));
                    self.advance();
                }
            }
        }

        program.attach_impl_methods();
        program
    }

    // ── Function Definition ─────────────────────────────

    pub fn parse_function_def(&mut self) -> Option<FunctionDefNode> {
        let start = self.current_span();
        let is_async = self.match_token(&TokenKind::Async);

        if !self.match_token(&TokenKind::Fn) {
            if is_async {
                self.error("Expected 'fn' after 'async'");
            }
            return None;
        }

        let name = match self.take_name() {
            Some(n) => n,
            None => {
                self.error("Expected function name after 'fn'");
                return None;
            }
        };

        let generic_params = self.parse_generic_params();

        self.expect(TokenKind::LParen).ok()?;
        let params = self.parse_params();
        self.expect(TokenKind::RParen).ok()?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            self.parse_type()
        } else {
            None
        };

        let body = if self.check(&TokenKind::LBrace) {
            Some(self.parse_block()?)
        } else if self.match_token(&TokenKind::Semicolon) {
            // Forward declaration (interface style)
            None
        } else {
            self.error("Expected '{' for function body");
            return None;
        };

        Some(FunctionDefNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            name,
            params,
            return_type,
            generic_params,
            body,
            is_async,
        })
    }

    // ── Struct Definition ───────────────────────────────

    pub fn parse_struct_def(&mut self) -> Option<StructDefNode> {
        let start = self.current_span();
        let is_nominal = self.match_token(&TokenKind::Nominal);

        if !self.match_token(&TokenKind::Struct) {
            return None;
        }

        let name = self.take_ident()?;
        let generic_params = self.parse_generic_params();

        // Check for newtype syntax: nominal struct Name(Type);
        if is_nominal
            && self.match_token(&TokenKind::LParen)
        {
            let inner_type = self.parse_type()?;
            self.expect(TokenKind::RParen).ok()?;
            self.expect(TokenKind::Semicolon).ok();

            let field = StructFieldNode {
                node_id: new_node_id(),
                span: Span::dummy(),
                name: "0".to_string(),
                type_ann: inner_type,
                default: None,
            };
            return Some(StructDefNode {
                node_id: new_node_id(),
                span: self.span_from(&start),
                name,
                generic_params,
                fields: vec![field],
                methods: vec![],
                is_nominal: true,
            });
        }

        self.expect(TokenKind::LBrace).ok()?;

        let mut fields = vec![];
        let mut methods = vec![];

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            if self.check(&TokenKind::Fn) {
                if let Some(func) = self.parse_function_def() {
                    methods.push(func);
                }
            } else {
                let field_start = self.current_span();
                let field_name = self.take_ident()?;
                let type_ann = self.parse_type_annotation()?;
                let default = if self.match_token(&TokenKind::Eq) {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                // Allow either ; or , as field separator
                if !self.match_token(&TokenKind::Semicolon) {
                    self.match_token(&TokenKind::Comma);
                }

                fields.push(StructFieldNode {
                    node_id: new_node_id(),
                    span: self.span_from(&field_start),
                    name: field_name,
                    type_ann,
                    default,
                });
            }
        }

        self.expect(TokenKind::RBrace).ok()?;

        Some(StructDefNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            name,
            generic_params,
            fields,
            methods,
            is_nominal,
        })
    }

    // ── Tagged Union Definition ─────────────────────────

    pub fn parse_tagged_union_def(&mut self) -> Option<TaggedUnionDefNode> {
        let start = self.current_span();

        if !self.match_token(&TokenKind::Type) {
            return None;
        }

        let name = self.take_ident()?;
        let generic_params = self.parse_generic_params();

        self.expect(TokenKind::Eq).ok()?;

        let mut variants = vec![];
        loop {
            let var = self.parse_variant()?;
            variants.push(var);
            if !self.match_token(&TokenKind::Pipe) {
                break;
            }
        }
        self.match_token(&TokenKind::Semicolon); // optional

        Some(TaggedUnionDefNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            name,
            generic_params,
            variants,
        })
    }

    pub fn parse_variant(&mut self) -> Option<VariantNode> {
        let start = self.current_span();
        let name = self.take_ident()?;

        let fields = if self.match_token(&TokenKind::LBrace) {
            let mut f = vec![];
            while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                let fname = self.take_ident()?;
                let ftype = self.parse_type_annotation()?;
                f.push(StructFieldNode {
                    node_id: new_node_id(),
                    span: Span::dummy(),
                    name: fname,
                    type_ann: ftype,
                    default: None,
                });
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBrace).ok()?;
            f
        } else {
            vec![]
        };

        Some(VariantNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            name,
            fields,
        })
    }

    // ── Class Definition ────────────────────────────────

    pub fn parse_class_def(&mut self) -> Option<ClassDefNode> {
        let start = self.current_span();
        self.expect(TokenKind::Class).ok()?;
        let name = self.take_ident()?;

        let extends = if self.match_keyword("extends") {
            self.take_ident()
        } else {
            None
        };

        let implements = if self.match_keyword("implements") {
            let mut impls = vec![];
            loop {
                if let Some(n) = self.take_ident() {
                    impls.push(n);
                } else {
                    break;
                }
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            impls
        } else {
            vec![]
        };

        self.expect(TokenKind::LBrace).ok()?;

        let mut members = vec![];
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            members.push(self.parse_class_member()?);
        }
        self.expect(TokenKind::RBrace).ok()?;

        Some(ClassDefNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            name,
            extends,
            implements,
            members,
        })
    }

    pub fn parse_class_member(&mut self) -> Option<ClassMemberNode> {
        let start = self.current_span();
        let is_override = self.match_token(&TokenKind::Override);

        let is_static = if self.check(&TokenKind::Static) {
            self.advance();
            true
        } else {
            false
        };

        let visibility = if self.check(&TokenKind::Private) {
            self.advance();
            "private".to_string()
        } else {
            "public".to_string()
        };

        if self.check(&TokenKind::Fn) {
            let func = self.parse_function_def()?;
            Some(ClassMemberNode {
                node_id: new_node_id(),
                span: self.span_from(&start),
                name: func.name.clone(),
                type_ann: None,
                func_def: Some(Box::new(func)),
                is_static,
                visibility,
                init: None,
                is_override,
            })
        } else {
            let name = self.take_ident()?;
            // Could be a field or method
            if self.check(&TokenKind::LParen) {
                // Method (constructor-like or regular method with parens first)
                // Parse as a function
                let func = self.parse_function_def_like(&name, start.clone())?;
                Some(ClassMemberNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    name,
                    type_ann: None,
                    func_def: Some(Box::new(func)),
                    is_static,
                    visibility,
                    init: None,
                    is_override,
                })
            } else {
                let type_ann = if self.check(&TokenKind::Colon) {
                    self.parse_type_annotation()
                } else {
                    None
                };
                let init = if self.match_token(&TokenKind::Eq) {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                self.expect(TokenKind::Semicolon).ok();

                Some(ClassMemberNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    name,
                    type_ann,
                    func_def: None,
                    is_static,
                    visibility,
                    init,
                    is_override,
                })
            }
        }
    }

    fn parse_function_def_like(
        &mut self,
        name: &str,
        start: Span,
    ) -> Option<FunctionDefNode> {
        self.expect(TokenKind::LParen).ok()?;
        let params = self.parse_params();
        self.expect(TokenKind::RParen).ok()?;

        let return_type = if self.match_token(&TokenKind::Arrow) {
            self.parse_type()
        } else {
            None
        };

        let body = self.parse_block()?;

        Some(FunctionDefNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            name: name.to_string(),
            params,
            return_type,
            generic_params: vec![],
            body: Some(body),
            is_async: false,
        })
    }

    // ── Interface Definition ────────────────────────────

    pub fn parse_interface_def(&mut self) -> Option<InterfaceDefNode> {
        let start = self.current_span();
        self.expect(TokenKind::Interface).ok()?;
        let name = self.take_ident()?;
        self.expect(TokenKind::LBrace).ok()?;

        let mut members = vec![];
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            members.push(self.parse_interface_member()?);
        }
        self.expect(TokenKind::RBrace).ok()?;

        Some(InterfaceDefNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            name,
            members,
        })
    }

    pub fn parse_interface_member(&mut self) -> Option<InterfaceMemberNode> {
        let start = self.current_span();

        if self.check(&TokenKind::Fn) {
            self.advance();
            let name = self.take_name()?;
            self.expect(TokenKind::LParen).ok()?;
            let params = self.parse_params();
            self.expect(TokenKind::RParen).ok()?;
            let return_type = if self.match_token(&TokenKind::Arrow) {
                self.parse_type()
            } else {
                None
            };
            self.expect(TokenKind::Semicolon).ok();

            Some(InterfaceMemberNode {
                node_id: new_node_id(),
                span: self.span_from(&start),
                name,
                type_ann: None,
                params,
                return_type,
                is_method: true,
            })
        } else {
            let name = self.take_ident()?;
            let type_ann = self.parse_type_annotation()?;
            self.expect(TokenKind::Semicolon).ok();

            Some(InterfaceMemberNode {
                node_id: new_node_id(),
                span: self.span_from(&start),
                name,
                type_ann: Some(type_ann),
                params: vec![],
                return_type: None,
                is_method: false,
            })
        }
    }

    // ── Impl Block ──────────────────────────────────────

    pub fn parse_impl_block(&mut self) -> Option<ImplBlock> {
        let start = self.current_span();
        self.expect(TokenKind::Impl).ok()?;
        let struct_name = self.take_ident()?;
        let generic_params = self.parse_generic_params();
        self.expect(TokenKind::LBrace).ok()?;

        let mut methods = vec![];
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            if let Some(func) = self.parse_function_def() {
                methods.push(func);
            }
        }
        self.expect(TokenKind::RBrace).ok()?;

        Some(ImplBlock {
            node_id: new_node_id(),
            span: self.span_from(&start),
            struct_name,
            generic_params,
            methods,
        })
    }

    // ── Import/Export ───────────────────────────────────

    pub fn parse_import_stmt(&mut self, is_type_import: bool) -> Option<ImportStmtNode> {
        let start = self.current_span();

        // Consume 'import' keyword
        self.expect(TokenKind::Import).ok()?;

        // Check for "import type" syntax
        let is_type = if self.check(&TokenKind::Type) {
            // Peek: if next after 'type' is {, *, or ident, it's "import type"
            let peek_ahead = self.peek();
            if matches!(peek_ahead.kind, TokenKind::LBrace | TokenKind::Ident(_) | TokenKind::Star) {
                self.advance(); // consume 'type'
                true
            } else {
                false
            }
        } else {
            false
        };
        let is_type = is_type || is_type_import;

        let mut names = vec![];

        if self.match_token(&TokenKind::Star) {
            // import * as name from "..."
            self.expect(TokenKind::As).ok()?;
            let alias = self.take_ident()?;
            names.push((alias, None));
            self.expect(TokenKind::From).ok()?;
        } else {
            self.expect(TokenKind::LBrace).ok()?;
            loop {
                let n = self.take_ident()?;
                let alias = if self.match_token(&TokenKind::As) {
                    self.take_ident()
                } else {
                    None
                };
                names.push((n, alias));
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBrace).ok()?;
            self.expect(TokenKind::From).ok()?;
        }

        let path = self.parse_string_literal()?;
        self.expect(TokenKind::Semicolon).ok();

        Some(ImportStmtNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            names,
            path,
            is_type_import: is_type,
        })
    }

    pub fn parse_export_stmt(&mut self) -> Option<ExportStmtNode> {
        let start = self.current_span();
        self.expect(TokenKind::Export).ok()?;

        match self.current_kind() {
            TokenKind::Fn | TokenKind::Async => {
                let func = self.parse_function_def()?;
                Some(ExportStmtNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    item: Box::new(TopLevelItem::Function(func)),
                })
            }
            TokenKind::Struct | TokenKind::Nominal => {
                let s = self.parse_struct_def()?;
                Some(ExportStmtNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    item: Box::new(TopLevelItem::Struct(s)),
                })
            }
            TokenKind::Type => {
                let tu = self.parse_tagged_union_def()?;
                Some(ExportStmtNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    item: Box::new(TopLevelItem::TaggedUnion(tu)),
                })
            }
            TokenKind::Class => {
                let c = self.parse_class_def()?;
                Some(ExportStmtNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    item: Box::new(TopLevelItem::Class(c)),
                })
            }
            _ => {
                self.error("Expected function, struct, type, or class after 'export'");
                None
            }
        }
    }

    // ── Attribute ───────────────────────────────────────

    pub fn parse_attribute(&mut self) {
        // #[allow(...)], #[deny(...)], #[target(...)]
        self.expect(TokenKind::At).ok();
        // Skip for now - attributes are used by the analyze system
        while !self.check(&TokenKind::Eof)
            && !self.check(&TokenKind::Fn)
            && !self.check(&TokenKind::Struct)
            && !self.check(&TokenKind::Class)
        {
            self.advance();
        }
    }

    // ── Block ───────────────────────────────────────────

    pub fn parse_block(&mut self) -> Option<BlockNode> {
        let start = self.current_span();
        self.expect(TokenKind::LBrace).ok()?;

        let mut stmts = vec![];
        let mut trailing_expr = None;

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            // Skip stray semicolons
            if self.match_token(&TokenKind::Semicolon) {
                continue;
            }

            // Try as statement first (handles assignments, control flow, etc.)
            if let Some(stmt) = self.parse_stmt() {
                stmts.push(stmt);
            } else if self.is_start_of_expr() {
                // Fall back to expression: could be trailing expr or expression statement
                let expr_start = self.pos;
                if let Some(expr) = self.parse_expr() {
                    if self.check(&TokenKind::RBrace) {
                        trailing_expr = Some(Box::new(expr));
                        break;
                    }
                    self.expect(TokenKind::Semicolon).ok();
                    stmts.push(Stmt::ExprStmt(ExprStmtNode {
                        node_id: new_node_id(),
                        span: expr.span().clone(),
                        expr,
                    }));
                    continue;
                } else {
                    self.pos = expr_start;
                }
            } else {
                // Skip to next statement boundary
                while !self.check(&TokenKind::Semicolon)
                    && !self.check(&TokenKind::RBrace)
                    && !self.check(&TokenKind::Eof)
                {
                    self.advance();
                }
                if self.match_token(&TokenKind::Semicolon) {}
            }
        }

        self.expect(TokenKind::RBrace).ok()?;

        Some(BlockNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            stmts,
            trailing_expr,
        })
    }

    fn is_start_of_expr(&self) -> bool {
        match self.current_kind() {
            TokenKind::Ident(_)
            | TokenKind::Number(_)
            | TokenKind::StringLit(_)
            | TokenKind::TemplateString(_)
            | TokenKind::True
            | TokenKind::False
            | TokenKind::If
            | TokenKind::Match
            | TokenKind::Await
            | TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::LBrace  // could be map literal
            | TokenKind::Minus
            | TokenKind::Bang
            | TokenKind::DotDotDot
            | TokenKind::Pipe      // closure
            | TokenKind::OrOr      // empty closure ||
            | TokenKind::Async     // async closure
            => true,
            _ => false,
        }
    }

    // ── Statements ──────────────────────────────────────

    pub fn parse_stmt(&mut self) -> Option<Stmt> {
        match self.current_kind() {
            TokenKind::Let => self.parse_var_decl_or_destructure(),
            TokenKind::If => self.parse_if_stmt().map(Stmt::IfStmt),
            TokenKind::While => self.parse_while_stmt().map(Stmt::WhileStmt),
            TokenKind::For => self.parse_for_stmt().map(Stmt::ForInStmt),
            TokenKind::Break => {
                let start = self.current_span();
                self.advance();
                self.expect(TokenKind::Semicolon).ok();
                Some(Stmt::BreakStmt(BreakStmtNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                }))
            }
            TokenKind::Continue => {
                let start = self.current_span();
                self.advance();
                self.expect(TokenKind::Semicolon).ok();
                Some(Stmt::ContinueStmt(ContinueStmtNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                }))
            }
            TokenKind::Return => self.parse_return_stmt().map(Stmt::ReturnStmt),
            TokenKind::Print | TokenKind::Println => {
                self.parse_print_stmt().map(Stmt::PrintStmt)
            }
            TokenKind::Ident(_) => {
                // Peek: if followed by assignment or index, parse here.
                // Otherwise return None so block parser handles it as expression.
                let after_ident = self.peek();
                let is_assign = matches!(after_ident.kind,
                    TokenKind::Eq | TokenKind::PlusEq | TokenKind::MinusEq
                    | TokenKind::StarEq | TokenKind::SlashEq | TokenKind::PercentEq
                    | TokenKind::LBracket);
                if is_assign {
                    let name = self.take_ident()?;
                    self.parse_assign_or_expr_stmt(name, 0)
                } else {
                    None // let block parser handle as expression
                }
            }
            _ => {
                // Not a statement — let the block parser handle it as expression/error
                None
            }
        }
    }

    fn parse_var_decl_or_destructure(&mut self) -> Option<Stmt> {
        let start = self.current_span();
        self.expect(TokenKind::Let).ok()?;
        let mutable = self.match_token(&TokenKind::Mut);

        // Check for destructure pattern
        if self.check(&TokenKind::LBracket) || self.check(&TokenKind::LBrace) {
            return self.parse_destructure(start, mutable).map(Stmt::Destructure);
        }

        let name = self.take_ident()?;
        let type_ann = if self.check(&TokenKind::Colon) {
            self.parse_type_annotation()
        } else {
            None
        };

        self.expect(TokenKind::Eq).ok()?;
        let init = self.parse_expr()?;
        self.expect(TokenKind::Semicolon).ok();

        Some(Stmt::VarDecl(VarDeclNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            mutable,
            name,
            type_ann,
            init,
        }))
    }

    fn parse_destructure(&mut self, start: Span, mutable: bool) -> Option<DestructureNode> {
        // Simple array destructure: let [a, b] = expr;
        // Or object destructure: let {name, age} = expr;
        let pattern = if self.match_token(&TokenKind::LBracket) {
            let mut elements = vec![];
            let mut rest = None;
            while !self.check(&TokenKind::RBracket) && !self.check(&TokenKind::Eof) {
                if self.match_token(&TokenKind::DotDotDot) {
                    rest = self.take_ident();
                    break;
                } else if self.match_token(&TokenKind::Underscore) {
                    elements.push(DestructureElement::Ignore);
                } else {
                    let name = self.take_ident()?;
                    elements.push(DestructureElement::Bind(name));
                }
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBracket).ok()?;
            DestructurePattern::Array(elements, rest)
        } else if self.match_token(&TokenKind::LBrace) {
            let mut fields = vec![];
            while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                let field = self.take_ident()?;
                let bind = if self.match_token(&TokenKind::Colon) {
                    self.take_ident()?
                } else {
                    field.clone()
                };
                fields.push((field, bind));
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::RBrace).ok()?;
            DestructurePattern::Object(fields)
        } else {
            self.error("Expected '[' or '{' for destructure pattern");
            return None;
        };

        self.expect(TokenKind::Eq).ok()?;
        let init = self.parse_expr()?;
        self.expect(TokenKind::Semicolon).ok();

        Some(DestructureNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            mutable,
            pattern,
            init,
        })
    }

    fn parse_assign_or_expr_stmt(&mut self, name: String, _pos: usize) -> Option<Stmt> {
        // Check for assignment: name = expr; name += expr; name[expr] = expr;
        let is_assign_op = matches!(self.current_kind(),
            TokenKind::Eq | TokenKind::PlusEq | TokenKind::MinusEq
            | TokenKind::StarEq | TokenKind::SlashEq | TokenKind::PercentEq);
        if is_assign_op {
            let start = self.current_span();
            let op = match self.current_kind() {
                TokenKind::Eq => "=",
                TokenKind::PlusEq => "+=",
                TokenKind::MinusEq => "-=",
                TokenKind::StarEq => "*=",
                TokenKind::SlashEq => "/=",
                TokenKind::PercentEq => "%=",
                _ => "=",
            };
            self.advance();
            let value = self.parse_expr()?;
            self.expect(TokenKind::Semicolon).ok();
            return Some(Stmt::Assign(AssignNode {
                node_id: new_node_id(),
                span: self.span_from(&start),
                target: name,
                index: None,
                op: op.to_string(),
                value,
            }));
        }

        // Check for index assignment: name[index] = expr
        if self.check(&TokenKind::LBracket) {
            self.advance();
            let index = self.parse_expr()?;
            self.expect(TokenKind::RBracket).ok()?;

            let op = match self.current_kind() {
                TokenKind::Eq => "=",
                TokenKind::PlusEq => "+=",
                TokenKind::MinusEq => "-=",
                TokenKind::StarEq => "*=",
                TokenKind::SlashEq => "/=",
                TokenKind::PercentEq => "%=",
                _ => "=",
            };
            self.advance();
            let value = self.parse_expr()?;
            self.expect(TokenKind::Semicolon).ok();

            return Some(Stmt::Assign(AssignNode {
                node_id: new_node_id(),
                span: Span::dummy(),
                target: name,
                index: Some(Box::new(index)),
                op: op.to_string(),
                value,
            }));
        }

        // Expression statement: name.method(args) or name(args) etc.
        // Reconstruct VarRef from the already-consumed name and apply postfix
        let start = Span::dummy(); // approximate
        let var_ref = Expr::VarRef(VarRefNode {
            node_id: new_node_id(),
            span: start.clone(),
            name,
        });
        let expr = self.parse_postfix(var_ref)?;
        self.expect(TokenKind::Semicolon).ok();
        Some(Stmt::ExprStmt(ExprStmtNode {
            node_id: new_node_id(),
            span: expr.span().clone(),
            expr,
        }))
    }

    pub fn parse_if_stmt(&mut self) -> Option<IfStmtNode> {
        let start = self.current_span();
        self.expect(TokenKind::If).ok()?;
        let condition = self.parse_expr()?;
        let then_body = self.parse_block()?;

        let mut else_ifs = vec![];
        let mut else_body = None;

        while self.match_keyword("else") {
            if self.match_token(&TokenKind::If) {
                let cond = self.parse_expr()?;
                let body = self.parse_block()?;
                else_ifs.push((cond, body));
            } else {
                else_body = Some(self.parse_block()?);
                break;
            }
        }

        Some(IfStmtNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            condition,
            then_body,
            else_ifs,
            else_body,
        })
    }

    pub fn parse_while_stmt(&mut self) -> Option<WhileStmtNode> {
        let start = self.current_span();
        self.expect(TokenKind::While).ok()?;
        let condition = self.parse_expr()?;
        let body = self.parse_block()?;
        Some(WhileStmtNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            condition,
            body,
        })
    }

    pub fn parse_for_stmt(&mut self) -> Option<ForInStmtNode> {
        let start = self.current_span();
        self.expect(TokenKind::For).ok()?;
        let var_name = self.take_ident()?;
        self.expect(TokenKind::In).ok()?;
        let iterable = self.parse_expr()?;
        let body = self.parse_block()?;
        Some(ForInStmtNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            var_name,
            iterable,
            body,
        })
    }

    pub fn parse_return_stmt(&mut self) -> Option<ReturnStmtNode> {
        let start = self.current_span();
        self.expect(TokenKind::Return).ok()?;
        let value = if self.check(&TokenKind::Semicolon) {
            None
        } else if self.is_start_of_expr() {
            let expr = self.parse_expr()?;
            Some(Box::new(expr))
        } else {
            None
        };
        self.expect(TokenKind::Semicolon).ok();
        Some(ReturnStmtNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            value,
        })
    }

    pub fn parse_print_stmt(&mut self) -> Option<PrintStmtNode> {
        let start = self.current_span();
        let newline = if self.match_token(&TokenKind::Println) {
            true
        } else {
            self.match_token(&TokenKind::Print);
            false
        };

        self.expect(TokenKind::LParen).ok()?;
        let arg = self.parse_expr()?;
        self.expect(TokenKind::RParen).ok()?;
        self.expect(TokenKind::Semicolon).ok();

        Some(PrintStmtNode {
            node_id: new_node_id(),
            span: self.span_from(&start),
            newline,
            arg,
        })
    }

    // ── Helpers ─────────────────────────────────────────

    fn parse_string_literal(&mut self) -> Option<String> {
        match self.current_kind() {
            TokenKind::StringLit(s) => {
                let s = s.clone();
                self.advance();
                Some(s)
            }
            _ => {
                self.error("Expected string literal");
                None
            }
        }
    }
}
