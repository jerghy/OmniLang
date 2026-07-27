/// Build script for the omnilang crate.
///
/// Sets `cfg(any_backend_enabled)` when any codegen backend feature is active,
/// centralizing the 8-backend feature gate list so that adding a new backend
/// only requires editing this single file.
fn main() {
    // Register the custom cfg so that clippy doesn't warn about an unknown cfg name.
    println!("cargo:rustc-check-cfg=cfg(any_backend_enabled)");
    let backends = [
        "codegen-javascript",
        "codegen-react",
        "codegen-vue",
        "codegen-dart",
        "codegen-flutter",
        "codegen-rust",
        "codegen-dioxus",
        "codegen-python",
    ];

    for backend in &backends {
        // Cargo converts feature names: `codegen-javascript` -> `CARGO_FEATURE_CODEGEN_JAVASCRIPT`
        let env_var = format!(
            "CARGO_FEATURE_{}",
            backend.replace('-', "_").to_uppercase()
        );
        if std::env::var_os(&env_var).is_some() {
            println!("cargo:rustc-cfg=any_backend_enabled");
            return;
        }
    }
}
