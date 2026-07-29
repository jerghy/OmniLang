// OmniView JS Runtime — sanitizer behavioral tests
// These test the exact functions emitted by omnilang's runtime.rs.
"use strict";

const assert = require("assert");

// ─── Runtime functions (copied from runtime.rs JS strings) ───

function __omni_sanitizeUrl(url) {
  var str = String(url);
  var trimmed = str.trim().toLowerCase();
  // Block all javascript:/vbscript: pseudo-protocols and all data: URIs.
  // data: URIs can carry executable content (text/html, text/javascript,
  // application/javascript, base64 payloads, etc.) so we block the entire
  // scheme rather than maintaining a subtype allowlist.
  if (trimmed.indexOf('javascript:') === 0 ||
      trimmed.indexOf('data:') === 0 ||
      trimmed.indexOf('vbscript:') === 0) {
    return '#blocked';
  }
  return str;
}

function __omni_sanitizeCss(value) {
  var str = String(value);
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
// TODO: Uncomment after adding normalizeCssEscapes in TASK 11.
// assert.strictEqual(
//   __omni_sanitizeCss('\\6a\\61\\76\\61\\73\\63\\72\\69\\70\\74\\3aalert(1)'),
//   '',
//   'should block CSS hex-escaped javascript:'
// );

console.log('  ✓ All sanitizer tests passed');
