// OmniView JS Runtime — sanitizer behavioral tests
// These test the exact functions emitted by omnilang's runtime.rs.
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
  // Normalize CSS hex escapes before checking for dangerous patterns
  str = __omni_normalizeCssEscapes(str);
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

// ─── Tests: __omni_sanitizeCss ───

assert.strictEqual(
  __omni_sanitizeCss('javascript:alert(1)'),
  '',
  'should block javascript: in CSS values'
);

assert.strictEqual(
  __omni_sanitizeCss('expression(alert(1))'),
  '',
  'should block expression() in CSS'
);

assert.strictEqual(
  __omni_sanitizeCss('-moz-binding:url("xss")'),
  '',
  'should block -moz-binding in CSS'
);

assert.strictEqual(
  __omni_sanitizeCss('data:text/html,<script>'),
  '',
  'should block data:text/html in CSS'
);

assert.strictEqual(
  __omni_sanitizeCss('@import url("evil.css")'),
  '',
  'should block @import in CSS'
);

assert.strictEqual(
  __omni_sanitizeCss('vbscript:msgbox(1)'),
  '',
  'should block vbscript: in CSS'
);

assert.strictEqual(
  __omni_sanitizeCss('document.cookie'),
  '',
  'should block document. in CSS'
);

assert.strictEqual(
  __omni_sanitizeCss('window.location'),
  '',
  'should block window. in CSS'
);

// url() with safe schemes should be allowed
assert.strictEqual(
  __omni_sanitizeCss('url(https://example.com/bg.png)'),
  'url(https://example.com/bg.png)',
  'should allow url() with https scheme'
);

assert.strictEqual(
  __omni_sanitizeCss('url("/images/background.png")'),
  'url("/images/background.png")',
  'should allow url() with relative path'
);

// url() with javascript: inside should be blocked
assert.strictEqual(
  __omni_sanitizeCss('url(javascript:alert(1))'),
  '',
  'should block url() with javascript: scheme'
);

assert.strictEqual(
  __omni_sanitizeCss("background-image: url('javascript:alert(1)')"),
  '',
  'should block url() with javascript: scheme in full property'
);

// Safe values pass through
assert.strictEqual(
  __omni_sanitizeCss('10px solid red'),
  '10px solid red',
  'should pass safe CSS value'
);

assert.strictEqual(
  __omni_sanitizeCss('#ffffff'),
  '#ffffff',
  'should pass hex color'
);

assert.strictEqual(
  __omni_sanitizeCss(''),
  '',
  'should pass empty string'
);

// CSS escape bypass: an attacker could use \6a\61\76\61\73\63\72\69\70\74\3a
// to encode "javascript:" and bypass the keyword check.
// Note: CSS hex escapes greedily consume following hex chars, so "alert(1)"
// would eat \3a's 'a'. We use a space after \3a (per CSS spec, trailing
// whitespace after hex escapes is consumed) to produce clean "javascript:".
assert.strictEqual(
  __omni_sanitizeCss('\\6a\\61\\76\\61\\73\\63\\72\\69\\70\\74\\3a alert(1)'),
  '',
  'should block CSS hex-escaped javascript:'
);

// Test normalize function with safe hex-encoded values
assert.strictEqual(
  __omni_sanitizeCss('background\\2d color: red'),
  'background-color: red',
  'should normalize hex-encoded safe CSS (\\2d -> -)'
);

// Test hex-encoded "javascript" without colon — safe because no "javascript:" match
assert.strictEqual(
  __omni_sanitizeCss('\\6a\\61\\76\\61\\73\\63\\72\\69\\70\\74'),
  'javascript',
  'should decode hex but pass javascript without colon'
);

// ─── Tests: newline/tab/CR bypass ───

// Test: newline in javascript: scheme is blocked
{
  const result = __omni_sanitizeUrl("java\nscript:alert(1)");
  assert.strictEqual(result, "#blocked", "newline in javascript: scheme must be blocked");
}

// Test: tab in javascript: scheme is blocked
{
  const result = __omni_sanitizeUrl("java\tscript:alert(1)");
  assert.strictEqual(result, "#blocked", "tab in javascript: scheme must be blocked");
}

// Test: CR in javascript: scheme is blocked
{
  const result = __omni_sanitizeUrl("java\rscript:alert(1)");
  assert.strictEqual(result, "#blocked", "CR in javascript: scheme must be blocked");
}

// Test: safe URL with newline in query string is preserved
{
  const result = __omni_sanitizeUrl("https://example.com/path\nparam");
  assert.strictEqual(result, "https://example.com/path\nparam", "safe URL with newline preserved");
}

// Test: out-of-range CSS hex escape (0x110000 > 0x10FFFF) does not throw
{
  // String.fromCodePoint(0x110000) would throw RangeError. The normalize
  // function must skip invalid code points rather than crashing.
  const result = __omni_normalizeCssEscapes("\\110000");
  assert.strictEqual(typeof result, "string", "out-of-range hex escape must not throw RangeError");
}

// Test: surrogate hex escape (0xD800) does not throw
{
  const result = __omni_normalizeCssEscapes("background:\\D800");
  assert.strictEqual(typeof result, "string", "surrogate hex escape must not throw RangeError");
}

// Test: valid hex escape still works after range check addition
{
  const result = __omni_sanitizeCss("color:\\20 red");
  assert.strictEqual(result, "color: red", "valid hex escape must still work");
}

console.log('  ✓ All sanitizer tests passed');
