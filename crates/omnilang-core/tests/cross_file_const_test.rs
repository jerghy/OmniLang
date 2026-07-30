use omnilang_core::analyse::analyse_multi;
use rustc_hash::FxHashMap;

#[test]
fn cross_file_const_in_design_token() {
    let mut sources = FxHashMap::default();
    sources.insert(
        "./tokens.omni".to_string(),
        r##"
        const BRAND = "#6366f1";
        const SPACING = "16px";
        "##
        .to_string(),
    );
    sources.insert(
        "./theme.omni".to_string(),
        r##"
        import omniview;
        import { BRAND, SPACING } from "./tokens.omni";

        design AppDesign {
            token primary: color = BRAND;
            token spacingMd: size = SPACING;
        }
        "##
        .to_string(),
    );

    // Map import paths to source file keys
    let mut path_mapping = FxHashMap::default();
    path_mapping.insert("./tokens.omni".to_string(), "./tokens.omni".to_string());
    path_mapping.insert("./theme.omni".to_string(), "./theme.omni".to_string());
    let result = analyse_multi(&sources, &path_mapping);
    // Check no errors
    let errors: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.severity == omnilang_core::error::Severity::Error)
        .collect();
    assert!(
        errors.is_empty(),
        "should have no errors, got: {:?}",
        errors
    );
}
