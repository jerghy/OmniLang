use super::{StdlibFn, ALL_TARGETS};
use crate::codegen::Target;

pub(super) fn register() -> Vec<StdlibFn> {
    vec![
        StdlibFn {
            module: "http",
            name: "http_send",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_http_send,
            is_async: true,
        },
        StdlibFn {
            module: "http",
            name: "http_get",
            min_args: 1,
            max_args: Some(1),
            available_targets: &ALL_TARGETS,
            codegen: codegen_http_get,
            is_async: true,
        },
        StdlibFn {
            module: "http",
            name: "http_post",
            min_args: 1,
            max_args: Some(2),
            available_targets: &ALL_TARGETS,
            codegen: codegen_http_post,
            is_async: true,
        },
    ]
}

pub(super) fn codegen_http_send(args: &[String], target: Target) -> String {
    let req = &args[0]; // HttpRequest struct variable
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            // fetch() returns a Response; check ok before extracting body
            format!(
                "fetch({req}.url, {{ method: {req}.method, headers: {req}.headers, body: {req}.body, signal: AbortSignal.timeout({req}.timeout) }}).then(r => {{ if (!r.ok) throw new Error(`HTTP ${{r.status}}`); return r.text(); }})",
                req = req
            )
        }
        Target::Python => {
            format!(
                "requests.request({req}.get('method'), {req}.get('url'), headers={req}.get('headers'), params={req}.get('query'), data={req}.get('body'), timeout={req}.get('timeout', 0)/1000)",
                req = req
            )
        }
        Target::Rust | Target::Dioxus => {
            format!(
                "reqwest::Client::new().request(reqwest::Method::from_str(&{req}.method).unwrap_or(reqwest::Method::GET), &{req}.url).headers({req}.headers.clone()).query(&{req}.query).body({req}.body.clone()).timeout(std::time::Duration::from_millis({req}.timeout as u64)).send()",
                req = req
            )
        }
        Target::Dart | Target::Flutter => {
            // The language's http_request returns the response TEXT (JS
            // fetch .text()); Dart's StreamedResponse carries the body on
            // its stream — await the send, then read the bytes (Task 5.8).
            format!(
                "(await (await http.Request({req}.method, Uri.parse({req}.url)).send()).stream.bytesToString())",
                req = req
            )
        }
    }
}

pub(super) fn codegen_http_get(args: &[String], target: Target) -> String {
    let url = &args[0];
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            // fetch() returns a Response; check ok before extracting body
            format!("fetch({url}).then(r => {{ if (!r.ok) throw new Error(`HTTP ${{r.status}}`); return r.text(); }})", url = url)
        }
        Target::Python => format!("requests.get({url})", url = url),
        Target::Rust | Target::Dioxus => {
            format!("reqwest::blocking::get({url})", url = url)
        }
        Target::Dart | Target::Flutter => {
            // http.get returns a Response — the language's http_get returns
            // the response TEXT (JS fetch .text()) — read the body
            // (Task 5.8: raw = await http.get(...) assigned a Response to
            // a String).
            format!("(await http.get(Uri.parse({url}))).body", url = url)
        }
    }
}

pub(super) fn codegen_http_post(args: &[String], target: Target) -> String {
    let url = &args[0];
    let body = args.get(1).map(|s| s.as_str()).unwrap_or("null");
    match target {
        Target::JavaScript
        | Target::TypeScript
        | Target::React
        | Target::Vue
        | Target::OmniView => {
            // fetch() returns a Response; check ok before extracting body
            format!(
                "fetch({url}, {{ method: 'POST', body: {body} }}).then(r => {{ if (!r.ok) throw new Error(`HTTP ${{r.status}}`); return r.text(); }})",
                url = url,
                body = body
            )
        }
        Target::Python => {
            format!("requests.post({url}, data={body})", url = url, body = body)
        }
        Target::Rust | Target::Dioxus => {
            format!(
                "reqwest::blocking::Client::new().post({url}).body({body}).send()",
                url = url,
                body = body
            )
        }
        Target::Dart | Target::Flutter => {
            // http.post returns a Future<Response> already — .send() was
            // an invalid call on the Future; the language's http_post
            // returns the response TEXT.
            format!(
                "(await http.post(Uri.parse({url}), body: {body})).body",
                url = url,
                body = body
            )
        }
    }
}
