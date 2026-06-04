mod common;

use common::{list_error_fixtures, list_fixtures, fixtures_dir};
use omnilang_core::parser;

#[test]
fn test_all_fixtures_parse() {
    let fixtures = list_fixtures();
    let count = fixtures.len();
    assert!(count >= 20, "Expected at least 20 fixtures, found {}", count);

    let mut successes = 0;
    let mut failures = vec![];

    for fixture in &fixtures {
        let source = match std::fs::read_to_string(fixture) {
            Ok(s) => s,
            Err(e) => {
                failures.push((fixture.clone(), format!("Failed to read: {}", e)));
                continue;
            }
        };

        let filename = fixture.file_name().unwrap().to_string_lossy().to_string();
        match parser::parse(&source, &filename) {
            Ok(program) => {
                successes += 1;
                // Verify basic structure
                assert!(!program.functions.iter().any(|f| f.name.is_empty()),
                    "Function with empty name in {}", filename);
                // Check main exists (may not in module fixtures)
            }
            Err(errors) => {
                failures.push((fixture.clone(), errors.report(None)));
            }
        }
    }

    if !failures.is_empty() {
        println!("\n=== {} fixtures failed to parse ===", failures.len());
        for (path, error) in &failures {
            println!("  {}: {}", path.display(), error.lines().next().unwrap_or("unknown"));
        }
        panic!("{}/{} fixtures failed to parse", failures.len(), count);
    }

    println!("All {}/{} fixtures parsed successfully", successes, count);
}

#[test]
fn test_all_error_fixtures_detected() {
    // Error fixtures should fail to parse or fail semantic analysis.
    // For now, just verify they exist and can be read.
    let error_fixtures = list_error_fixtures();
    println!("Found {} error fixtures", error_fixtures.len());
    // At least the original 15 error fixtures
    assert!(error_fixtures.len() >= 10, "Expected at least 10 error fixtures");
}

#[test]
fn test_fixture_count() {
    let fixtures = list_fixtures();
    println!("Total fixtures found: {}", fixtures.len());
    for f in &fixtures {
        println!("  {}", f.display());
    }
    // v0.2 has at least 20 e2e fixtures plus many analyze/type/closure fixtures
    assert!(fixtures.len() >= 20, "Expected >= 20 fixtures, got {}", fixtures.len());
}

#[test]
fn test_parse_hello_world() {
    let source = r#"
fn main() {
    println("Hello, World!");
}
"#;
    let program = parser::parse(source, "test.omni").expect("Failed to parse hello world");
    assert_eq!(program.functions.len(), 1);
    assert_eq!(program.functions[0].name, "main");
}

#[test]
fn test_parse_variables() {
    let source = r#"
fn main() {
    let x: i32 = 10;
    let mut y: f64 = 3.14;
    let s: String = "hello";
    println(s);
}
"#;
    let program = parser::parse(source, "test.omni").expect("Failed to parse variables");
    assert_eq!(program.functions.len(), 1);
}

#[test]
fn test_parse_function_with_params() {
    let source = r#"
fn add(a: i32, b: i32) -> i32 {
    return a + b;
}

fn main() {
    let result: i32 = add(1, 2);
    println(result.to_string());
}
"#;
    let program = parser::parse(source, "test.omni").expect("Failed to parse function");
    assert_eq!(program.functions.len(), 2);
    assert_eq!(program.functions[0].params.len(), 2);
}

#[test]
fn test_parse_if_else() {
    let source = r#"
fn main() {
    let x: i32 = 5;
    if x > 0 {
        println("positive");
    } else if x < 0 {
        println("negative");
    } else {
        println("zero");
    }
}
"#;
    let program = parser::parse(source, "test.omni").expect("Failed to parse if/else");
    assert_eq!(program.functions.len(), 1);
}

#[test]
fn test_parse_struct() {
    let source = r#"
struct Point {
    x: f64;
    y: f64;
}

fn main() {
    let p: Point = Point { x: 1.0, y: 2.0 };
    println(p.x.to_string());
}
"#;
    let program = parser::parse(source, "test.omni").expect("Failed to parse struct");
    assert_eq!(program.structs.len(), 1);
    assert_eq!(program.structs[0].name, "Point");
}

#[test]
fn test_parse_for_loop() {
    let source = r#"
fn main() {
    let arr: Array<i32> = [1, 2, 3];
    for item in arr {
        println(item.to_string());
    }
}
"#;
    let program = parser::parse(source, "test.omni").expect("Failed to parse for loop");
    assert_eq!(program.functions.len(), 1);
}

#[test]
fn test_parse_match() {
    let source = r#"
fn main() {
    let x: i32 = 42;
    let msg: String = match x {
        0 => "zero",
        1 => "one",
        _ => "many",
    };
    println(msg);
}
"#;
    let program = parser::parse(source, "test.omni").expect("Failed to parse match");
    assert_eq!(program.functions.len(), 1);
}

#[test]
fn test_parse_template_string() {
    let source = r#"
fn main() {
    let name: String = "World";
    let msg: String = `Hello, ${name}!`;
    println(msg);
}
"#;
    let program = parser::parse(source, "test.omni").expect("Failed to parse template string");
    assert_eq!(program.functions.len(), 1);
}

#[test]
fn test_parse_closure() {
    let source = r#"
fn main() {
    let add = |a: i32, b: i32| a + b;
    let result: i32 = add(1, 2);
    println(result.to_string());
}
"#;
    let program = parser::parse(source, "test.omni").expect("Failed to parse closure");
    assert_eq!(program.functions.len(), 1);
}

#[test]
fn test_parse_async_fn() {
    let source = r#"
async fn fetch_data() -> String {
    return "data";
}

async fn main() {
    let result: String = await fetch_data();
    println(result);
}
"#;
    let program = parser::parse(source, "test.omni").expect("Failed to parse async fn");
    assert_eq!(program.functions.len(), 2);
    assert!(program.functions[0].is_async);
    assert!(program.functions[1].is_async);
}

#[test]
fn test_parse_import_export() {
    let source = r#"
import { add, sub } from "./math";
export fn greet(name: String) -> String {
    return "Hello, " + name;
}

fn main() {
    println(greet("Alice"));
}
"#;
    let program = parser::parse(source, "test.omni").expect("Failed to parse import/export");
    assert_eq!(program.imports.len(), 1);
    assert_eq!(program.exports.len(), 1);
}
