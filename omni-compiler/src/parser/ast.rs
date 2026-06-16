//! AST node types for the OmniLang parser.
//! All AST nodes live here. The parser produces a Program from token stream.

#[derive(Debug, Clone)]
pub struct Program {
    pub components: Vec<()>,
    pub stores: Vec<()>,
    pub themes: Vec<()>,
    pub functions: Vec<()>,
    pub structs: Vec<()>,
    pub tagged_unions: Vec<()>,
    pub imports: Vec<()>,
}
