pub mod resolver;
pub mod type_checker;
pub mod shadow;
pub mod restrict;
pub mod main_check;
pub mod capture;

use crate::ast::*;
use crate::errors::ErrorCollector;
use std::collections::HashMap;

pub fn analyze(
    program: &Program,
    _source_lines: Option<&HashMap<String, Vec<String>>>,
) -> Result<
    (
        HashMap<NodeId, TypeRepr>,
        Vec<HashMap<String, String>>,
    ),
    ErrorCollector,
> {
    let mut errors = ErrorCollector::new();

    // Pass 1: Symbol resolution
    let scope_info = resolver::resolve(program, &mut errors);

    // Pass 2: Type checking
    let type_map = type_checker::check(program, &scope_info, &mut errors);

    // Pass 3: Shadow detection
    let rename_map = shadow::check(program, &scope_info, &mut errors);

    // Pass 4: Restriction checking
    restrict::check(program, &scope_info, &type_map, &mut errors);

    // Pass 5: Main function validation
    main_check::check(program, &mut errors);

    if errors.has_errors() {
        return Err(errors);
    }

    Ok((type_map, rename_map))
}
