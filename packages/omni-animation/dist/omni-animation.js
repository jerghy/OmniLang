"use strict";
var Omni = (() => {
  var __defProp = Object.defineProperty;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };
  var __copyProps = (to, from, except, desc) => {
    if (from && typeof from === "object" || typeof from === "function") {
      for (let key of __getOwnPropNames(from))
        if (!__hasOwnProp.call(to, key) && key !== except)
          __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
    }
    return to;
  };
  var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);

  // src/index.ts
  var index_exports = {};
  __export(index_exports, {
    animate: () => animate,
    buildKeyframes: () => buildKeyframes,
    buildProperties: () => buildProperties,
    flipAfter: () => flipAfter,
    flipBefore: () => flipBefore,
    resolveEasing: () => resolveEasing,
    scrollProgress: () => scrollProgress,
    staggerDelays: () => staggerDelays,
    timeline: () => timeline
  });

  // src/keyframes.ts
  var TRANSFORM_MAP = {
    x: "translateX",
    y: "translateY",
    translateX: "translateX",
    translateY: "translateY",
    translateZ: "translateZ",
    scale: "scale",
    scaleX: "scaleX",
    scaleY: "scaleY",
    rotate: "rotate",
    rotateX: "rotateX",
    rotateY: "rotateY",
    rotateZ: "rotateZ",
    skewX: "skewX",
    skewY: "skewY"
  };
  var TRANSFORM_PROPS = new Set(Object.keys(TRANSFORM_MAP));
  var PX_PROPS = /* @__PURE__ */ new Set([
    "width",
    "height",
    "margin",
    "padding",
    "borderRadius",
    "borderWidth",
    "marginTop",
    "marginRight",
    "marginBottom",
    "marginLeft",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
    "borderTopLeftRadius",
    "borderTopRightRadius",
    "borderBottomLeftRadius",
    "borderBottomRightRadius",
    "gap",
    "rowGap",
    "columnGap",
    "maxWidth",
    "minWidth",
    "maxHeight",
    "minHeight",
    "top",
    "left",
    "right",
    "bottom",
    "fontSize",
    "letterSpacing"
  ]);
  var PCT_PROPS = /* @__PURE__ */ new Set([
    "offsetDistance"
  ]);
  var FILTER_MAP = {
    blur: "blur",
    brightness: "brightness",
    contrast: "contrast",
    saturate: "saturate",
    hueRotate: "hue-rotate"
  };
  function buildProperties(props) {
    const cssProps = {};
    const transforms = [];
    const filters = [];
    if (!props) return cssProps;
    for (const [key, value] of Object.entries(props)) {
      if (TRANSFORM_PROPS.has(key)) {
        const cssName = TRANSFORM_MAP[key];
        const num = typeof value === "number" ? value : parseFloat(value);
        if (key === "x" || key === "y" || key === "translateX" || key === "translateY") {
          if (typeof value === "string" && value.endsWith("%")) {
            transforms.push(`${cssName}(${value})`);
          } else {
            transforms.push(`${cssName}(${num}px)`);
          }
        } else if (key === "translateZ") {
          transforms.push(`${cssName}(${num}px)`);
        } else if (key.startsWith("rotate") || key.startsWith("skew")) {
          transforms.push(`${cssName}(${num}deg)`);
        } else {
          transforms.push(`${cssName}(${num})`);
        }
      } else if (key in FILTER_MAP) {
        const cssName = FILTER_MAP[key];
        const num = typeof value === "number" ? value : parseFloat(value);
        if (key === "blur") filters.push(`${cssName}(${num}px)`);
        else if (key === "hueRotate") filters.push(`${cssName}(${num}deg)`);
        else filters.push(`${cssName}(${num})`);
      } else {
        let formatted = value;
        if (typeof value === "number" && PX_PROPS.has(key)) {
          formatted = `${value}px`;
        } else if (typeof value === "number" && PCT_PROPS.has(key)) {
          formatted = `${value * 100}%`;
        }
        cssProps[key] = String(formatted);
      }
    }
    if (transforms.length) cssProps.transform = transforms.join(" ");
    if (filters.length) cssProps.filter = filters.join(" ");
    return cssProps;
  }
  function buildKeyframes(stops) {
    const sorted = [...stops].sort((a, b) => a.offset - b.offset);
    return sorted.map((stop) => {
      const kf = { offset: stop.offset };
      const cssProps = buildProperties(stop.properties);
      Object.assign(kf, cssProps);
      return kf;
    });
  }

  // src/easing.ts
  var NAMED_EASINGS = {
    linear: "linear",
    ease: "ease",
    easeIn: "ease-in",
    easeOut: "ease-out",
    easeInOut: "ease-in-out",
    "ease-in": "ease-in",
    "ease-out": "ease-out",
    "ease-in-out": "ease-in-out",
    // power1 (quadratic)
    power1In: "cubic-bezier(0.550,0.085,0.680,0.530)",
    power1Out: "cubic-bezier(0.250,0.460,0.450,0.940)",
    power1InOut: "cubic-bezier(0.455,0.030,0.515,0.955)",
    "power1-in": "cubic-bezier(0.550,0.085,0.680,0.530)",
    "power1-out": "cubic-bezier(0.250,0.460,0.450,0.940)",
    "power1-in-out": "cubic-bezier(0.455,0.030,0.515,0.955)",
    // power2 (cubic)
    power2In: "cubic-bezier(0.550,0.055,0.675,0.190)",
    power2Out: "cubic-bezier(0.215,0.610,0.355,1.000)",
    power2InOut: "cubic-bezier(0.645,0.045,0.355,1.000)",
    "power2-in": "cubic-bezier(0.550,0.055,0.675,0.190)",
    "power2-out": "cubic-bezier(0.215,0.610,0.355,1.000)",
    "power2-in-out": "cubic-bezier(0.645,0.045,0.355,1.000)",
    // power3 (quartic)
    power3In: "cubic-bezier(0.895,0.030,0.685,0.220)",
    power3Out: "cubic-bezier(0.165,0.840,0.440,1.000)",
    power3InOut: "cubic-bezier(0.770,0.000,0.175,1.000)",
    "power3-in": "cubic-bezier(0.895,0.030,0.685,0.220)",
    "power3-out": "cubic-bezier(0.165,0.840,0.440,1.000)",
    "power3-in-out": "cubic-bezier(0.770,0.000,0.175,1.000)",
    // power4 (quintic)
    power4In: "cubic-bezier(0.755,0.050,0.855,0.060)",
    power4Out: "cubic-bezier(0.230,1.000,0.320,1.000)",
    power4InOut: "cubic-bezier(0.860,0.000,0.070,1.000)",
    "power4-in": "cubic-bezier(0.755,0.050,0.855,0.060)",
    "power4-out": "cubic-bezier(0.230,1.000,0.320,1.000)",
    "power4-in-out": "cubic-bezier(0.860,0.000,0.070,1.000)",
    // back
    backIn: "cubic-bezier(0.600,-0.280,0.735,0.045)",
    backOut: "cubic-bezier(0.175,0.885,0.320,1.275)",
    backInOut: "cubic-bezier(0.680,-0.550,0.265,1.550)",
    "back-in": "cubic-bezier(0.600,-0.280,0.735,0.045)",
    "back-out": "cubic-bezier(0.175,0.885,0.320,1.275)",
    "back-in-out": "cubic-bezier(0.680,-0.550,0.265,1.550)",
    // Bounce approximations (WAAPI cannot natively represent bounce curves.
    // These cubic-bezier values approximate the directional asymmetry: bounceIn
    // has overshoot at the start, bounceOut has overshoot at the end.)
    bounceIn: "cubic-bezier(0.215,0.610,0.355,1.000)",
    bounceOut: "cubic-bezier(0.175,0.885,0.320,1.275)",
    bounceInOut: "cubic-bezier(0.645,0.045,0.355,1.000)",
    "bounce-in": "cubic-bezier(0.215,0.610,0.355,1.000)",
    "bounce-out": "cubic-bezier(0.175,0.885,0.320,1.275)",
    "bounce-in-out": "cubic-bezier(0.645,0.045,0.355,1.000)",
    // elastic (approximations)
    elasticIn: "cubic-bezier(0.600,0.040,0.980,0.335)",
    elasticOut: "cubic-bezier(0.250,0.460,0.450,0.940)",
    elasticInOut: "cubic-bezier(0.680,-0.550,0.265,1.550)",
    "elastic-in": "cubic-bezier(0.600,0.040,0.980,0.335)",
    "elastic-out": "cubic-bezier(0.250,0.460,0.450,0.940)",
    "elastic-in-out": "cubic-bezier(0.680,-0.550,0.265,1.550)",
    // spring → generic bouncy
    spring: "cubic-bezier(0.250,0.460,0.450,0.940)",
    stepStart: "steps(1,start)",
    stepEnd: "steps(1,end)",
    "step-start": "steps(1,start)",
    "step-end": "steps(1,end)"
  };
  function resolveEasing(easing) {
    if (typeof easing === "string") {
      return NAMED_EASINGS[easing] ?? easing;
    }
    switch (easing.type) {
      case "cubicBezier":
        return `cubic-bezier(${easing.x1},${easing.y1},${easing.x2},${easing.y2})`;
      case "back": {
        const d = easing.direction ?? "out";
        if (d === "in") return "cubic-bezier(0.600,-0.280,0.735,0.045)";
        if (d === "inOut") return "cubic-bezier(0.680,-0.550,0.265,1.550)";
        return "cubic-bezier(0.175,0.885,0.320,1.275)";
      }
      case "elastic": {
        const d = easing.direction ?? "out";
        if (d === "in") return "cubic-bezier(0.600,0.040,0.980,0.335)";
        if (d === "inOut") return "cubic-bezier(0.680,-0.550,0.265,1.550)";
        return "cubic-bezier(0.250,0.460,0.450,0.940)";
      }
      default:
        return "ease-out";
    }
  }

  // src/animate.ts
  function animate(el, stops, options) {
    if (!el) return null;
    const keyframes = buildKeyframes(stops);
    const timing = {
      duration: options.duration,
      delay: options.delay ?? 0,
      easing: resolveEasing(options.easing ?? "ease-out"),
      fill: options.fill ?? "both",
      direction: options.direction ?? "normal",
      iterations: options.iterations ?? 1
    };
    return el.animate(keyframes, timing);
  }

  // src/timeline.ts
  function timeline(steps) {
    const animations = [];
    for (const step of steps) {
      if (!step.keyframes || step.keyframes.length === 0) {
        animations.push(null);
        continue;
      }
      const stepOptions = step.options ?? {};
      const opts = { ...stepOptions, delay: (stepOptions.delay ?? 0) + step.at };
      const anim = animate(step.el, step.keyframes, opts);
      animations.push(anim);
    }
    return animations;
  }

  // src/stagger.ts
  function staggerDelays(count, each, from = "start") {
    const indices = [];
    if (from === "start") {
      for (let i = 0; i < count; i++) indices.push(i);
    } else if (from === "end") {
      for (let i = count - 1; i >= 0; i--) indices.push(i);
    } else if (from === "edges") {
      let l = 0, r = count - 1;
      while (l <= r) {
        indices.push(l++);
        if (l <= r) indices.push(r--);
      }
    } else {
      const mid = Math.floor((count - 1) / 2);
      indices.push(mid);
      let l = mid - 1, r = mid + 1;
      while (l >= 0 || r < count) {
        if (l >= 0) indices.push(l--);
        if (r < count) indices.push(r++);
      }
    }
    const delays = new Array(count).fill(0);
    indices.forEach((origIdx, pos) => {
      delays[origIdx] = pos * each;
    });
    return delays;
  }

  // src/scroll.ts
  function scrollProgress(el, startCfg, endCfg) {
    const vh = window.innerHeight;
    const startPos = parseScrollTrigger(startCfg, el, vh);
    const endPos = parseScrollTrigger(endCfg, el, vh);
    const raw = (window.scrollY - startPos) / (endPos - startPos);
    return Math.max(0, Math.min(1, raw));
  }
  function parseScrollTrigger(cfg, el, vh) {
    const parts = cfg.trim().split(/\s+/);
    if (parts.length < 2) return 0;
    const edge = parts[0];
    const offset = parts[1];
    const rect = el.getBoundingClientRect();
    let elPos;
    if (edge === "top") elPos = rect.top;
    else if (edge === "center") elPos = rect.top + rect.height / 2;
    else if (edge === "bottom") elPos = rect.bottom;
    else elPos = rect.top;
    const absEdge = window.scrollY + elPos;
    if (offset.endsWith("%")) return absEdge - vh * (parseFloat(offset) / 100);
    if (offset === "bottom") return absEdge - vh;
    if (offset === "top") return absEdge;
    return absEdge - vh * 0.5;
  }

  // src/flip.ts
  var flipState = /* @__PURE__ */ new Map();
  function flipBefore(container) {
    const containerEl = typeof container === "string" ? document.querySelector(container) : container;
    if (!containerEl) return;
    containerEl.querySelectorAll("[data-for-key]").forEach((el) => {
      const key = el.getAttribute("data-for-key");
      if (key) flipState.set(key, { rect: el.getBoundingClientRect(), element: el });
    });
  }
  function flipAfter(container, duration = 400, easing = "ease-out") {
    const containerEl = typeof container === "string" ? document.querySelector(container) : container;
    if (!containerEl) return;
    containerEl.querySelectorAll("[data-for-key]").forEach((el) => {
      const key = el.getAttribute("data-for-key");
      if (!key) return;
      const prev = flipState.get(key);
      if (!prev) return;
      const newRect = el.getBoundingClientRect();
      const dx = prev.rect.left - newRect.left;
      const dy = prev.rect.top - newRect.top;
      if (dx === 0 && dy === 0) return;
      el.style.transform = `translate(${dx}px,${dy}px)`;
      el.style.transition = "none";
      void el.offsetHeight;
      el.style.transition = `transform ${duration}ms ${easing}`;
      el.style.transform = "translate(0,0)";
      const cleanupStyles = () => {
        ;
        el.style.transition = "";
        el.style.transform = "";
      };
      const fallbackTimer = setTimeout(cleanupStyles, duration + 50);
      el.addEventListener("transitionend", function handler() {
        clearTimeout(fallbackTimer);
        cleanupStyles();
        el.removeEventListener("transitionend", handler);
      });
    });
    flipState.clear();
  }
  return __toCommonJS(index_exports);
})();
