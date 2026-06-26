use omni_compiler::lexer::tokenize;
use omni_compiler::parser::parse;

#[test]
fn test_all_remaining() {
    let files = [
        (r"D:\space\OmniLang\test\projects\learn-omniview\src\pages\ComponentSpecialsDemo.omni", "ComponentSpecialsDemo"),
        (r"D:\space\OmniLang\test\projects\learn-omniview\src\components\OptionalChainDemo.omni", "OptionalChainDemo"),
        (r"D:\space\OmniLang\test\projects\learn-omniview\src\components\CssVarsDemo.omni", "CssVarsDemo"),
        (r"D:\space\OmniLang\test\projects\learn-omniview\src\components\SlotDefaultDemo.omni", "SlotDefaultDemo"),
        (r"D:\space\OmniLang\test\projects\learn-omniview\src\components\ThemeCssVarsDemo.omni", "ThemeCssVarsDemo"),
    ];
    for (path, name) in &files {
        let source = std::fs::read_to_string(path).unwrap();
        let tokens = tokenize(&source);
        let result = parse(&tokens, &source);
        match result {
            Ok(_) => eprintln!("OK: {}", name),
            Err(e) => eprintln!("FAIL {}: line {}: {}", name, e.span.line, e.message),
        }
    }
}
