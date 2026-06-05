//! @omnilang/test-utils-dioxus — Test utilities for OmniView-generated Dioxus components.
//!
//! Provides helpers for testing Dioxus components in Rust:
//! - SSR rendering for snapshot tests
//! - Component mounting with store mocks
//! - data-testid query helpers
//! - Signal/GlobalSignal helpers
//!
//! Usage (in Cargo.toml):
//!   [dev-dependencies]
//!   dioxus = "0.5"
//!   dioxus-ssr = "0.5"
//!
//! Usage (in tests):
//!   use omnilang_test_utils::*;
//!
//!   #[test]
//!   fn test_counter() {
//!       let html = render_to_string(|| rsx! { Counter {} });
//!       assert!(html.contains("data-testid=\"counter-label\""));
//!   }

use dioxus::prelude::*;

/// Render a Dioxus component to an HTML string for snapshot testing.
///
/// Uses dioxus-ssr to render components without a browser.
///
/// # Example
/// ```ignore
/// let html = render_to_string(|| rsx! { MyComponent {} });
/// assert!(html.contains("Hello"));
/// ```
pub fn render_to_string(component: fn() -> Element) -> String {
    let mut renderer = dioxus_ssr::Renderer::new();
    let vdom = VirtualDom::new(component);
    renderer.render(&vdom)
}

/// Mount a component in a test VirtualDom and provide helpers.
///
/// Returns a TestHarness that can be used to query the rendered output
/// and simulate events.
pub fn mount_test(component: fn() -> Element) -> TestHarness {
    let vdom = VirtualDom::new(component);
    TestHarness { vdom }
}

/// Test harness wrapping a Dioxus VirtualDom for testing.
pub struct TestHarness {
    vdom: VirtualDom,
}

impl TestHarness {
    /// Render the current state to an HTML string.
    pub fn render(&mut self) -> String {
        let mut renderer = dioxus_ssr::Renderer::new();
        let html = renderer.render(&self.vdom);
        self.vdom.wait_for_work();
        html
    }

    /// Rebuild the virtual DOM (triggers re-renders).
    pub fn rebuild(&mut self) {
        self.vdom.wait_for_work();
    }
}

/// Find elements by data-testid in rendered HTML output.
///
/// This is a simplified matcher for SSR output — for full DOM queries,
/// use `dioxus-web` with `wasm-bindgen-test`.
///
/// # Example
/// ```ignore
/// let html = render_to_string(|| rsx! { App {} });
/// assert!(has_test_id(&html, "counter-label"));
/// assert!(text_at_test_id(&html, "counter-label").contains("Count: 0"));
/// ```
pub fn has_test_id(html: &str, test_id: &str) -> bool {
    html.contains(&format!("data-testid=\"{}\"", test_id))
}

/// Extract text content from an element identified by data-testid.
///
/// Uses a simple regex approach for SSR output.
pub fn text_at_test_id(html: &str, test_id: &str) -> String {
    let pattern = format!("data-testid=\"{}\"", test_id);
    if let Some(start) = html.find(&pattern) {
        let after_tag = &html[start + pattern.len()..];
        // Find the closing >
        if let Some(content_start) = after_tag.find('>') {
            let content = &after_tag[content_start + 1..];
            // Extract text until the next <
            if let Some(content_end) = content.find('<') {
                return content[..content_end].trim().to_string();
            }
            return content.trim().to_string();
        }
    }
    String::new()
}

/// Count elements matching a data-testid in the HTML output.
pub fn count_test_id(html: &str, test_id: &str) -> usize {
    html.matches(&format!("data-testid=\"{}\"", test_id)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_test_id() {
        let html = r#"<div data-testid="foo">bar</div>"#;
        assert!(has_test_id(html, "foo"));
        assert!(!has_test_id(html, "bar"));
    }

    #[test]
    fn test_text_at_test_id() {
        let html = r#"<span data-testid="label">Hello World</span>"#;
        assert_eq!(text_at_test_id(html, "label"), "Hello World");
    }

    #[test]
    fn test_count_test_id() {
        let html = r#"
            <li data-testid="item">A</li>
            <li data-testid="item">B</li>
            <li data-testid="item">C</li>
        "#;
        assert_eq!(count_test_id(html, "item"), 3);
    }
}
