#[cfg(feature = "codegen-omniview")]
#[test]
fn test_resolve_styles_warns_unknown_class() {
    // class: card without any design block or style block → should warn
    let source = r#"import omniview;

component main {
    view {
        <div id="main.view">
            <p id="p1" class: card label="Text"></p>
            <p id="p2" class: unknownClass label="Unknown"></p>
        </div>
    }
}
"#;
    let result = omnilang::resolve_styles(source);
    println!("warnings: {:?}", result.warnings);
    println!("element_classes: {:?}", result.element_classes);

    // Should have warnings for unknown class references
    assert!(
        !result.warnings.is_empty(),
        "Expected warnings for unknown class 'card' and 'unknownClass'"
    );
}

#[cfg(feature = "codegen-omniview")]
#[test]
fn test_resolve_styles_with_style_block_class() {
    // class referencing a class from the component's style block → no warning
    let source = r#"import omniview;

component main {
    view {
        <div class: wrapper>
            <p class: title>Hello</p>
        </div>
    }
    style {
        .wrapper { padding: "16px"; }
        .title { font-size: "20px"; }
    }
}
"#;
    let result = omnilang::resolve_styles(source);
    println!("warnings: {:?}", result.warnings);

    assert!(
        result.warnings.is_empty(),
        "Unexpected warnings: {:?}",
        result.warnings
    );
}

#[cfg(feature = "codegen-omniview")]
#[test]
fn test_resolve_styles_with_design_class() {
    let source = r##"import omniview;

design AppDesign {
    class Card {
        padding: "16px";
        background: "#fff";
    }
}

component main {
    view {
        <div class: Card>
            <p>Hello</p>
        </div>
    }
}
"##;
    let result = omnilang::resolve_styles(source);
    println!("warnings: {:?}", result.warnings);
    println!("design_css len: {}", result.design_css.len());

    assert!(
        result.warnings.is_empty(),
        "Unexpected warnings: {:?}",
        result.warnings
    );
    assert!(!result.design_css.is_empty(), "Expected design CSS");
}

#[cfg(feature = "codegen-omniview")]
#[test]
fn test_resolve_styles_variant_warning() {
    let source = r##"import omniview;

design AppDesign {
    class Card {
        padding: "16px";
    }
}

component main {
    view {
        <button variant: AppDesign.Button(color=primary)>Click</button>
    }
}
"##;
    let result = omnilang::resolve_styles(source);
    println!("warnings: {:?}", result.warnings);

    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.contains("variant") || w.contains("Button")),
        "Expected variant warning, got: {:?}",
        result.warnings
    );
}

/// 评审 systems-architect MEDIUM-1：画布 success 曾把 warnings 计入——
/// B2 后未知属性动态绑定产生 warning，同一输入 CLI 构建成功而画布
/// 显示失败。success 必须只按 errors 门控（与编译路径一致）。
#[cfg(feature = "codegen-omniview")]
#[test]
fn test_resolve_styles_warning_does_not_flip_success() {
    // 裸未知属性 + 动态值 → B2 warning（unknown attribute 'foo'），
    // 但无 error——画布必须报 success。
    let source = r#"import omniview;

component App {
  signal count: i32 = 0;
  view {
    <div foo={count}></div>
  }
}
"#;
    let result = omnilang::resolve_styles(source);
    assert!(
        !result.warnings.is_empty(),
        "fixture 必须产生 warning（未知属性动态绑定），got: {:?}",
        result.warnings
    );
    assert!(
        result.success,
        "仅 warning 无 error 时画布 success 必须为 true，warnings: {:?}",
        result.warnings
    );
}
