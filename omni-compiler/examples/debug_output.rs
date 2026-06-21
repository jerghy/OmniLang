use omni_compiler::codegen::omniview;
use omni_compiler::ir;
use omni_compiler::lexer;
use omni_compiler::parser;
use omni_compiler::semantic;

fn debug(src: &str, label: &str) {
    let tokens = lexer::tokenize(src);
    match parser::parse(&tokens, src) {
        Ok(program) => {
            let semantics = semantic::analyze(&program);
            let ir_program = ir::lower(&program, &semantics);
            let output = omniview::generate(&ir_program);
            println!("=== {} ===", label);
            println!("{}", output.js);
            println!();
        }
        Err(e) => eprintln!("Parse error for {}: {}", label, e.message),
    }
}

fn main() {
    debug(
        "component Counter { signal count: i32 = 0 fn inc() { count = count + 1 } view { <br/> } }",
        "FUNC",
    );
    debug(
        "component App { fn handle() { } view { <Button @onPress={handle}>Go</Button> } }",
        "EVENT",
    );
    debug(
        "component App { fn click() { } view { <Button onPress={click}>Go</Button> } }",
        "ONPRESS",
    );
}
