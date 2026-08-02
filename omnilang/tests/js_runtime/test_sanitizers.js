// OmniView JS Runtime — sanitizer behavioral tests
//
// The OmniView runtime functions (__omni_sanitizeUrl / __omni_sanitizeCss /
// __omni_normalizeCssEscapes) and the React runtime helper (sanitizeCss /
// normalizeCssEscapes, emitted into useDesignTokens.ts) are both generated
// from ONE shared emitter: omnilang/src/codegen/shared/sanitize.rs
// (task 3.2 — three-copy drift fix). Before that, the React copy had lost
// hex-escape normalization, data:text/html, document./window. and the url()
// context checks, so `\6a\61\76\61\73\63\72\69\70\74\3a alert(1)`
// (hex-escaped "javascript:") bypassed it at runtime.
//
// The copies below mirror the generated output. The shared assertion set at
// the bottom runs the SAME expectations against BOTH copies so the two
// runtimes can never drift apart again.
"use strict";

const assert = require("assert");

// ─── Runtime functions (copied from runtime.rs JS strings) ───

function __omni_sanitizeUrl(url) {
  var str = String(url);
  var trimmed = str.trim().toLowerCase();
  // Strip CR, LF, TAB before scheme matching — browsers (Chromium, WebKit,
  // Gecko) strip these characters from URLs before navigation, so
  // "java\nscript:alert(1)" would execute as "javascript:alert(1)".
  var normalized = trimmed.replace(/[\r\n\t]/g, '');
  // Block all javascript:/vbscript: pseudo-protocols and all data: URIs.
  // data: URIs can carry executable content (text/html, text/javascript,
  // application/javascript, base64 payloads, etc.) so we block the entire
  // scheme rather than maintaining a subtype allowlist.
  if (normalized.indexOf('javascript:') === 0 ||
      normalized.indexOf('data:') === 0 ||
      normalized.indexOf('vbscript:') === 0) {
    return '#blocked';
  }
  return str;
}

function __omni_normalizeCssEscapes(value) {
  // Normalize CSS hex escape sequences like \6a (→j) before pattern matching
  // to prevent attackers from bypassing keyword-based sanitizers.
  var result = '';
  var i = 0;
  var len = value.length;
  while (i < len) {
    var ch = value.charAt(i);
    if (ch === '\\') {
      i++;
      var hex = '';
      while (i < len && hex.length < 6 && /^[0-9a-fA-F]$/.test(value.charAt(i))) {
        hex += value.charAt(i);
        i++;
      }
      if (hex.length > 0) {
        // Consume optional trailing space (CSS spec)
        if (i < len && value.charAt(i) === ' ') {
          i++;
        }
        var code = parseInt(hex, 16);
        // String.fromCodePoint throws RangeError for code points > 0x10FFFF
        // and surrogates (0xD800-0xDFFF). Skip these — they cannot represent
        // valid characters and preserving the raw escape is safer than crashing.
        // Matches Rust behavior: char::from_u32(code) returns None for these.
        if (!isNaN(code) && code <= 0x10FFFF && !(code >= 0xD800 && code <= 0xDFFF)) {
          result += String.fromCodePoint(code);
          continue;
        }
      }
      // Not a valid hex escape, preserve backslash and next char
      result += '\\';
      if (i < len) {
        result += value.charAt(i);
        i++;
      }
    } else {
      result += ch;
      i++;
    }
  }
  return result;
}

function __omni_sanitizeCss(value) {
  var str = String(value);
  // Strip CR, LF, TAB before any pattern matching — browsers strip these
  // characters during CSS tokenization (CSS Syntax §4.3.7), so
  // "jav\r\nascript:" would be normalized by the browser to "javascript:".
  // This mirrors the same normalization in __omni_sanitizeUrl.
  str = str.replace(/[\r\n\t]/g, '');
  // Normalize CSS hex escapes before checking for dangerous patterns
  str = __omni_normalizeCssEscapes(str);
  // Strip again: escapes decode AFTER the pre-strip, so \a produces a
  // literal LF (\d a CR, \9 a TAB) here. Without this second strip,
  // "java\ascript:" — decoded to "java\nscript:" — would bypass the
  // keyword checks below. Mirrors sanitize_css_value in sanitize.rs.
  str = str.replace(/[\r\n\t]/g, '');
  var lower = str.toLowerCase();
  // Block protocol-level injection patterns
  if (lower.indexOf('javascript:') >= 0 || lower.indexOf('expression(') >= 0 ||
      lower.indexOf('-moz-binding') >= 0 ||
      lower.indexOf('data:text/html') >= 0 || lower.indexOf('@import') >= 0 ||
      lower.indexOf('vbscript:') >= 0 || lower.indexOf('document.') >= 0 ||
      lower.indexOf('window.') >= 0) {
    return '';
  }
  // Allow url() only with safe schemes. url() is needed for
  // background-image and other standard CSS resource loading.
  // Block url() containing javascript: or vbscript: schemes.
  // The outer block already catches standalone javascript:/vbscript:,
  // but they could appear inside a url() value after stripping.
  var urlIdx = lower.indexOf('url(');
  if (urlIdx >= 0) {
    var afterUrl = lower.substring(urlIdx);
    if (afterUrl.indexOf('javascript:') >= 0 || afterUrl.indexOf('vbscript:') >= 0) {
      return '';
    }
  }
  return str;
}

// ─── React runtime helper copy ───
// Mirrors what react/design.rs emits into useDesignTokens.ts — the SAME
// shared emitter output as the OmniView runtime above, with the React
// function names (type annotations omitted — this file is plain JS).

function normalizeCssEscapes(value) {
  // Normalize CSS hex escape sequences like \6a (→j) before pattern matching
  // to prevent attackers from bypassing keyword-based sanitizers.
  var result = '';
  var i = 0;
  var len = value.length;
  while (i < len) {
    var ch = value.charAt(i);
    if (ch === '\\') {
      i++;
      var hex = '';
      while (i < len && hex.length < 6 && /^[0-9a-fA-F]$/.test(value.charAt(i))) {
        hex += value.charAt(i);
        i++;
      }
      if (hex.length > 0) {
        // Consume optional trailing space (CSS spec)
        if (i < len && value.charAt(i) === ' ') {
          i++;
        }
        var code = parseInt(hex, 16);
        // String.fromCodePoint throws RangeError for code points > 0x10FFFF
        // and surrogates (0xD800-0xDFFF). Skip these — they cannot represent
        // valid characters and preserving the raw escape is safer than crashing.
        // Matches Rust behavior: char::from_u32(code) returns None for these.
        if (!isNaN(code) && code <= 0x10FFFF && !(code >= 0xD800 && code <= 0xDFFF)) {
          result += String.fromCodePoint(code);
          continue;
        }
      }
      // Not a valid hex escape, preserve backslash and next char
      result += '\\';
      if (i < len) {
        result += value.charAt(i);
        i++;
      }
    } else {
      result += ch;
      i++;
    }
  }
  return result;
}

function sanitizeCss(value) {
  var str = String(value);
  // Strip CR, LF, TAB before any pattern matching — browsers strip these
  // characters during CSS tokenization (CSS Syntax §4.3.7), so
  // "jav\r\nascript:" would be normalized by the browser to "javascript:".
  // This mirrors the same normalization in __omni_sanitizeUrl.
  str = str.replace(/[\r\n\t]/g, '');
  // Normalize CSS hex escapes before checking for dangerous patterns
  str = normalizeCssEscapes(str);
  // Strip again: escapes decode AFTER the pre-strip, so \a produces a
  // literal LF (\d a CR, \9 a TAB) here. Without this second strip,
  // "java\ascript:" — decoded to "java\nscript:" — would bypass the
  // keyword checks below. Mirrors sanitize_css_value in sanitize.rs.
  str = str.replace(/[\r\n\t]/g, '');
  var lower = str.toLowerCase();
  // Block protocol-level injection patterns
  if (lower.indexOf('javascript:') >= 0 || lower.indexOf('expression(') >= 0 ||
      lower.indexOf('-moz-binding') >= 0 ||
      lower.indexOf('data:text/html') >= 0 || lower.indexOf('@import') >= 0 ||
      lower.indexOf('vbscript:') >= 0 || lower.indexOf('document.') >= 0 ||
      lower.indexOf('window.') >= 0) {
    return '';
  }
  // Allow url() only with safe schemes. url() is needed for
  // background-image and other standard CSS resource loading.
  // Block url() containing javascript: or vbscript: schemes.
  // The outer block already catches standalone javascript:/vbscript:,
  // but they could appear inside a url() value after stripping.
  var urlIdx = lower.indexOf('url(');
  if (urlIdx >= 0) {
    var afterUrl = lower.substring(urlIdx);
    if (afterUrl.indexOf('javascript:') >= 0 || afterUrl.indexOf('vbscript:') >= 0) {
      return '';
    }
  }
  return str;
}

// ─── Tests: __omni_sanitizeUrl ───

assert.strictEqual(
  __omni_sanitizeUrl('javascript:alert(1)'),
  '#blocked',
  'should block javascript: scheme'
);

assert.strictEqual(
  __omni_sanitizeUrl('data:text/html,<script>alert(1)</script>'),
  '#blocked',
  'should block data: scheme'
);

assert.strictEqual(
  __omni_sanitizeUrl('vbscript:msgbox("xss")'),
  '#blocked',
  'should block vbscript: scheme'
);

assert.strictEqual(
  __omni_sanitizeUrl('JAVASCRIPT:alert(1)'),
  '#blocked',
  'should block JAVASCRIPT: with uppercase'
);

assert.strictEqual(
  __omni_sanitizeUrl(' https://example.com'),
  ' https://example.com',
  'should pass https: URL with leading space (preserves original)'
);

assert.strictEqual(
  __omni_sanitizeUrl('https://example.com/image.png'),
  'https://example.com/image.png',
  'should pass safe https: URL'
);

assert.strictEqual(
  __omni_sanitizeUrl(''),
  '',
  'should pass empty string'
);

// Test: newline/tab/CR in javascript: scheme is blocked
{
  const result = __omni_sanitizeUrl("java\nscript:alert(1)");
  assert.strictEqual(result, "#blocked", "newline in javascript: scheme must be blocked");
}

{
  const result = __omni_sanitizeUrl("java\tscript:alert(1)");
  assert.strictEqual(result, "#blocked", "tab in javascript: scheme must be blocked");
}

{
  const result = __omni_sanitizeUrl("java\rscript:alert(1)");
  assert.strictEqual(result, "#blocked", "CR in javascript: scheme must be blocked");
}

// Safe URL with newline in query string is preserved
{
  const result = __omni_sanitizeUrl("https://example.com/path\nparam");
  assert.strictEqual(result, "https://example.com/path\nparam", "safe URL with newline preserved");
}

// ─── Shared CSS sanitizer assertion set ───
// The SAME expectations must hold for the OmniView runtime copy AND the
// React runtime helper copy. Any divergence is a sanitizer drift bug.

const sanitizeCopies = [
  { name: 'OmniView __omni_sanitizeCss', fn: __omni_sanitizeCss },
  { name: 'React sanitizeCss', fn: sanitizeCss },
];

const cssCases = [
  // Keyword blocks
  ['javascript:alert(1)', '', 'block javascript: in CSS values'],
  ['expression(alert(1))', '', 'block expression() in CSS'],
  ['-moz-binding:url("xss")', '', 'block -moz-binding in CSS'],
  ['data:text/html,<script>', '', 'block data:text/html in CSS'],
  ['@import url("evil.css")', '', 'block @import in CSS'],
  ['vbscript:msgbox(1)', '', 'block vbscript: in CSS'],
  ['document.cookie', '', 'block document. in CSS'],
  ['window.location', '', 'block window. in CSS'],
  // url() with safe schemes should be allowed
  ['url(https://example.com/bg.png)', 'url(https://example.com/bg.png)',
   'allow url() with https scheme'],
  ['url("/images/background.png")', 'url("/images/background.png")',
   'allow url() with relative path'],
  // url() with javascript: inside should be blocked
  ['url(javascript:alert(1))', '', 'block url() with javascript: scheme'],
  ["background-image: url('javascript:alert(1)')", '',
   'block url() with javascript: scheme in full property'],
  // Safe values pass through
  ['10px solid red', '10px solid red', 'pass safe CSS value'],
  ['#ffffff', '#ffffff', 'pass hex color'],
  ['', '', 'pass empty string'],
  // CSS hex-escape bypass payload: \6a\61\76\61\73\63\72\69\70\74\3a encodes
  // "javascript:". This payload pierced the React copy before the shared
  // helper (three-copy drift). Note: CSS hex escapes greedily consume
  // following hex chars, so the space after \3a is consumed per the CSS spec
  // to produce clean "javascript:".
  ['\\6a\\61\\76\\61\\73\\63\\72\\69\\70\\74\\3a alert(1)', '',
   'block CSS hex-escaped javascript: bypass payload'],
  // Normalize function with safe hex-encoded values
  ['background\\2d color: red', 'background-color: red',
   'normalize hex-encoded safe CSS (\\2d -> -)'],
  // Hex-encoded "javascript" without colon — safe because no "javascript:" match
  ['\\6a\\61\\76\\61\\73\\63\\72\\69\\70\\74', 'javascript',
   'decode hex but pass javascript without colon'],
  // Valid hex escape still works
  ['color:\\20 red', 'color: red', 'valid hex escape still works'],
  // Escape-decoded whitespace (review LOW): \a decodes to LF, \d to CR,
  // \9 to TAB — AFTER the pre-strip, so the post-normalize strip must catch
  // them or "java\ascript:" (decoded "java\nscript:") bypasses the keywords.
  ['url(java\\ascript:alert(1))', '', 'block \\a-decoded LF in url() javascript:'],
  ['url(java\\dscript:alert(1))', '', 'block \\d-decoded CR in url() javascript:'],
  ['url(java\\9script:alert(1))', '', 'block \\9-decoded TAB in url() javascript:'],
];

for (const { name, fn } of sanitizeCopies) {
  for (const [input, expected, label] of cssCases) {
    assert.strictEqual(fn(input), expected, `[${name}] ${label}`);
  }
}

// Out-of-range (0x110000 > 0x10FFFF) and surrogate (0xD800) hex escapes must
// not throw through the sanitizer (String.fromCodePoint would RangeError).
const noThrowInputs = ['\\110000', 'background:\\D800'];
for (const { name, fn } of sanitizeCopies) {
  for (const input of noThrowInputs) {
    let threw = null;
    try {
      fn(input);
    } catch (e) {
      threw = e;
    }
    assert.strictEqual(threw, null, `[${name}] must not throw for ${JSON.stringify(input)}`);
  }
}

// ─── Normalize function edge cases (direct) ───
// Both runtimes expose the normalize helper (shared emitter output).

const normalizeCopies = [
  { name: 'OmniView __omni_normalizeCssEscapes', fn: __omni_normalizeCssEscapes },
  { name: 'React normalizeCssEscapes', fn: normalizeCssEscapes },
];

for (const { name, fn } of normalizeCopies) {
  // Out-of-range hex escape must not throw RangeError
  assert.strictEqual(typeof fn('\\110000'), 'string',
    `[${name}] out-of-range hex escape must not throw RangeError`);
  // Surrogate hex escape must not throw RangeError
  assert.strictEqual(typeof fn('background:\\D800'), 'string',
    `[${name}] surrogate hex escape must not throw RangeError`);
}

console.log('  ✓ All sanitizer tests passed');
