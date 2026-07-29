// OmniView JS Runtime — signal system behavioral tests
// These test the exact functions emitted by omnilang's runtime.rs.
"use strict";

const assert = require("assert");

// ─── Runtime functions (copied from runtime.rs JS strings) ───

let __omni_activeEffect = null;
const __omni_subs = new Map();

function __omni_signal(name, init) {
  let val = init;
  const subs = new Set();
  const signal = {
    _default: init,
    get value() {
      if (__omni_activeEffect) {
        subs.add(__omni_activeEffect);
        __omni_activeEffect._deps.add(() => subs.delete(__omni_activeEffect));
      }
      return val;
    },
    set value(v) {
      if (val !== v) {
        val = v;
        const s = new Set(subs);
        s.forEach(e => {
          const prev = __omni_activeEffect;
          __omni_activeEffect = e;
          try { e.run(); }
          catch(err) { __omni_reportError("error", "effect:run", String(err), ""); }
          finally { __omni_activeEffect = prev; }
        });
      }
    },
    get() { return signal.value; },
    set(v) { signal.value = v; },
    peek() { return val; },
  };
  __omni_subs.set(name, signal);
  return signal;
}

function __omni_readonlySignal(name, init) {
  const inner = __omni_signal(name, init);
  return {
    _default: init,
    get value() { return inner.value; },
    get() { return inner.value; },
    peek() { return inner.peek(); },
    // No set value, no set() — child components cannot write
  };
}

function __omni_effect(fn) {
  const e = { _deps: new Set() };
  e.run = () => {
    e._deps.forEach(d => d());
    e._deps.clear();
    fn();
  };
  const prev = __omni_activeEffect;
  __omni_activeEffect = e;
  try { e.run(); } finally { __omni_activeEffect = prev; }
  return () => e._deps.forEach(d => d());
}

function __omni_reportError(context, error) {
  var severity = "error";
  var code = context;
  if (context.indexOf(":") >= 0) {
    var parts = context.split(":");
    var prefix = parts[0];
    if (prefix === "warn" || prefix === "error" || prefix === "fatal") {
      severity = prefix;
      code = parts.slice(1).join(":");
    }
  }
  var msg = "[OmniView] " + severity.toUpperCase() + " " + code + ": " + String(error);
  if (severity === "fatal") { console.error(msg); }
  else if (severity === "warn") { console.warn(msg); }
  else { console.error(msg); }
}

// ─── Tests ───

// Test 1: signal get/set
{
  const count = __omni_signal("count", 0);
  assert.strictEqual(count.value, 0, "initial value should be 0");
  count.value = 5;
  assert.strictEqual(count.value, 5, "after set value = 5");
  count.set(10);
  assert.strictEqual(count.value, 10, "after set(10), value = 10");
  assert.strictEqual(count.get(), 10, "get() returns 10");
  assert.strictEqual(count.peek(), 10, "peek() returns 10");
  assert.strictEqual(count._default, 0, "_default should be initial value");
}

// Test 2: signal with string type
{
  const name = __omni_signal("name", "hello");
  assert.strictEqual(name.value, "hello", "string initial value");
  name.value = "world";
  assert.strictEqual(name.value, "world", "string after set");
}

// Test 3: effect fires on dependency change
{
  const a = __omni_signal("effect_a", 1);
  const b = __omni_signal("effect_b", 2);
  let result = 0;

  const dispose = __omni_effect(() => {
    result = a.value + b.value;
  });

  // Effect should run immediately
  assert.strictEqual(result, 3, "effect should run immediately: 1 + 2 = 3");

  // Change dependency — effect should re-fire
  a.value = 10;
  assert.strictEqual(result, 12, "effect should re-fire after a changes: 10 + 2 = 12");

  b.value = 20;
  assert.strictEqual(result, 30, "effect should re-fire after b changes: 10 + 20 = 30");

  // Cleanup
  dispose();
}

// Test 4: effect does not fire for unrelated signals
{
  const tracked = __omni_signal("tracked", 0);
  const unrelated = __omni_signal("unrelated", 0);
  let callCount = 0;

  const dispose = __omni_effect(() => {
    tracked.value; // read dependency
    callCount++;
  });

  callCount = 0; // Reset after initial run

  unrelated.value = 99;
  assert.strictEqual(callCount, 0, "effect should NOT fire when unrelated signal changes");

  tracked.value = 1;
  assert.strictEqual(callCount, 1, "effect should fire when tracked signal changes");

  dispose();
}

// Test 5: effect exception recovery — an error in one effect
// should not prevent other effects from running.
// The runtime catches errors in the signal setter's notification loop,
// not during __omni_effect's initial run (which uses try/finally without catch).
{
  const s1 = __omni_signal("recovery_s1", -1); // Start at -1 so first effect doesn't throw initially
  const s2 = __omni_signal("recovery_s2", 2);
  let s2Result = 0;

  const s1Effect = () => {
    if (s1.value > 0) {
      throw new Error("test error");
    }
  };

  // Create throwing effect — initial run won't throw because s1 = -1 <= 0
  let dispose1;
  try {
    dispose1 = __omni_effect(s1Effect);
  } catch (_) {
    // ignore initial throw if it happens
  }

  const dispose2 = __omni_effect(() => {
    s2Result = s2.value;
  });

  // Second effect ran initially
  assert.strictEqual(s2Result, 2, "second effect should have initial value");

  // Now make s1 positive so the first effect throws on re-fire
  s1.value = 1;
  // Even though the first effect threw, the second effect should still be registered

  // Change s2 — signal setter catches errors per-effect and continues iterating
  s2.value = 5;
  assert.strictEqual(s2Result, 5, "second effect should update after error in first during notification");

  // Change s1 back to non-throwing value
  s1.value = -1;
  s2.value = 10;
  assert.strictEqual(s2Result, 10, "effect should recover after s1 stops throwing");

  if (dispose1) dispose1();
  dispose2();
}

// Test 6: readonlySignal has no setter
{
  const ro = __omni_readonlySignal("ro", 42);
  assert.strictEqual(ro.value, 42, "readonlySignal initial value");
  assert.strictEqual(ro.get(), 42, "readonlySignal get() works");
  assert.strictEqual(ro.peek(), 42, "readonlySignal peek() works");
  assert.strictEqual(ro._default, 42, "readonlySignal _default");

  // Verify no setter exists
  assert.strictEqual(typeof ro.set, "undefined", "readonlySignal should have no set() method");

  // Setting .value should be a no-op (or throw in strict mode)
  // Since it has no setter, assigning .value silently fails in non-strict mode,
  // but we run in strict mode so it will throw
  assert.throws(() => {
    ro.value = 100;
  }, TypeError, "setting readonlySignal.value should throw in strict mode");
}

// Test 7: signal registers in __omni_subs map
{
  const sig = __omni_signal("test_registry", true);
  assert.ok(__omni_subs.has("test_registry"), "signal should be registered in __omni_subs");
  assert.strictEqual(__omni_subs.get("test_registry"), sig, "signal reference should match");
}

console.log('  ✓ All signal tests passed');
