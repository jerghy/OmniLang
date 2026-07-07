//! `omnic create` — scaffold a new OmniLang project.
//! Mirrors Python cmd_create (cli.py:403).

use std::path::Path;

pub fn cmd_create(name: &str, project_type: &str) -> anyhow::Result<()> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        anyhow::bail!("Directory '{name}' already exists.");
    }

    std::fs::create_dir_all(project_dir)?;

    // .omnilang.toml — mirrors Python cli.py:406-417
    let config = format!(
        r#"[project]
name = "{name}"
version = "0.1.0"

[source]
dir = "src"

[output]
dir = "build"
"#
    );
    std::fs::write(project_dir.join(".omnilang.toml"), &config)?;
    println!("  Config: {}/.omnilang.toml", project_dir.display());

    // src/
    let src_dir = project_dir.join("src");
    std::fs::create_dir_all(&src_dir)?;

    // main.omni — mirrors Python LOGIC_TEMPLATE / VIEW_TEMPLATE
    let template = if project_type == "logic" {
        format!(
            "// {name} — OmniLang logic project\n\
             fn main() {{\n    print(\"Hello from {name}!\");\n}}\n"
        )
    } else {
        format!(
            "// {name} — OmniLang view project\n\
             import omniview;\n\n\
             component App {{\n\n\
             \n    view {{\n\n\
             \n        <div class: app>\n\
             \n            <h1>{name}</h1>\n\
             \n            <p>Welcome to your OmniView app!</p>\n\
             \n        </div>\n\
             \n    }}\n\
             \n}}\n"
        )
    };
    std::fs::write(src_dir.join("main.omni"), &template)?;
    println!("  Source: {}/src/main.omni", project_dir.display());

    println!(
        "\nCreated {project_type} project '{name}' at {}",
        std::fs::canonicalize(project_dir)?.display()
    );
    println!("  cd {name}");
    println!("  omnic run src/main.omni");
    Ok(())
}
