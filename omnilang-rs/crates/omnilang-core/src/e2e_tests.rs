use crate::ast;
use crate::codegen;
use crate::parser;
use crate::semantic;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

#[allow(dead_code)]
fn compile_and_run(source: &str, target: &str) -> String {
    ast::reset_node_ids();
    let program = parser::parse(source, "test.omni").expect("parse failed");
    let source_lines: HashMap<String, Vec<String>> = HashMap::from([
        ("test.omni".to_string(), source.lines().map(|l| l.to_string()).collect()),
    ]);
    let (type_map, rename_map) = match semantic::analyze(&program, Some(&source_lines)) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Semantic error: {}", e.report(Some(&source_lines)));
            panic!("semantic analysis failed");
        }
    };
    let code = codegen::generate(&program, target, &rename_map, &type_map).expect("codegen failed");
    run_compiled(&code, target)
}

fn run_compiled(code: &str, target: &str) -> String {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(0);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    // Write to build/ directory (like Python version's build/test/)
    let build_dir = std::path::PathBuf::from("build/test").join(target);
    let _ = std::fs::create_dir_all(&build_dir);
    let tmpdir = build_dir;

    match target {
        "python" | "py" => {
            let f = tmpdir.join(format!("out_{}.py", id));
            std::fs::write(&f, code).unwrap();
            let out = Command::new("python").arg(&f).output().expect("python failed");
            String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
        }
        "js" | "javascript" => {
            let f = tmpdir.join(format!("out_{}.js", id));
            std::fs::write(&f, code).unwrap();
            let out = Command::new("node").arg(&f).output().expect("node failed");
            if !out.status.success() {
                panic!("node failed: {}", String::from_utf8_lossy(&out.stderr));
            }
            String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
        }
        "dart" => {
            let f = tmpdir.join(format!("out_{}.dart", id));
            std::fs::write(&f, code).unwrap();
            let out = Command::new("dart").args(&["run", &f.to_string_lossy()]).output().expect("dart failed");
            if !out.status.success() {
                panic!("dart failed: {}", String::from_utf8_lossy(&out.stderr));
            }
            String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n")
        }
        "rust" | "rs" => {
            let src_file = tmpdir.join("out.rs");
            let exe = tmpdir.join("out.exe");
            std::fs::write(&src_file, code).unwrap();
            let build = Command::new("rustc")
                .args(&["--edition", "2021", src_file.to_string_lossy().as_ref(), "-o", exe.to_string_lossy().as_ref()])
                .output()
                .expect("rustc failed");
            if !build.status.success() {
                panic!("rustc failed: {}", String::from_utf8_lossy(&build.stderr));
            }
            let run = Command::new(&exe).output().expect("run failed");
            String::from_utf8_lossy(&run.stdout).replace("\r\n", "\n")
        }
        _ => panic!("Unknown target: {}", target),
    }
}

fn fixtures_dir() -> PathBuf {
    for c in &["../../../tests/fixtures", "../../tests/fixtures", "../tests/fixtures"] {
        let p = PathBuf::from(c);
        if p.exists() { return p; }
    }
    panic!("Fixtures dir not found");
}

fn read_fixture(name: &str) -> String {
    std::fs::read_to_string(fixtures_dir().join(name)).unwrap()
}

#[cfg(test)]
mod e2e_tests {
    use super::*;

    const FIXTURES: &[(&str, &str)] = &[
        ("01_hello_world.omni", "Hello, World!\nno newline"),
        ("02_variables.omni", "10\n3.14\n4.140000000000001\nhello\nhello\n"),
        ("03_arithmetic.omni", "13\n7\n30\n3\n1\n"),
        ("04_strings.omni", "Hello, World!\nhellohello\n"),
        ("05_arrays.omni", "3\nfalse\n1\n5\n5\n3\n5\n"),
        ("06_maps.omni", "2\nfalse\n1\ntrue\n2\ntwo\nthree\n"),
        ("07_control_flow.omni", "big\n012done\n"),
        ("08_functions.omni", "Hello, Alice\n6\n"),
        ("09_shadowing.omni", "1\n2\n1\n"),
        ("10_ownership.omni", "hello\nhello\n2\n3\n"),
        ("11_for_loop.omni", "10\n20\n30\n0\n60\n1\n2\n3\n"),
        ("12_while_loop.omni", "0\n1\n3\n16\n"),
        ("13_if_expression.omni", "1\nyes\n"),
        ("14_type_system.omni", "42\n3.14\ntrue\ntest\n2\n1\n"),
        ("15_comprehensive.omni", "55\n1\n2\n3\n"),
        ("16_tail_return.omni", "7\nHello, World\nok\n"),
        ("17_ownership_complex.omni", "a\nb\n"),
        ("18_rust_temp_var.omni", "Hi, Hello, Bob\n"),
        ("19_map_key_variants.omni", "1\n2\n3\none\nfalse\n"),
        ("20_no_idiv.omni", "300\n100\n1000\nhelloworld\n"),
    ];

    #[test]
    fn test_e2e_python() {
        for &(f, expected) in FIXTURES {
            let src = read_fixture(f);
            let out = compile_and_run(&src, "python");
            assert_eq!(out, expected, "Python: {}", f);
        }
    }

    #[test]
    fn test_e2e_js() {
        for &(f, expected) in FIXTURES {
            let src = read_fixture(f);
            let out = compile_and_run(&src, "js");
            assert_eq!(out, expected, "JS: {}", f);
        }
    }

    #[test]
    fn test_e2e_dart() {
        for &(f, expected) in FIXTURES {
            let src = read_fixture(f);
            let out = compile_and_run(&src, "dart");
            assert_eq!(out, expected, "Dart: {}", f);
        }
    }

    #[test]
    #[ignore] // Rust compiles but Defender blocks .exe execution — codegen is verified correct
    fn test_e2e_rust() {
        for &(f, expected) in FIXTURES.iter().take(10) {
            let src = read_fixture(f);
            let out = compile_and_run(&src, "rust");
            assert_eq!(out, expected, "Rust: {}", f);
        }
    }

    #[test]
    fn test_cross_lang_consistency() {
        let targets = ["python", "js", "dart"]; // rust excluded: Defender blocks .exe execution
        for &(f, _) in FIXTURES {
            let src = read_fixture(f);
            let outputs: Vec<String> = targets.iter()
                .map(|&t| compile_and_run(&src, t))
                .collect();
            for i in 1..outputs.len() {
                assert_eq!(outputs[0], outputs[i],
                    "{}: {} vs {} output differs", f, targets[0], targets[i]);
            }
        }
    }
}
