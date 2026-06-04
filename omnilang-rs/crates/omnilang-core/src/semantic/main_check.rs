use crate::ast::*;
use crate::errors::ErrorCollector;

/// Validate the program has a valid `fn main()` entry point.
pub fn check(program: &Program, errors: &mut ErrorCollector) {
    let mut main_func: Option<&FunctionDefNode> = None;

    for func in &program.functions {
        if func.name == "main" {
            main_func = Some(func);
            break;
        }
    }

    match main_func {
        None => {
            errors.add(
                "Program must contain a fn main() entry point".to_string(),
                None,
                None,
            );
        }
        Some(main) => {
            // Check: main must have no parameters
            if !main.params.is_empty() {
                errors.add(
                    "fn main() must have no parameters".to_string(),
                    Some(main.span.clone()),
                    None,
                );
            }

            // Check: main must not declare a non-void return type
            if let Some(ref rt) = main.return_type {
                if rt.base_name() != "void" {
                    errors.add(
                        "fn main() must not declare a return type (or only void)".to_string(),
                        Some(main.span.clone()),
                        None,
                    );
                }
            }

            // Check: main body must not have a trailing expression
            if let Some(ref body) = main.body {
                if body.trailing_expr.is_some() {
                    errors.add(
                        "fn main() cannot have a trailing expression (implicit return)".to_string(),
                        Some(body.span.clone()),
                        None,
                    );
                }

                // Check: no return statements with values in main
                check_no_return_value(&body.stmts, errors);
            }
        }
    }
}

fn check_no_return_value(stmts: &[Stmt], errors: &mut ErrorCollector) {
    for stmt in stmts {
        match stmt {
            Stmt::ReturnStmt(rs) => {
                if rs.value.is_some() {
                    errors.add(
                        "fn main() cannot return a value".to_string(),
                        Some(rs.span.clone()),
                        None,
                    );
                }
            }
            Stmt::IfStmt(s) => {
                check_no_return_value_in_block(&s.then_body.stmts, errors);
                for (_, body) in &s.else_ifs {
                    check_no_return_value_in_block(&body.stmts, errors);
                }
                if let Some(ref else_body) = s.else_body {
                    check_no_return_value_in_block(&else_body.stmts, errors);
                }
            }
            Stmt::WhileStmt(s) => {
                check_no_return_value_in_block(&s.body.stmts, errors);
            }
            Stmt::ForInStmt(s) => {
                check_no_return_value_in_block(&s.body.stmts, errors);
            }
            _ => {}
        }
    }
}

fn check_no_return_value_in_block(stmts: &[Stmt], errors: &mut ErrorCollector) {
    check_no_return_value(stmts, errors);
}
