/**
 * OmniView Runtime — self-contained reactive system for OmniLang-compiled apps.
 *
 * Replaces React + omni-runtime.ts with a native signal-based architecture:
 *   signal(init, meta)   — reactive state (replaces useState)
 *   computed(fn, meta)   — derived values (replaces useMemo)
 *   effect(fn)           — side effects (replaces useEffect)
 *   store(name, fn)      — namespaced signal collection (replaces Zustand)
 *   h(tag, attrs, kids)  — virtual DOM node
 *   render(vnode, el)    — mount DOM tree
 *
 * Debug is BUILT-IN — signal.set() is the ONE interception point for:
 *   state-change reporting → WebSocket → Editor DebugPanel
 *   breakpoint checking    → pause/step/resume
 *
 * Protocol (WebSocket JSON, compatible with existing debug.py/DebugPanel):
 *   App → Editor: state-change, execution-trace, runtime-ready, runtime-error, paused, resumed
 *   Editor → App: set-breakpoint, remove-breakpoint, step, resume, get-state
 */

// ── Types ──────────────────────────────────────────────────

/** Metadata attached to every signal for debug identification. */
export interface SignalMeta {
  id: string;       // "ComponentName.fieldName" — semantic ID
  name: string;     // human-readable name
  type?: string;    // type annotation ("number", "string", etc.)
}

/** A reactive value container. */
export interface Signal<T> {
  get(): T;
  set(v: T | ((prev: T) => T)): void;
  subscribe(fn: (v: T) => void): () => void;
  /** Read WITHOUT registering as a dependency (for effects that shouldn't re-track). */
  peek(): T;
  /** Semantic identifier for debug mapping. */
  readonly id: string;
}

/** A lazily-evaluated derived signal. */
export interface Computed<T> {
  get(): T;
  subscribe(fn: (v: T) => void): () => void;
  peek(): T;
  readonly id: string;
}

/** Returned by effect() and render() — call to dispose. */
export type CleanupFn = () => void;

/** A virtual DOM node produced by h(). */
export interface VNode {
  tag: string;
  attrs: Record<string, unknown>;
  children: (VNode | string | number | null | undefined | Signal<unknown> | Computed<unknown>)[];
  key?: string;
}

/** Distinguish signals from plain objects. */
function isSignalLike(x: unknown): x is Signal<unknown> | Computed<unknown> {
  return typeof x === "object" && x !== null && "subscribe" in (x as Record<string, unknown>);
}

// ── Dependency Auto-Tracking ──────────────────────────────

interface TrackCtx {
  deps: Set<Signal<unknown> | Computed<unknown>>;
}

let activeTracker: TrackCtx | null = null;

function runWithTracker<T>(fn: () => T): { result: T; deps: Set<Signal<unknown> | Computed<unknown>> } {
  const prev = activeTracker;
  const ctx: TrackCtx = { deps: new Set() };
  activeTracker = ctx;
  let result: T;
  try {
    result = fn();
  } finally {
    activeTracker = prev;
  }
  return { result, deps: ctx.deps };
}

function trackDep(s: Signal<unknown> | Computed<unknown>): void {
  if (activeTracker) {
    activeTracker.deps.add(s);
  }
}

// ── Batching ──────────────────────────────────────────────

let batchDepth = 0;
const pendingNotifies = new Set<Signal<unknown> | Computed<unknown>>();


function scheduleNotify(s: SignalImpl<unknown>): void {
  if (batchDepth > 0) {
    pendingNotifies.add(s);
  } else {
    s._notify();
  }
}

// ── DebugBus ──────────────────────────────────────────────

interface PendingEntry {
  signalId: string;
  value: unknown;
  apply: () => void;
}

type WsMessage =
  | { type: "state-change"; signalId: string; name: string; value: unknown; timestamp: number }
  | { type: "execution-trace"; nodeId: string; edgeId?: string; timestamp: number }
  | { type: "runtime-ready" | "runtime-error"; message?: string }
  | { type: "paused" | "resumed"; signalId?: string; value?: unknown; timestamp: number };

type IncomingCmd =
  | { type: "set-breakpoint"; signalId: string }
  | { type: "remove-breakpoint"; signalId: string }
  | { type: "step" | "resume" | "get-state" };

class DebugBus {
  static breakpoints = new Set<string>();
  static paused = false;
  static flushing = false;
  static pendingQueue: PendingEntry[] = [];

  private static ws: WebSocket | null = null;
  private static reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private static messageQueue: WsMessage[] = [];
  private static wsUrl: string = "";

  /** Initialize WebSocket connection — called once at startup. */
  static init(wsUrl?: string): void {
    DebugBus.wsUrl = wsUrl || "ws://localhost:8942/api/debug/ws";
    DebugBus.connect();
  }

  private static connect(): void {
    if (DebugBus.ws?.readyState === WebSocket.OPEN) return;
    try {
      DebugBus.ws = new WebSocket(DebugBus.wsUrl);
      DebugBus.ws.onopen = () => {
        DebugBus.send({ type: "runtime-ready" });
        // Flush queued messages
        for (const msg of DebugBus.messageQueue) {
          DebugBus.sendRaw(msg);
        }
        DebugBus.messageQueue = [];
      };
      DebugBus.ws.onmessage = (event) => {
        try {
          const msg: IncomingCmd = JSON.parse(event.data as string);
          switch (msg.type) {
            case "set-breakpoint":
              if (msg.signalId) DebugBus.breakpoints.add(msg.signalId);
              break;
            case "remove-breakpoint":
              if (msg.signalId) DebugBus.breakpoints.delete(msg.signalId);
              break;
            case "step":
              DebugBus.step();
              break;
            case "resume":
              DebugBus.resume();
              break;
          }
        } catch { /* ignore malformed */ }
      };
      DebugBus.ws.onclose = () => {
        DebugBus.scheduleReconnect();
      };
    } catch {
      DebugBus.scheduleReconnect();
    }
  }

  private static scheduleReconnect(): void {
    if (DebugBus.reconnectTimer) return;
    DebugBus.reconnectTimer = setTimeout(() => {
      DebugBus.reconnectTimer = null;
      DebugBus.connect();
    }, 2000);
  }

  static send(msg: WsMessage): void {
    if (DebugBus.ws?.readyState === WebSocket.OPEN) {
      DebugBus.sendRaw(msg);
    } else if (DebugBus.messageQueue.length < 100) {
      DebugBus.messageQueue.push(msg);
    }
  }

  private static sendRaw(msg: WsMessage): void {
    try { DebugBus.ws?.send(JSON.stringify(msg)); } catch { /* best-effort */ }
  }

  /** Check breakpoint BEFORE applying value. Returns true if deferred (caller should NOT apply). */
  static check(signalId: string, name: string, value: unknown, apply: () => void): boolean {
    // Always report state changes
    DebugBus.send({ type: "state-change", signalId, name, value, timestamp: Date.now() });

    // Check breakpoint
    if (!DebugBus.flushing && DebugBus.breakpoints.has(signalId)) {
      DebugBus.paused = true;

      // Replace existing entry for same signal (StrictMode double-invoke guard)
      const existing = DebugBus.pendingQueue.find(p => p.signalId === signalId);
      if (existing) {
        existing.value = value;
        existing.apply = apply;
      } else {
        DebugBus.pendingQueue.push({ signalId, value, apply });
      }

      DebugBus.send({ type: "paused", signalId, value, timestamp: Date.now() });
      return true; // deferred
    }
    return false; // proceed
  }

  static hasBreakpoint(signalId: string): boolean {
    return DebugBus.breakpoints.has(signalId);
  }

  static step(): void {
    if (DebugBus.pendingQueue.length === 0) return;
    DebugBus.flushing = true;
    const item = DebugBus.pendingQueue.shift()!;
    item.apply();
    DebugBus.flushing = false;

    if (DebugBus.pendingQueue.length === 0) {
      DebugBus.paused = false;
      DebugBus.send({ type: "resumed", timestamp: Date.now() });
    } else {
      // Still paused — notify of next pending
      const next = DebugBus.pendingQueue[0];
      DebugBus.send({ type: "paused", signalId: next.signalId, value: next.value, timestamp: Date.now() });
    }
  }

  static resume(): void {
    DebugBus.flushing = true;
    while (DebugBus.pendingQueue.length > 0) {
      const item = DebugBus.pendingQueue.shift()!;
      item.apply();
    }
    DebugBus.paused = false;
    DebugBus.flushing = false;
    DebugBus.send({ type: "resumed", timestamp: Date.now() });
  }

  /** Send execution trace for FlowCanvas node highlighting. */
  static trace(nodeId: string, edgeId?: string): void {
    DebugBus.send({ type: "execution-trace", nodeId, edgeId, timestamp: Date.now() });
  }
}

// Initialize DebugBus only when debug mode is explicitly enabled
if (typeof window !== "undefined") {
  const wsUrl = (window as unknown as Record<string, unknown>).__OMNI_DEBUG_WS_URL__ as string | undefined;
  if (wsUrl) {
    DebugBus.init(wsUrl);
  }
}

// ── Signal Implementation ─────────────────────────────────

class SignalImpl<T> implements Signal<T> {
  private _value: T;
  private _subscribers = new Set<(v: T) => void>();
  public readonly id: string;
  private readonly _name: string;

  constructor(initialValue: T, meta?: SignalMeta) {
    this._value = initialValue;
    this.id = meta?.id ?? "";
    this._name = meta?.name ?? meta?.id ?? "";
  }

  get(): T {
    trackDep(this);
    return this._value;
  }

  peek(): T {
    return this._value;
  }

  set(v: T | ((prev: T) => T)): void {
    const resolved = typeof v === "function" ? (v as (prev: T) => T)(this._value) : v;

    // DebugBus interception — ONE place for ALL state types
    const deferred = DebugBus.check(
      this.id,
      this._name,
      resolved,
      () => this._apply(resolved)
    );

    if (!deferred) {
      this._apply(resolved);
    }
  }

  private _apply(value: T): void {
    if (this._value === value) return;
    this._value = value;
    scheduleNotify(this as unknown as SignalImpl<unknown>);
  }

  /** @internal — called by scheduler */
  _notify(): void {
    const v = this._value;
    for (const fn of this._subscribers) {
      try { fn(v); } catch { /* best-effort: don't let one subscriber break others */ }
    }
  }

  subscribe(fn: (v: T) => void): CleanupFn {
    this._subscribers.add(fn);
    return () => { this._subscribers.delete(fn); };
  }
}

// ── Computed Implementation ───────────────────────────────

class ComputedImpl<T> implements Computed<T> {
  private _value: T | undefined;
  private _dirty = true;
  private _factory: () => T;
  private _subscribers = new Set<(v: T) => void>();
  public readonly id: string;

  constructor(factory: () => T, meta?: SignalMeta) {
    this._factory = factory;
    this.id = meta?.id ?? "";
  }

  get(): T {
    trackDep(this);
    if (this._dirty) {
      this._recalc();
    }
    return this._value!;
  }

  peek(): T {
    if (this._dirty) {
      this._recalc();
    }
    return this._value!;
  }

  private _recalc(): void {
    // Clean up old subscriptions
    if (this._oldUnsubs) {
      for (const u of this._oldUnsubs) u();
    }

    const { result, deps } = runWithTracker(() => this._factory());
    this._value = result;
    this._dirty = false;

    // Subscribe to all dependencies — when any changes, mark dirty
    const unsubs: CleanupFn[] = [];
    for (const dep of deps) {
      unsubs.push(dep.subscribe(() => {
        if (!this._dirty) {
          this._dirty = true;
          scheduleNotify(this as unknown as SignalImpl<unknown>);
        }
      }));
    }
    this._oldUnsubs = unsubs;
  }

  private _oldUnsubs: CleanupFn[] = [];

  /** @internal */
  _notify(): void {
    const v = this.peek();
    for (const fn of this._subscribers) {
      try { fn(v); } catch { /* best-effort */ }
    }
  }

  subscribe(fn: (v: T) => void): CleanupFn {
    this._subscribers.add(fn);
    return () => { this._subscribers.delete(fn); };
  }
}

// ── Public API: signal, computed, effect, store ────────────

/**
 * Create a reactive signal. Drop-in replacement for useState.
 *
 * Example:
 *   const count = signal(0, { id: "Counter.count", name: "count", type: "number" });
 *   count.get();           // 0
 *   count.set(count.get() + 1);
 *   count.subscribe(v => console.log("count:", v));
 */
export function signal<T>(initialValue: T, meta?: SignalMeta): Signal<T> {
  return new SignalImpl(initialValue, meta);
}

/**
 * Create a derived/computed signal. Replaces useMemo.
 * Dependencies are auto-tracked — no manual dep array needed.
 *
 * Example:
 *   const doubled = computed(() => count.get() * 2, { id: "Counter.doubled", name: "doubled" });
 */
export function computed<T>(factory: () => T, meta?: SignalMeta): Computed<T> {
  return new ComputedImpl(factory, meta);
}

/**
 * Create a reactive effect. Replaces useEffect.
 * Auto-tracks signal reads — re-runs when any dependency changes.
 *
 * Example:
 *   effect(() => {
 *     console.log("Count is now:", count.get());
 *     return () => console.log("cleanup");
 *   });
 */
export function effect(fn: () => CleanupFn | void): CleanupFn {
  let cleanup: CleanupFn | void;
  let unsubs: CleanupFn[] = [];

  const run = () => {
    // Run cleanup from previous invocation
    if (cleanup) {
      try { cleanup(); } catch { /* best-effort */ }
    }

    // Unsubscribe old deps
    for (const u of unsubs) u();
    unsubs = [];

    // Track dependencies
    const { deps } = runWithTracker(() => {
      cleanup = fn();
    });

    // Subscribe to new deps
    for (const dep of deps) {
      unsubs.push(dep.subscribe(() => run()));
    }
  };

  run();

  // Return dispose function
  return () => {
    if (cleanup) {
      try { cleanup(); } catch { /* best-effort */ }
    }
    for (const u of unsubs) u();
    unsubs = [];
  };
}

/**
 * Create a namespaced reactive store. Replaces Zustand create().
 * Properties that are signals are auto-unwrapped via Proxy.
 * Functions remain as-is (actions).
 * Getter properties become computed values.
 *
 * Example:
 *   const TodoStore = store("TodoStore", () => {
 *     const items = signal<string[]>([], { id: "TodoStore.items", name: "items" });
 *     return {
 *       items,
 *       add(text: string) { items.set([...items.get(), text]); },
 *       get count() { return items.get().length; },
 *     };
 *   });
 */
export function store<T extends Record<string, unknown>>(
  name: string,
  factory: () => T
): T & { $id: string } {
  const raw = factory();
  const signalKeys = new Set<string>();

  // Identify which keys are signals/computeds
  for (const key of Object.keys(raw)) {
    if (isSignalLike(raw[key])) {
      signalKeys.add(key);
    }
  }

  return new Proxy(raw as T & { $id: string }, {
    get(target, prop, receiver) {
      if (prop === "$id") return name;
      const val = Reflect.get(target, prop, receiver);
      if (signalKeys.has(prop as string) && isSignalLike(val)) {
        return val.get();
      }
      if (typeof val === "function") return val.bind(target);
      return val;
    },
    set(target, prop, value, receiver) {
      const existing = Reflect.get(target, prop, receiver);
      if (signalKeys.has(prop as string) && isSignalLike(existing)) {
        (existing as Signal<unknown>).set(value);
        return true;
      }
      return Reflect.set(target, prop, value, receiver);
    },
    ownKeys(target) {
      return Reflect.ownKeys(target);
    },
    getOwnPropertyDescriptor(target, prop) {
      return Reflect.getOwnPropertyDescriptor(target, prop);
    },
  }) as T & { $id: string };
}

// ── DOM: h() — Virtual Node Builder ───────────────────────

/**
 * Create a virtual DOM node. Replaces JSX.
 *
 * Example:
 *   h("div", { class: "container" },
 *     h("button", { onClick: increment }, "+1"),
 *     h("span", {}, count)  // Signal child auto-bound
 *   )
 */
export function h(
  tag: string,
  attrs: Record<string, unknown> = {},
  ...children: (VNode | string | number | null | undefined | Signal<unknown> | Computed<unknown>)[]
): VNode {
  return { tag, attrs, children: children.flat() };
}

// ── DOM: render() — Mount to Real DOM ─────────────────────

/** Mount a VNode tree into a container. Returns cleanup function. */
export function render(
  vnode: VNode | VNode[] | null | undefined,
  container: HTMLElement
): CleanupFn {
  const cleanups: CleanupFn[] = [];

  const nodes = Array.isArray(vnode) ? vnode : [vnode];
  for (const n of nodes) {
    if (!n) continue;
    const { el, cleanup } = mountVNode(n, cleanups);
    container.appendChild(el);
    if (cleanup) cleanups.push(cleanup);
  }

  return () => {
    container.innerHTML = "";
    for (const c of cleanups) {
      try { c(); } catch { /* best-effort */ }
    }
    cleanups.length = 0;
  };
}

/** Create a real DOM element from a VNode. Returns element + optional cleanup. */
function mountVNode(
  vnode: VNode,
  cleanups: CleanupFn[]
): { el: HTMLElement; cleanup?: CleanupFn } {
  // Handle HTML elements directly
  const el = document.createElement(vnode.tag);

  // Apply attributes and events
  for (const [key, value] of Object.entries(vnode.attrs)) {
    if (key.startsWith("on") && typeof value === "function") {
      // Event handler: onClick → click
      const eventName = key.slice(2).toLowerCase();
      el.addEventListener(eventName, value as EventListener);
      cleanups.push(() => el.removeEventListener(eventName, value as EventListener));
    } else if (key === "class" || key === "className") {
      el.className = String(value);
    } else if (key === "style" && typeof value === "object" && value !== null) {
      Object.assign(el.style, value as Record<string, string>);
    } else if (key in el) {
      (el as unknown as Record<string, unknown>)[key] = value;
    } else {
      el.setAttribute(key, String(value));
    }
  }

  // Apply children
  for (const child of vnode.children) {
    if (child === null || child === undefined) continue;

    if (isSignalLike(child)) {
      // Reactive binding: create a text node and bind to signal
      const textNode = document.createTextNode(String(child.get()));
      el.appendChild(textNode);
      cleanups.push(child.subscribe((v: unknown) => {
        textNode.textContent = v === null || v === undefined ? "" : String(v);
      }));
    } else if (child instanceof HTMLElement) {
      // Native DOM element (from renderConditional, renderList, etc.)
      el.appendChild(child);
    } else if (typeof child === "string" || typeof child === "number") {
      el.appendChild(document.createTextNode(String(child)));
    } else if (typeof child === "object" && "tag" in child) {
      const sub = mountVNode(child as VNode, cleanups);
      el.appendChild(sub.el);
      if (sub.cleanup) cleanups.push(sub.cleanup);
    }
  }

  return { el };
}

// ── DOM: Reactive Bindings ────────────────────────────────

/**
 * Bind a signal to an element's textContent. Returns cleanup.
 * Use for text nodes that should update when the signal changes.
 */
export function bindTextContent(el: HTMLElement, s: Signal<unknown> | Computed<unknown>): CleanupFn {
  el.textContent = String(s.get());
  return s.subscribe((v: unknown) => {
    el.textContent = v === null || v === undefined ? "" : String(v);
  });
}

/**
 * Bind a signal to a DOM property (e.g., value, checked, disabled).
 */
export function bindProperty(
  el: HTMLElement,
  prop: string,
  s: Signal<unknown> | Computed<unknown>
): CleanupFn {
  (el as unknown as Record<string, unknown>)[prop] = s.get();
  return s.subscribe((v: unknown) => {
    (el as unknown as Record<string, unknown>)[prop] = v;
  });
}

/**
 * Bind a signal to a DOM attribute.
 */
export function bindAttribute(
  el: HTMLElement,
  attr: string,
  s: Signal<unknown> | Computed<unknown>
): CleanupFn {
  el.setAttribute(attr, String(s.get()));
  return s.subscribe((v: unknown) => {
    el.setAttribute(attr, String(v));
  });
}

// ── DOM: Control Flow ─────────────────────────────────────

/**
 * Conditionally render content based on a boolean signal.
 * Returns a transparent wrapper element that can be embedded in h() trees.
 */
export function renderConditional(
  condition: Signal<boolean> | Computed<boolean>,
  createTrue: () => VNode | VNode[] | null | undefined,
  createFalse?: () => VNode | VNode[] | null | undefined
): HTMLElement {
  const wrapper = document.createElement("span");
  wrapper.style.display = "contents";
  const mountPoint = document.createComment("if");
  wrapper.appendChild(mountPoint);
  let currentCleanup: CleanupFn | null = null;

  const update = (val: boolean) => {
    // Remove current content
    if (currentCleanup) { currentCleanup(); currentCleanup = null; }

    // Remove DOM after mountPoint
    let next = mountPoint.nextSibling;
    while (next) {
      const toRemove = next;
      next = next.nextSibling;
      toRemove.remove();
    }

    // Create new content
    const vnode = val ? createTrue() : (createFalse ? createFalse() : null);
    if (vnode) {
      const nodes = Array.isArray(vnode) ? vnode : [vnode];
      const fragment = document.createDocumentFragment();
      const subCleanups: CleanupFn[] = [];
      for (const n of nodes) {
        if (!n) continue;
        const { el, cleanup } = mountVNode(n, subCleanups);
        fragment.appendChild(el);
        if (cleanup) subCleanups.push(cleanup);
      }
      mountPoint.after(fragment);
      currentCleanup = () => {
        for (const c of subCleanups) c();
        subCleanups.length = 0;
      };
    }
  };

  update(condition.get());
  condition.subscribe(update);
  // Register cleanup through the wrapper's removal detection
  return wrapper;
}

/**
 * Render a list from a signal array. Returns a wrapper element for embedding in h() trees.
 * Key-based DOM recycling for efficient updates.
 */
export function renderList<T>(
  items: Signal<T[]> | Computed<T[]>,
  createItem: (item: T, index: number) => VNode,
  keyFn?: (item: T) => string
): HTMLElement {
  const wrapper = document.createElement("span");
  wrapper.style.display = "contents";
  let keyedElements = new Map<string, HTMLElement>();
  let cleanupMap = new Map<string, CleanupFn>();

  const getKey = keyFn || ((_: T, i: number) => String(i));

  const update = (newItems: T[]) => {
    const newKeyed = new Map<string, HTMLElement>();
    const newCleanups = new Map<string, CleanupFn>();
    const fragment = document.createDocumentFragment();

    for (let i = 0; i < newItems.length; i++) {
      const item = newItems[i];
      const key = getKey(item, i);

      let el = keyedElements.get(key);
      if (el) {
        // Reuse existing element
        keyedElements.delete(key);
        const c = cleanupMap.get(key);
        if (c) cleanupMap.delete(key);
      } else {
        // Create new element
        const subCleanups: CleanupFn[] = [];
        const vnode = createItem(item, i);
        const { el: newEl, cleanup } = mountVNode(vnode, subCleanups);
        el = newEl;
        if (cleanup) subCleanups.push(cleanup);
        newCleanups.set(key, () => {
          for (const c of subCleanups) c();
          subCleanups.length = 0;
        });
      }
      if (el) {
        newKeyed.set(key, el);
        if (!newCleanups.has(key) && cleanupMap.has(key)) {
          newCleanups.set(key, cleanupMap.get(key)!);
        }
        fragment.appendChild(el);
      }
    }

    // Clean up removed elements
    for (const [, el] of keyedElements) {
      el.remove();
    }
    for (const [, c] of cleanupMap) {
      try { c(); } catch { /* best-effort */ }
    }

    // Replace content
    wrapper.innerHTML = "";
    wrapper.appendChild(fragment);

    keyedElements = newKeyed;
    cleanupMap = newCleanups;
  };

  update(items.get());
  items.subscribe(update);
  return wrapper;
}

// ── Component Lifecycle ───────────────────────────────────

const lifecycleCleanups: CleanupFn[] = [];
let disposed = false;

/**
 * Register a cleanup function to run on component unmount.
 * Use for effect disposals, event listener removals, etc.
 */
export function onCleanup(fn: CleanupFn): void {
  if (disposed) return;
  lifecycleCleanups.push(fn);
}

/**
 * Dispose all registered cleanups. Call on component unmount.
 */
export function dispose(): void {
  if (disposed) return;
  disposed = true;
  for (const c of lifecycleCleanups) {
    try { c(); } catch { /* best-effort */ }
  }
  lifecycleCleanups.length = 0;
}

// ── Execution Tracing ─────────────────────────────────────

/**
 * Mark a function/statement as executed for FlowCanvas highlighting.
 * Wraps a function to send execution-trace before invocation.
 */
export function traceExecution(nodeId: string, edgeId?: string): void {
  DebugBus.trace(nodeId, edgeId);
}

/** Wrap a function with automatic trace execution. */
export function withTrace<T extends (...args: unknown[]) => unknown>(
  nodeId: string,
  fn: T
): T {
  return ((...args: unknown[]) => {
    DebugBus.trace(nodeId);
    return fn(...args);
  }) as T;
}

// ── Browser Console Access ────────────────────────────────

if (typeof window !== "undefined") {
  (window as unknown as Record<string, unknown>).__OMNI_RUNTIME__ = {
    DebugBus,
    signal,
    computed,
    effect,
    store,
    h,
    render,
    traceExecution,
  };
}
