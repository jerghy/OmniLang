/// ── Editor Bridge ──
window.__omni_signals = {};
window.__omni_components = {};

function __omni_buildNidMap() {
  var map = {};
  document.querySelectorAll('[data-nid]').forEach(function(el) {
    map[el.dataset.nid] = {
      elementId: el.id || '',
      tag: el.tagName.toLowerCase(),
      component: (el.closest('[data-component]') || {}).dataset ? el.closest('[data-component]').dataset.component : '',
    };
  });
  return map;
}

document.addEventListener('click', function(e) {
  var el = e.target.closest('[data-nid]');
  if (el) {
    e.stopPropagation();
    parent.postMessage({ type: 'omni-select', nid: el.dataset.nid }, window.location.origin);
  }
});

window.addEventListener('message', function(e) {
  if (e.origin !== window.location.origin) return;
  switch (e.data.type) {
    case 'omni-replace-component':
      __omni_replaceComponent(e.data.name, e.data.js);
      break;
    case 'omni-highlight':
      __omni_editor.highlight(e.data.nid);
      break;
    case 'omni-clear-hl':
      __omni_editor.clearHighlight();
      break;
  }
});

window.__omni_editor = {
  highlight: function(nid) {
    var el = document.querySelector('[data-nid="' + nid + '"]');
    if (el) { el.style.outline = '2px solid #4F46E5'; }
  },
  clearHighlight: function() {
    document.querySelectorAll('[data-nid]').forEach(function(el) {
      el.style.outline = '';
    });
  },
};

function __omni_replaceComponent(name, factoryCode) {
  // Defense-in-depth: validate factoryCode is a function expression before eval.
  // The primary protection is the postMessage origin check, but this
  // guards against same-origin XSS in the parent editor page sending
  // a crafted message with non-function js payload.
  if (typeof factoryCode !== 'string' || !/^\s*function\s*\(/.test(factoryCode)) {
    __omni_reportError('warn', 'editor', 'Invalid factory code: expected function expression');
    return;
  }
  var oldComp = window.__omni_components[name];
  var savedSignals = {};
  if (oldComp && oldComp.signals) {
    Object.keys(oldComp.signals).forEach(function(k) {
      savedSignals[k] = oldComp.signals[k].peek();
    });
  }
  try {
    window.__omni_components[name] = (0, eval)(factoryCode);
  } catch(e) {
    try { parent.postMessage({ type: 'omni-replace-error', name: name, error: String(e) }, window.location.origin); } catch(_) {}
    if (oldComp) window.__omni_components[name] = oldComp;
    return;
  }
  if (oldComp && oldComp.teardown) oldComp.teardown();
  if (__omni_el._cache) __omni_el._cache = {};
  var newComp = window.__omni_components[name];
  if (newComp && newComp.signals) {
    Object.keys(savedSignals).forEach(function(k) {
      if (newComp.signals[k]) newComp.signals[k].set(savedSignals[k]);
    });
  }
  parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, window.location.origin);
}

parent.postMessage({ type: 'omni-rendered', nidMap: __omni_buildNidMap() }, window.location.origin);
