//! Regression tests for unmapped-canonical events (v4 3.3, domain HIGH-1):
//! hover/toggle/longpress are Flutter convenience shapes mixed into the
//! canonical table — they have no DOM event, so OmniView's `map_event`
//! pass-through emitted never-firing listeners
//! (`addEventListener("toggle")`). The fix: warning + skip.

/// Compiles `src` through the public OmniView pipeline and returns the
/// generated JS and the codegen warnings.
fn compile_omniview_src_with_warnings(src: &str) -> (String, Vec<String>) {
    let result = crate::compile_omniview(src, false, false);
    assert!(
        result.success,
        "compilation should succeed: {:?}",
        result.errors
    );
    (result.files.unwrap().js, result.warnings)
}

#[test]
fn hover_toggle_warn_and_skip_on_omniview() {
    // v4 domain HIGH-1: hover/toggle 在 OmniView 必须 warning + 不发射
    // 死监听器（此前 addEventListener("toggle") 永不触发）。
    let (js, warnings) = compile_omniview_src_with_warnings(
        "component App { view { <div @onHover={() => {}} @onToggle={() => {}} /> } }",
    );
    assert!(warnings.len() >= 2, "缺警告：{warnings:?}");
    assert!(
        warnings.iter().any(|w| w.contains("hover")),
        "缺 hover 警告：{warnings:?}"
    );
    assert!(
        warnings.iter().any(|w| w.contains("toggle")),
        "缺 toggle 警告：{warnings:?}"
    );
    assert!(
        !js.contains("addEventListener(\"hover\")"),
        "死监听器仍发射"
    );
    assert!(
        !js.contains("addEventListener(\"toggle\")"),
        "死监听器仍发射"
    );
}

#[test]
fn longpress_warn_and_skip_on_omniview() {
    // Same family: @onLongpress on OmniView must warn + skip too.
    let (js, warnings) = compile_omniview_src_with_warnings(
        "component App { fn hold() {} view { <button @onLongpress={hold}>Hold</button> } }",
    );
    assert!(
        warnings.iter().any(|w| w.contains("longpress")),
        "缺 longpress 警告：{warnings:?}"
    );
    assert!(
        !js.contains("addEventListener(\"longpress\")"),
        "死监听器仍发射"
    );
}

#[test]
fn mapped_events_still_emit_on_omniview() {
    // The guard must not touch mapped canonicals — click/press/input still
    // emit their listeners.
    let (js, warnings) = compile_omniview_src_with_warnings(
        "component App { view { <button @onPress={() => {}}>Go</button> } }",
    );
    assert!(warnings.is_empty(), "映射事件不应警告：{warnings:?}");
    assert!(
        js.contains("addEventListener(\"click\""),
        "click 监听器应发射"
    );
}
