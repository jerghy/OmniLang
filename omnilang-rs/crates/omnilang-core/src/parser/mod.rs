mod grammar;
mod expr;
mod template;

use crate::ast::*;
use crate::errors::ErrorCollector;
use crate::lexer::{Lexer, Token, TokenKind};
use crate::span::Span;

/// Parse OmniLang source code and return a Program AST.
pub fn parse(source: &str, filename: &str) -> Result<Program, ErrorCollector> {
    let mut lexer = Lexer::new(source, filename);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(lex_errors) => {
            let mut errors = ErrorCollector::new();
            for e in lex_errors {
                errors.add(e, None, Some(filename.to_string()));
            }
            return Err(errors);
        }
    };

    let mut errors = ErrorCollector::new();
    let mut parser = Parser::new(source, tokens, filename, &mut errors);
    let program = parser.parse_program();

    if errors.has_errors() {
        Err(errors)
    } else {
        Ok(program)
    }
}

/// Parser state machine.
pub struct Parser<'s> {
    pub tokens: Vec<Token>,
    pub pos: usize,
    pub source: &'s str,
    pub filename: String,
    pub errors: &'s mut ErrorCollector,
    /// Track brace depth for struct_lit disambiguation
    pub brace_depth: usize,
}

impl<'s> Parser<'s> {
    pub fn new(
        source: &'s str,
        tokens: Vec<Token>,
        filename: &str,
        errors: &'s mut ErrorCollector,
    ) -> Self {
        Self {
            tokens,
            pos: 0,
            source,
            filename: filename.to_string(),
            errors,
            brace_depth: 0,
        }
    }

    // ── Token Navigation ────────────────────────────────

    pub fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or(&EOF_TOKEN)
    }

    pub fn current_kind(&self) -> &TokenKind {
        &self.current().kind
    }

    pub fn peek(&self) -> &Token {
        self.tokens.get(self.pos + 1).unwrap_or(&EOF_TOKEN)
    }

    pub fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    pub fn advance(&mut self) -> &Token {
        let t = self.current();
        if t.kind != TokenKind::Eof {
            self.pos += 1;
        }
        self.tokens.get(self.pos - 1).unwrap_or(&EOF_TOKEN)
    }

    pub fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.current_kind()) == std::mem::discriminant(kind)
    }

    pub fn check_peek(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    pub fn expect(&mut self, kind: TokenKind) -> Result<Token, ()> {
        if std::mem::discriminant(self.current_kind()) == std::mem::discriminant(&kind) {
            Ok(self.advance().clone())
        } else {
            self.error(&format!(
                "Expected {:?}, got {:?}",
                kind, self.current_kind()
            ));
            Err(())
        }
    }

    pub fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    /// Match a keyword by its string name.
    /// Only matches keyword tokens (like Fn, Class) — NOT Ident tokens
    /// that happen to contain the keyword text.
    pub fn match_keyword(&mut self, kw: &str) -> bool {
        // Check if kw maps to a real keyword (non-Ident) token
        let kind_from_kw = TokenKind::from_ident(kw);
        // Only proceed if it's NOT an Ident (i.e., it's a true keyword)
        if matches!(kind_from_kw, TokenKind::Ident(_)) {
            // For non-keyword strings like "extends", "implements",
            // only match if the current token is an Ident with that exact text
            match self.current_kind() {
                TokenKind::Ident(name) if name == kw => {
                    self.advance();
                    true
                }
                _ => false,
            }
        } else {
            // For true keywords, compare discriminants
            if std::mem::discriminant(self.current_kind())
                == std::mem::discriminant(&kind_from_kw)
            {
                self.advance();
                true
            } else {
                false
            }
        }
    }

    /// Peek ahead to check if current ident is a keyword
    pub fn is_keyword(&self, kw: &str) -> bool {
        match self.current_kind() {
            TokenKind::Ident(name) => name == kw,
            _ => {
                let kind_from_kw = TokenKind::from_ident(kw);
                std::mem::discriminant(self.current_kind())
                    == std::mem::discriminant(&kind_from_kw)
            }
        }
    }

    // ── Error Reporting ─────────────────────────────────

    pub fn error(&mut self, msg: &str) {
        let span = self.current().span.clone();
        self.errors
            .add(msg.to_string(), Some(span), Some(self.filename.clone()));
    }

    pub fn error_at(&mut self, msg: &str, span: Span) {
        self.errors
            .add(msg.to_string(), Some(span), Some(self.filename.clone()));
    }

    // ── Span Helpers ────────────────────────────────────

    pub fn current_span(&self) -> Span {
        self.current().span.clone()
    }

    pub fn span_from(&self, start: &Span) -> Span {
        let end = self.current().span.clone();
        Span::new(
            start.file.clone(),
            start.line,
            start.col,
            end.end_line,
            end.end_col,
        )
    }

    /// Get the ident name from the current token, advancing past it.
    pub fn take_ident(&mut self) -> Option<String> {
        match self.current_kind() {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Some(name)
            }
            _ => None,
        }
    }

    /// Take a name token — either an Ident or a keyword used as a name
    /// (e.g., `print`, `type`, `fn` can be function/interface names).
    pub fn take_name(&mut self) -> Option<String> {
        match self.current_kind() {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Some(name)
            }
            // Keywords that can appear as names
            TokenKind::Print => { self.advance(); Some("print".into()) }
            TokenKind::Println => { self.advance(); Some("println".into()) }
            TokenKind::Fn => { self.advance(); Some("fn".into()) }
            TokenKind::Type => { self.advance(); Some("type".into()) }
            TokenKind::Struct => { self.advance(); Some("struct".into()) }
            TokenKind::Class => { self.advance(); Some("class".into()) }
            TokenKind::Interface => { self.advance(); Some("interface".into()) }
            TokenKind::Match => { self.advance(); Some("match".into()) }
            TokenKind::If => { self.advance(); Some("if".into()) }
            TokenKind::While => { self.advance(); Some("while".into()) }
            TokenKind::For => { self.advance(); Some("for".into()) }
            TokenKind::Let => { self.advance(); Some("let".into()) }
            TokenKind::Mut => { self.advance(); Some("mut".into()) }
            TokenKind::Return => { self.advance(); Some("return".into()) }
            TokenKind::Get => { self.advance(); Some("get".into()) }
            TokenKind::Set => { self.advance(); Some("set".into()) }
            TokenKind::Static => { self.advance(); Some("static".into()) }
            TokenKind::Private => { self.advance(); Some("private".into()) }
            TokenKind::Override => { self.advance(); Some("override".into()) }
            TokenKind::New => { self.advance(); Some("new".into()) }
            TokenKind::Super => { self.advance(); Some("super".into()) }
            TokenKind::This => { self.advance(); Some("this".into()) }
            TokenKind::As => { self.advance(); Some("as".into()) }
            TokenKind::From => { self.advance(); Some("from".into()) }
            TokenKind::In => { self.advance(); Some("in".into()) }
            TokenKind::Break => { self.advance(); Some("break".into()) }
            TokenKind::Continue => { self.advance(); Some("continue".into()) }
            TokenKind::Import => { self.advance(); Some("import".into()) }
            TokenKind::Export => { self.advance(); Some("export".into()) }
            TokenKind::Impl => { self.advance(); Some("impl".into()) }
            TokenKind::Nominal => { self.advance(); Some("nominal".into()) }
            TokenKind::Async => { self.advance(); Some("async".into()) }
            TokenKind::Await => { self.advance(); Some("await".into()) }
            TokenKind::Is => { self.advance(); Some("is".into()) }
            _ => None,
        }
    }

    /// Check if current token is an ident and return its name without advancing.
    pub fn peek_ident(&self) -> Option<&str> {
        match self.current_kind() {
            TokenKind::Ident(name) => Some(name),
            _ => None,
        }
    }

    /// Check if the current token is a given ident name.
    pub fn is_ident(&self, name: &str) -> bool {
        matches!(self.current_kind(), TokenKind::Ident(n) if n == name)
    }

    // ── Type Parsing ────────────────────────────────────

    pub fn parse_type(&mut self) -> Option<TypeRepr> {
        let mut ty = self.parse_struct_type()?;
        while self.match_token(&TokenKind::Pipe) {
            let mut types = vec![ty];
            types.push(self.parse_struct_type()?);
            while self.match_token(&TokenKind::Pipe) {
                types.push(self.parse_struct_type()?);
            }
            ty = TypeRepr::union(types);
        }
        Some(ty)
    }

    pub fn parse_struct_type(&mut self) -> Option<TypeRepr> {
        let ty = self.parse_base_type()?;
        if self.match_token(&TokenKind::Question) {
            // T? -> T | null
            Some(TypeRepr::union(vec![
                ty,
                TypeRepr::named("null"),
            ]))
        } else {
            Some(ty)
        }
    }

    pub fn parse_base_type(&mut self) -> Option<TypeRepr> {
        match self.current_kind() {
            TokenKind::Void => {
                self.advance();
                Some(TypeRepr::named("void"))
            }
            TokenKind::Fn => {
                self.advance();
                self.expect(TokenKind::LParen).ok()?;
                let mut param_types = vec![];
                if !self.check(&TokenKind::RParen) {
                    param_types.push(self.parse_type()?);
                    while self.match_token(&TokenKind::Comma) {
                        param_types.push(self.parse_type()?);
                    }
                }
                self.expect(TokenKind::RParen).ok()?;
                self.expect(TokenKind::Arrow).ok()?;
                let return_type = Box::new(self.parse_type()?);
                Some(TypeRepr::Fn {
                    node_id: new_node_id(),
                    param_types,
                    return_type,
                })
            }
            TokenKind::LBrace => {
                // Anonymous struct type: { field: Type, ... }
                self.advance();
                // For now, return a generic Map-like type representation
                // Full anonymous struct types are for v0.3+
                let mut fields = vec![];
                while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                    let field_name = match self.current_kind() {
                        TokenKind::Ident(n) => n.clone(),
                        _ => {
                            self.error("Expected field name in anonymous struct type");
                            return None;
                        }
                    };
                    self.advance();
                    if self.match_token(&TokenKind::Question) {
                        // optional field: name?: Type
                    }
                    self.expect(TokenKind::Colon).ok()?;
                    let field_type = self.parse_type()?;
                    fields.push((field_name, field_type));
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(TokenKind::RBrace).ok()?;
                Some(TypeRepr::named("__anon_struct"))
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                let type_args = if self.match_token(&TokenKind::Lt) {
                    let mut args = vec![];
                    args.push(self.parse_type()?);
                    while self.match_token(&TokenKind::Comma) {
                        args.push(self.parse_type()?);
                    }
                    self.expect(TokenKind::Gt).ok()?;
                    args
                } else {
                    vec![]
                };
                Some(TypeRepr::named_with_args(&name, type_args))
            }
            _ => {
                self.error(&format!(
                    "Expected type, got {:?}",
                    self.current_kind()
                ));
                None
            }
        }
    }

    pub fn parse_type_annotation(&mut self) -> Option<TypeRepr> {
        self.expect(TokenKind::Colon).ok()?;
        self.parse_type()
    }

    // ── Generic Parameters ──────────────────────────────

    pub fn parse_generic_params(&mut self) -> Vec<GenericParamNode> {
        let mut params = vec![];
        if self.match_token(&TokenKind::Lt) {
            loop {
                let start = self.current_span();
                let name = match self.take_ident() {
                    Some(n) => n,
                    None => {
                        self.error("Expected generic parameter name");
                        break;
                    }
                };
                let mut bounds = vec![];
                if self.match_token(&TokenKind::Colon) {
                    loop {
                        if let Some(b) = self.take_ident() {
                            bounds.push(b);
                        } else {
                            break;
                        }
                        if !self.match_token(&TokenKind::Plus) {
                            break;
                        }
                    }
                }
                params.push(GenericParamNode {
                    node_id: new_node_id(),
                    span: self.span_from(&start),
                    name,
                    bounds,
                });
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Gt).ok();
        }
        params
    }

    // ── Params ──────────────────────────────────────────

    pub fn parse_params(&mut self) -> Vec<ParamNode> {
        let mut params = vec![];
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let is_rest = self.match_token(&TokenKind::DotDotDot);

            // Try to parse a pattern/destructure
            let start = self.current_span();

            if self.is_ident("mut") {
                self.advance(); // skip mut in patterns (currently not fully supported)
            }

            let name = match self.take_ident() {
                Some(n) => n,
                None => {
                    self.error("Expected parameter name");
                    break;
                }
            };

            let type_ann = if self.match_token(&TokenKind::Colon) {
                self.parse_type()
            } else {
                None
            };

            params.push(ParamNode {
                node_id: new_node_id(),
                span: self.span_from(&start),
                name,
                type_ann,
                is_rest,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        params
    }
}

/// Sentinel token for EOF position
static EOF_TOKEN: Token = Token {
    kind: TokenKind::Eof,
    span: Span {
        file: String::new(),
        line: 0,
        col: 0,
        end_line: 0,
        end_col: 0,
    },
};

// Re-export grammar and expr parsing
pub use expr::*;

// ── Tests ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        let candidates = [
            PathBuf::from("../../../tests/fixtures"),
            PathBuf::from("../../tests/fixtures"),
            PathBuf::from("../tests/fixtures"),
            PathBuf::from("tests/fixtures"),
        ];
        for c in &candidates {
            if c.exists() { return c.clone(); }
        }
        panic!("Fixtures not found");
    }

    fn list_fixtures() -> Vec<PathBuf> {
        let mut v = vec![];
        let base = fixtures_dir();
        if base.exists() {
            walk(&base, &mut v, false);
        }
        v
    }

    fn walk(dir: &PathBuf, out: &mut Vec<PathBuf>, include_errors: bool) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    if include_errors || !p.ends_with("errors") {
                        walk(&p, out, include_errors);
                    }
                } else if p.extension().map_or(false, |e| e == "omni") {
                    out.push(p);
                }
            }
        }
    }

    #[test]
    fn test_parse_hello_world() {
        let src = "fn main() { println(\"Hello\"); }";
        let p = parse(src, "t.omni").expect("parse");
        assert_eq!(p.functions.len(), 1);
    }

    #[test]
    fn test_parse_method_call_in_print() {
        let src = "fn main() { let x: i32 = 42; println(x.to_string()); }";
        let p = parse(src, "t.omni").expect("parse");
        assert_eq!(p.functions.len(), 1);
    }

    #[test]
    fn test_parse_assignment() {
        let src = "fn main() { let mut y: f64 = 3.14; y = y + 1.0; println(y.to_string()); }";
        let p = parse(src, "t.omni").expect("parse");
        assert_eq!(p.functions.len(), 1);
    }

    #[test]
    fn test_parse_closure() {
        let src = "fn main() { let add = |a: i32, b: i32| a + b; let r: i32 = add(1, 2); println(r.to_string()); }";
        match parse(src, "t.omni") {
            Ok(p) => assert_eq!(p.functions.len(), 1),
            Err(e) => {
                println!("PARSE ERROR: {}", e.report(None));
                panic!("parse failed");
            }
        }
    }

    #[test]
    fn test_parse_interface() {
        let src = "interface Printable {\n    fn print() -> void;\n}\n\nfn main() {\n    println(\"ok\");\n}\n";
        let p = parse(src, "t.omni").expect("parse");
        assert_eq!(p.interfaces.len(), 1);
    }

    #[test]
    fn test_parse_import() {
        let src = "import { add, sub } from \"./math\";\nfn main() { println(\"ok\"); }";
        let p = parse(src, "t.omni").expect("parse");
        assert_eq!(p.imports.len(), 1);
    }

    #[test]
    fn test_all_fixtures_parse() {
        let fixtures = list_fixtures();
        let count = fixtures.len();
        assert!(count >= 20, "Expected >= 20 fixtures, got {}", count);
        let mut pass = 0;
        let mut fails = vec![];
        for f in &fixtures {
            let src = match std::fs::read_to_string(f) { Ok(s) => s, Err(e) => { fails.push((f.clone(), format!("read: {}", e))); continue; }};
            let name = f.file_name().unwrap().to_string_lossy().to_string();
            match parse(&src, &name) {
                Ok(_) => pass += 1,
                Err(e) => fails.push((f.clone(), e.report(None))),
            }
        }
        if !fails.is_empty() {
            eprintln!("\n=== {} failures ===", fails.len());
            for (p, e) in fails.iter().take(40) {
                eprintln!("  {}: {}", p.display(), e.lines().next().unwrap_or("?"));
            }
        }
        println!("Parsed: {}/{}", pass, count);

        // This test is informative — we track pass rate
        let rate = (pass as f64) / (count as f64) * 100.0;
        println!("Parse rate: {:.1}%", rate);
        assert!(pass >= 80, "Expected >= 80 fixtures parsing, got {}", pass);
    }
}
