; OmniLang Tree-sitter Grammar — Syntax Highlighting
; Phase 8: VS Code / IDE integration

; ── Keywords ──────────────────────────────────────────
[
  "fn" "let" "mut" "if" "else" "while" "for" "in" "break" "continue" "return"
  "true" "false" "struct" "class" "new" "this" "extends" "implements"
  "match" "type" "import" "export" "from" "as"
  "async" "await" "nominal" "impl"
] @keyword

; ── Types ─────────────────────────────────────────────
[
  "i32" "f64" "bool" "String" "Array" "Map" "void"
] @type.builtin

(type_identifier) @type
(generic_param) @type

; ── Functions ─────────────────────────────────────────
(function_def name: (identifier) @function)
(method_def name: (identifier) @method)
(call_expr function: (identifier) @function.call)

; ── Variables ─────────────────────────────────────────
(var_decl name: (identifier) @variable)
(param name: (identifier) @parameter)
(var_ref) @variable

; ── Strings ───────────────────────────────────────────
(string_lit) @string
(template_string) @string
(template_expr "${" @punctuation.special "}" @punctuation.special) @embedded

; ── Numbers ───────────────────────────────────────────
(number_lit) @number

; ── Comments ──────────────────────────────────────────
(comment_single) @comment.line
(comment_block) @comment.block

; ── Operators ─────────────────────────────────────────
[
  "+" "-" "*" "/" "%"
  "=" "+=" "-=" "*=" "/=" "%="
  "==" "!=" "<" ">" "<=" ">="
  "&&" "||" "!"
  "->" "=>" "|"
  "?" "?." "??"
  "..."
] @operator

; ── Punctuation ───────────────────────────────────────
[
  "(" ")" "{" "}" "[" "]" "." "," ":" ";" "@"
] @punctuation.delimiter

; ── Decorators ────────────────────────────────────────
(decorator) @attribute
