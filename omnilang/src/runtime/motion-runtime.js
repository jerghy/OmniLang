// OmniView Motion Runtime v0.1.0
// Web Animation Engine -- WAAPI integration, spring physics, easing functions
// Embedded into compiled output by the omni-compiler codegen.
(function() {
"use strict";

/* --- 1. Spring Physics Solver --- */

class SpringSolver {
  constructor(tension, friction, mass, velocity, precision) {
    this.tension = tension != null ? tension : 180;
    this.friction = friction != null ? friction : 12;
    this.mass = mass != null ? mass : 1;
    this.velocity = velocity != null ? velocity : 0;
    this.precision = precision != null ? precision : 0.01;
  }

  get omega0() {
    return Math.sqrt(this.tension / this.mass);
  }

  get zeta() {
    return this.friction / (2 * Math.sqrt(this.tension * this.mass));
  }

  solve(from, to, t) {
    const delta = to - from;
    const v0 = this.velocity;
    const zeta = this.zeta;
    const omega0 = this.omega0;

    if (zeta < 1) {
      // Underdamped -- oscillatory approach to equilibrium
      const omegad = omega0 * Math.sqrt(1 - zeta * zeta);
      const envelope = Math.exp(-zeta * omega0 * t);
      const A = delta;
      const B = (v0 + zeta * omega0 * delta) / omegad;
      return to - envelope * (A * Math.cos(omegad * t) + B * Math.sin(omegad * t));
    } else if (Math.abs(zeta - 1) < 1e-6) {
      // Critically damped -- fastest non-oscillatory return
      const envelope = Math.exp(-omega0 * t);
      return to - envelope * (delta + (v0 + omega0 * delta) * t);
    } else {
      // Overdamped -- slow, non-oscillatory decay
      const sqrtTerm = Math.sqrt(zeta * zeta - 1);
      const r1 = -zeta * omega0 + omega0 * sqrtTerm;
      const r2 = -zeta * omega0 - omega0 * sqrtTerm;
      const c1 = (v0 - r2 * delta) / (r1 - r2);
      const c2 = delta - c1;
      return to - (c1 * Math.exp(r1 * t) + c2 * Math.exp(r2 * t));
    }
  }

  getDuration(from, to) {
    const total = Math.abs(to - from);
    const prec = total < 5 ? this.precision * 0.1 : this.precision;
    for (let t = 0; t < 10; t += 1 / 60) {
      const pos = this.solve(from, to, t);
      const pos2 = this.solve(from, to, t + 0.001);
      const vel = (pos2 - pos) * 1000;
      if (Math.abs(pos - to) < prec && Math.abs(vel) < prec) {
        return t * 1000;
      }
    }
    return 5000;
  }
}


/* --- 2. Easing Function Registry --- */

const EASINGS = {};

// --- Standard ---

EASINGS.linear = function linear(t) {
  return t;
};

EASINGS.easeIn = function easeIn(t) {
  return t * t * t;
};

EASINGS.easeOut = function easeOut(t) {
  return 1 - Math.pow(1 - t, 3);
};

EASINGS.easeInOut = function easeInOut(t) {
  return t < 0.5
    ? 4 * t * t * t
    : 1 - Math.pow(-2 * t + 2, 3) / 2;
};

// --- Power curves (quadratic through quintic) ---

// Power 1 (quadratic): t^2, 1-(1-t)^2, etc.
EASINGS.power1In = function power1In(t) {
  return t * t;
};
EASINGS.power1Out = function power1Out(t) {
  return 1 - (1 - t) * (1 - t);
};
EASINGS.power1InOut = function power1InOut(t) {
  return t < 0.5
    ? 2 * t * t
    : 1 - Math.pow(-2 * t + 2, 2) / 2;
};

// Power 2 (cubic): t^3, 1-(1-t)^3, etc.
EASINGS.power2In = EASINGS.easeIn;
EASINGS.power2Out = EASINGS.easeOut;
EASINGS.power2InOut = EASINGS.easeInOut;

// Power 3 (quartic)
EASINGS.power3In = function power3In(t) {
  return t * t * t * t;
};
EASINGS.power3Out = function power3Out(t) {
  return 1 - Math.pow(1 - t, 4);
};
EASINGS.power3InOut = function power3InOut(t) {
  return t < 0.5
    ? 8 * t * t * t * t
    : 1 - Math.pow(-2 * t + 2, 4) / 2;
};

// Power 4 (quintic)
EASINGS.power4In = function power4In(t) {
  return t * t * t * t * t;
};
EASINGS.power4Out = function power4Out(t) {
  return 1 - Math.pow(1 - t, 5);
};
EASINGS.power4InOut = function power4InOut(t) {
  return t < 0.5
    ? 16 * t * t * t * t * t
    : 1 - Math.pow(-2 * t + 2, 5) / 2;
};

// --- Back (overshoot) ---

function backInFn(t, intensity) {
  const i = intensity != null ? intensity : 1.70158;
  return (i + 1) * t * t * t - i * t * t;
}

function backOutFn(t, intensity) {
  const i = intensity != null ? intensity : 1.70158;
  return 1 + (i + 1) * Math.pow(t - 1, 3) + i * Math.pow(t - 1, 2);
}

function backInOutFn(t, intensity) {
  const i = intensity != null ? intensity : 1.70158;
  const i2 = i * 1.525;
  return t < 0.5
    ? (Math.pow(2 * t, 2) * ((i2 + 1) * 2 * t - i2)) / 2
    : (Math.pow(2 * t - 2, 2) * ((i2 + 1) * (t * 2 - 2) + i2) + 2) / 2;
}

EASINGS.backIn = function backIn(t) { return backInFn(t); };
EASINGS.backOut = function backOut(t) { return backOutFn(t); };
EASINGS.backInOut = function backInOut(t) { return backInOutFn(t); };

// --- Bounce ---

function bounceOutFn(t) {
  const n1 = 7.5625;
  const d1 = 2.75;
  if (t < 1 / d1) {
    return n1 * t * t;
  } else if (t < 2 / d1) {
    return n1 * (t -= 1.5 / d1) * t + 0.75;
  } else if (t < 2.5 / d1) {
    return n1 * (t -= 2.25 / d1) * t + 0.9375;
  } else {
    return n1 * (t -= 2.625 / d1) * t + 0.984375;
  }
}

EASINGS.bounceOut = bounceOutFn;

EASINGS.bounceIn = function bounceIn(t) {
  return 1 - bounceOutFn(1 - t);
};

EASINGS.bounceInOut = function bounceInOut(t) {
  return t < 0.5
    ? (1 - bounceOutFn(1 - 2 * t)) / 2
    : (1 + bounceOutFn(2 * t - 1)) / 2;
};

// --- Elastic ---

function elasticOutFn(t, amplitude, period) {
  const a = amplitude != null ? amplitude : 1;
  const p = period != null ? period : 0.3;
  if (t === 0 || t === 1) return t;
  const s = p / (2 * Math.PI) * Math.asin(1 / a);
  return a * Math.pow(2, -10 * t) * Math.sin((t - s) * (2 * Math.PI) / p) + 1;
}

function elasticInFn(t, amplitude, period) {
  const a = amplitude != null ? amplitude : 1;
  const p = period != null ? period : 0.3;
  if (t === 0 || t === 1) return t;
  const s = p / (2 * Math.PI) * Math.asin(1 / a);
  return -(a * Math.pow(2, 10 * (t - 1)) * Math.sin(((t - 1) - s) * (2 * Math.PI) / p));
}

EASINGS.elasticOut = function elasticOut(t) { return elasticOutFn(t); };
EASINGS.elasticIn = function elasticIn(t) { return elasticInFn(t); };

EASINGS.elasticInOut = function elasticInOut(t) {
  if (t === 0 || t === 1) return t;
  const a = 1;
  const p = 0.3;
  const s = p / (2 * Math.PI) * Math.asin(1 / a);
  if ((t *= 2) < 1) {
    return -0.5 * (a * Math.pow(2, 10 * (t - 1)) * Math.sin(((t - 1) - s) * (2 * Math.PI) / p));
  }
  return a * Math.pow(2, -10 * (t - 1)) * Math.sin(((t - 1) - s) * (2 * Math.PI) / p) * 0.5 + 1;
};

// --- Cubic Bezier Solver (Newton's method) ---

function cubicBezier(p1x, p1y, p2x, p2y) {
  function sampleCurveX(t) {
    return ((1 - t) * 3 * (1 - t) * t * p1x + 3 * (1 - t) * t * t * p2x + t * t * t);
  }
  function sampleCurveY(t) {
    return ((1 - t) * 3 * (1 - t) * t * p1y + 3 * (1 - t) * t * t * p2y + t * t * t);
  }
  function sampleCurveDerivativeX(t) {
    return (3 * (1 - t) * (1 - t) * p1x + 6 * (1 - t) * t * (p2x - p1x) + 3 * t * t * (1 - p2x));
  }

  function solve(x, epsilon) {
    const e = epsilon != null ? epsilon : 1e-6;
    let t0, t1, t2, x2, d2;
    // First try a few rounds of Newton
    t2 = x;
    for (let i = 0; i < 8; i++) {
      x2 = sampleCurveX(t2) - x;
      if (Math.abs(x2) < e) return sampleCurveY(t2);
      d2 = sampleCurveDerivativeX(t2);
      if (Math.abs(d2) < 1e-6) break;
      t2 -= x2 / d2;
    }
    // Fallback: bisection
    t0 = 0;
    t1 = 1;
    t2 = x;
    if (t2 < t0) return sampleCurveY(t0);
    if (t2 > t1) return sampleCurveY(t1);
    while (t0 < t1) {
      x2 = sampleCurveX(t2);
      if (Math.abs(x2 - x) < e) return sampleCurveY(t2);
      if (x > x2) {
        t0 = t2;
      } else {
        t1 = t2;
      }
      t2 = (t1 - t0) * 0.5 + t0;
    }
    return sampleCurveY(t2);
  }

  return solve;
}

EASINGS.cubicBezier = cubicBezier;

// Pre-built common cubic-bezier curves
EASINGS.ease = function ease(t) {
  return cubicBezier(0.25, 0.1, 0.25, 1)(t);
};
EASINGS.easeInCubic = function easeInCubic(t) {
  return cubicBezier(0.32, 0, 0.67, 0)(t);
};
EASINGS.easeOutCubic = function easeOutCubic(t) {
  return cubicBezier(0.33, 1, 0.68, 1)(t);
};
EASINGS.easeInOutCubic = function easeInOutCubic(t) {
  return cubicBezier(0.65, 0, 0.35, 1)(t);
};

// --- Steps ---

function stepsFn(count, position) {
  const c = count || 1;
  const pos = position === 'start' ? 0 : 1;
  return function stepsEasing(t) {
    const step = Math.floor(t * c);
    const remainder = t * c - step;
    if (remainder === 0 && t !== 1) {
      return step / c;
    }
    if (pos === 0) {
      return Math.min(t * c, c) / c;
    }
    return Math.max(step, 0) / c;
  };
}

EASINGS.steps = stepsFn;

// Resolve easing by name or config
function resolveEasing(easing) {
  if (typeof easing === 'function') return easing;
  if (typeof easing === 'string') {
    // Check if it's a cubic-bezier(...)
    const cbMatch = easing.match(/^cubic-bezier\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*\)$/);
    if (cbMatch) {
      return cubicBezier(
        parseFloat(cbMatch[1]),
        parseFloat(cbMatch[2]),
        parseFloat(cbMatch[3]),
        parseFloat(cbMatch[4])
      );
    }
    // Check if it's steps(...)
    const stMatch = easing.match(/^steps\(\s*(\d+)\s*(?:,\s*(start|end)\s*)?\)$/);
    if (stMatch) {
      return stepsFn(parseInt(stMatch[1], 10), stMatch[2]);
    }
    // Look up named easing
    if (EASINGS[easing]) return EASINGS[easing];
    // Default
    return EASINGS.easeOut;
  }
  // Handle SpringSolver instance: return a placeholder function.
  // The AnimationPlayer._runJSAnimation() detects SpringSolver via _easingName
  // and creates a duration-aware wrapper at play time, so we just mark it here.
  if (easing && typeof easing === 'object' && typeof easing.solve === 'function') {
    return function springPlaceholder(t) { return t; };
  }
  return EASINGS.easeOut;
}

// Determine if an easing is compatible with WAAPI
function isWAAPICompatible(easing) {
  if (typeof easing === 'string') {
    // Map camelCase to kebab-case for WAAPI compatibility check
    var waapiEasing = easing
      .replace(/[A-Z]/g, function(m) { return '-' + m.toLowerCase(); })
      .replace(/^(-?)power\d/, '');
    // WAAPI supports: linear, ease, ease-in, ease-out, ease-in-out,
    // cubic-bezier(n,n,n,n), steps(n), step-start, step-end
    if (/^(linear|ease|ease-in|ease-out|ease-in-out|step-start|step-end)$/.test(waapiEasing)) return true;
    if (/^cubic-bezier\(/.test(waapiEasing)) return true;
    if (/^steps\(/.test(waapiEasing)) return true;
    return false;
  }
  return false;
}


/* --- 3. AnimationPlayer Class --- */

class AnimationPlayer {
  // Track active animation per element for interrupt handling
  static _activeByElement = new Map();

  constructor(el, config) {
    this.el = el;
    this.config = config;

    // Parse config
    this._from = config.from || {};
    this._to = config.to || {};
    this._over = config.over != null ? config.over : 300;
    this._delay = config.delay || 0;
    this._easingName = config.easing || 'easeOut';
    this._easing = resolveEasing(this._easingName);
    this._iterations = config.iterations != null ? config.iterations : 1;
    this._direction = config.direction || 'normal';
    this._fill = config.fill || 'both';
    this._interrupt = config.interrupt || 'cancel';

    // Labels for seek-by-name
    this._labels = config.labels || null;

    // Multi-stop keyframes (25%, 50%, 75%, etc.)
    this._keyframes = config.keyframes || null;

    // Path animation support
    this._path = config.path || null;
    this._pathRotate = config.pathRotate || false;

    // Repeat delay/refresh
    this._repeatDelay = config.repeatDelay != null ? config.repeatDelay : 0;
    this._repeatRefresh = config.repeatRefresh || false;

    // State
    this.progress = 0;
    this._playing = false;
    this._completed = false;
    this._iteration = 0;
    this._paused = config.paused || false;
    this._callbacks = config.callbacks || {};
    this._rafId = null;
    this._startTime = null;
    this._pauseTime = null;
    this._elapsed = 0;
    this._waapiAnimation = null;
    this._killed = false;
    this._reversed = false;
    this._repeatTimer = null;
    this._queuedNext = null;

    // Pre-compute initial style values if from is not set
    if (Object.keys(this._from).length === 0 && Object.keys(this._to).length > 0) {
      for (const key of Object.keys(this._to)) {
        this._from[key] = this._getCurrentValue(key);
      }
    }

    // Path animation: prepare config — styles applied when play() starts
    if (this._path) {
      // Set aside from/to since we override with offsetDistance below;
      // but DON'T set element styles here — they'll be applied on play().
      this._originalFrom = this._from;
      this._originalTo = this._to;
      this._from = { offsetDistance: '0%' };
      this._to = { offsetDistance: '100%' };
    }
  }

  play() {
    if (this._killed) return;
    // If created with paused: true, don't auto-play — leave for external control
    if (this.config.paused && this._startTime == null) {
      return;
    }

    // Interrupt handling: check for existing animation on same element
    const existing = AnimationPlayer._activeByElement.get(this.el);
    if (existing && existing !== this) {
      switch (this._interrupt) {
        case 'cancel':
          // Stop the existing animation immediately
          existing.kill();
          break;
        case 'finish':
          // Jump existing animation to end state
          existing.seek(1);
          existing._completed = true;
          existing._playing = false;
          existing._onAnimationComplete();
          break;
        case 'compose':
          // Queue this animation to start after existing completes
          existing._queuedNext = this;
          return;  // Don't start yet — wait for current to complete
      }
    }

    // Register as active for this element
    AnimationPlayer._activeByElement.set(this.el, this);

    if (this._completed) {
      this._completed = false;
      this._iteration = 0;
      this.progress = 0;
      this._elapsed = 0;
    }
    if (this._paused) {
      this._paused = false;
      if (this._pauseTime != null) {
        this._elapsed += performance.now() - this._pauseTime;
        this._pauseTime = null;
      }
    }

    // Apply path animation CSS (offset-path, offset-rotate) only when playing,
    // so pausing the animation doesn't permanently alter element layout.
    if (this._path) {
      this.el.style.offsetPath = `path("${this._path}")`;
      if (this._pathRotate) {
        this.el.style.offsetRotate = 'auto';
      }
    }

    // Stagger children animation: animate each child individually with delays
    if (this.config.staggerChildren && this.el.children && this.el.children.length > 0) {
      this._playStaggered();
      return;
    }

    this._playing = true;
    this._startTime = performance.now();

    // Try WAAPI first for compatible easings
    // Check if easing is a SpringSolver instance (object, not string)
    const isSpringSolver = this._easingName && typeof this._easingName === 'object' && typeof this._easingName.solve === 'function';
    const isSpringLike = isSpringSolver || /spring|bounce|elastic|back/i.test(String(this._easingName));
    if (!isSpringLike && isWAAPICompatible(this._easingName)) {
      this._runWAAPI();
    } else {
      this._runJSAnimation();
    }
  }

  /** Animate children with stagger delays */
  _playStaggered() {
    var children = Array.from(this.el.children).filter(function(c) { return c.nodeType === 1; });
    if (children.length === 0) { this._completed = true; return; }
    var staggerDelay = this.config.staggerChildren || 0;
    var baseDelay = this.config.delayChildren || 0;
    var direction = this.config.staggerFrom || 'start';
    var self = this;

    // Build order array based on direction
    var indices = [];
    var n = children.length;
    if (direction === 'start') {
      for (var i = 0; i < n; i++) indices.push(i);
    } else if (direction === 'end') {
      for (var i = n - 1; i >= 0; i--) indices.push(i);
    } else if (direction === 'edges') {
      var left = 0, right = n - 1;
      while (left <= right) {
        if (left === right) { indices.push(left); break; }
        indices.push(left);
        if (left !== right) indices.push(right);
        left++; right--;
      }
    } else { // center
      var mid = Math.floor((n - 1) / 2);
      indices.push(mid);
      var l = mid - 1, r = mid + 1;
      while (l >= 0 || r < n) {
        if (l >= 0) indices.push(l--);
        if (r < n) indices.push(r++);
      }
    }

    var total = indices.length;
    var completed = 0;
    indices.forEach(function(idx, pos) {
      var child = children[idx];
      var delay = baseDelay + pos * staggerDelay;
      setTimeout(function() {
        var childFrom = {};
        for (var k in self._from) childFrom[k] = self._from[k];
        var childTo = {};
        for (var k in self._to) childTo[k] = self._to[k];
        var childConfig = {
          from: childFrom,
          to: childTo,
          over: self._over,
          easing: self._easingName,
          fill: 'forwards'
        };
        var player = new AnimationPlayer(child, childConfig);
        player.play();
        // Track stagger completion
        var checkDone = setInterval(function() {
          if (player._completed) {
            clearInterval(checkDone);
            completed++;
            if (completed >= total) {
              self._completed = true;
              if (self._callbacks.onComplete) self._callbacks.onComplete();
              self._onAnimationComplete();
            }
          }
        }, 16);
      }, delay);
    });
  }

  pause() {
    if (this._waapiAnimation) {
      this._waapiAnimation.pause();
    }
    this._paused = true;
    this._playing = false;
    this._pauseTime = performance.now();
    if (this._rafId) {
      cancelAnimationFrame(this._rafId);
      this._rafId = null;
    }
  }

  resume() {
    if (!this._paused || this._killed) return;
    this._paused = false;
    this._playing = true;
    if (this._waapiAnimation) {
      this._waapiAnimation.play();
    } else {
      this._startTime = performance.now();
      this._runJSAnimation();
    }
  }

  reverse() {
    this._reversed = !this._reversed;
    if (this._waapiAnimation) {
      this._waapiAnimation.reverse();
    } else {
      // Swap from/to
      const tmp = this._from;
      this._from = this._to;
      this._to = tmp;
      this.progress = 1 - this.progress;
    }
  }

  seek(p) {
    // Support seek-by-label-name (string) in addition to numeric progress
    if (typeof p === 'string' && this._labels && this._labels[p] != null) {
      const labelTime = this._labels[p];
      const totalTime = this._over * (this._iterations > 1 ? this._iterations : 1);
      p = totalTime > 0 ? labelTime / totalTime : 0;
    }
    this.progress = Math.max(0, Math.min(1, Number(p) || 0));
    this._applyProperties(this.progress, this._from, this._to);
  }

  /** Called when animation completes naturally — triggers queued next (compose). */
  _onAnimationComplete() {
    if (AnimationPlayer._activeByElement.get(this.el) === this) {
      AnimationPlayer._activeByElement.delete(this.el);
    }
    if (this._queuedNext) {
      const next = this._queuedNext;
      this._queuedNext = null;
      next.play();
    }
  }

  kill() {
    this._killed = true;
    // Clean up path animation CSS to avoid permanent layout changes
    if (this._path) {
      this.el.style.offsetPath = '';
      this.el.style.offsetRotate = '';
    }
    this._playing = false;
    this._completed = false;
    if (this._waapiAnimation) {
      this._waapiAnimation.cancel();
      this._waapiAnimation = null;
    }
    if (this._rafId) {
      cancelAnimationFrame(this._rafId);
      this._rafId = null;
    }
    AnimationPlayer._activeByElement.delete(this.el);
  }

  get playing() {
    return this._playing && !this._paused;
  }

  get completed() {
    return this._completed;
  }

  get iteration() {
    return this._iteration;
  }

  /* --- Internal: WAAPI path --- */

  _runWAAPI() {
    // Support multi-stop keyframes (25%, 50%, 75%, etc.)
    if (this._keyframes && this._keyframes.length > 0) {
      var multiKeyframes = this._keyframes.map(function(kf) {
        var cssKf = { offset: kf.offset };
        var transforms = [];
        for (var key in kf) {
          if (key === 'offset') continue;
          var cssKey = PROPERTY_MAP[key];
          if (!cssKey) continue;
          if (TRANSFORM_PROPS.has(key)) {
            if (key === 'scale' || key === 'scaleX' || key === 'scaleY') {
              transforms.push(cssKey + '(' + kf[key] + ')');
            } else if (key === 'rotate' || key === 'rotateX' || key === 'rotateY' || key === 'rotateZ') {
              transforms.push(cssKey + '(' + kf[key] + 'deg)');
            } else if (key === 'x') {
              transforms.push('translateX(' + kf[key] + 'px)');
            } else if (key === 'y') {
              transforms.push('translateY(' + kf[key] + 'px)');
            }
          } else if (key === 'blur') {
            cssKf.filter = (cssKf.filter || '') + ' blur(' + kf[key] + 'px)';
          } else if (key === 'brightness') {
            cssKf.filter = (cssKf.filter || '') + ' brightness(' + kf[key] + ')';
          } else {
            cssKf[cssKey] = kf[key];
          }
        }
        if (transforms.length > 0) {
          cssKf.transform = transforms.join(' ');
        }
        return cssKf;
      });
      const waapiEasing = this._getWAAPIEasing();
      const options = {
        duration: this._over,
        delay: this._delay,
        easing: waapiEasing,
        iterations: this._iterations,
        direction: this._direction,
        fill: this._fill,
      };
      try {
        this._waapiAnimation = this.el.animate(multiKeyframes, options);
        this._setupWAAPIHandlers();
      } catch (e) {
        this._runJSAnimation();
      }
      return;
    }

    const keyframes = {};
    const cssProps = this._toCSSProperties(this._to);

    // Convert _from (Omni property names) to CSS properties for initial state
    const hasFrom = this._from && Object.keys(this._from).length > 0;
    const fromCSS = hasFrom ? this._toCSSProperties(this._from) : {};

    // Build keyframe at progress 1
    for (const [prop, value] of Object.entries(cssProps)) {
      // Use _from value for the initial keyframe when available (e.g., @enter animations)
      const initial = fromCSS[prop] != null ? fromCSS[prop] : this._getCurrentCSSValue(prop);
      keyframes[prop] = [initial, value];
    }

    // If _from is explicitly set, apply from values as inline styles as well
    // so that the WAAPI keyframe starts from the correct visual state and so
    // that the "to" state interpolates from the expected origin.
    if (hasFrom) {
      this._applyProperties(0, this._from, this._from);
    }

    // If there are no animatable properties, fire callback and exit
    if (Object.keys(keyframes).length === 0) {
      this._completed = true;
      this._playing = false;
      if (this._callbacks.onComplete) this._callbacks.onComplete();
      this._onAnimationComplete();
      return;
    }

    const waapiEasing = this._getWAAPIEasing();
    const options = {
      duration: this._over,
      delay: this._delay,
      easing: waapiEasing,
      iterations: this._iterations,
      direction: this._direction,
      fill: this._fill,
    };

    try {
      this._waapiAnimation = this.el.animate(keyframes, options);
      this._setupWAAPIHandlers();
    } catch (e) {
      // Fallback to JS animation if WAAPI fails
      this._runJSAnimation();
    }
  }

  _setupWAAPIHandlers() {
    this._waapiAnimation.onfinish = () => {
      if (this._killed) return;
      this._completed = true;
      this._playing = false;
      this._iteration = this._waapiAnimation.currentTime != null
        ? Math.floor(this._waapiAnimation.currentTime / this._over) + 1
        : this._iteration + 1;
      if (this._callbacks.onComplete) this._callbacks.onComplete();
      this._onAnimationComplete();
    };

    this._waapiAnimation.oncancel = () => {
      this._playing = false;
    };
  }

  /* --- Internal: JS requestAnimationFrame path --- */

  _runJSAnimation() {
    if (this._killed) return;

    var self = this;
    const from = this._from;
    const to = this._to;
    const over = this._over;
    const delay = this._delay;
    // If easing is a SpringSolver object, create a duration-aware wrapper
    const easing = this._easingName && typeof this._easingName === 'object' && typeof this._easingName.solve === 'function'
      ? (function(solver, dur) {
          return function(t) { return solver.solve(0, 1, t * (dur / 1000)); };
        })(this._easingName, this._over)
      : this._easing;
    let startTime = performance.now();
    const fromCache = {};

    // Compute initial from values
    for (const key of Object.keys(to)) {
      fromCache[key] = from[key] != null ? from[key] : this._getCurrentValue(key);
    }

    let delayPassed = false;
    let delayStartTime = startTime;

    function frame(time, self) {
      if (self._killed) return;

      // Handle delay (only on first iteration)
      if (!delayPassed) {
        if (time - delayStartTime < delay) {
          self._rafId = requestAnimationFrame(function(t) { frame(t, self); });
          return;
        }
        delayPassed = true;
        startTime = time - delay;
      }

      const elapsed = time - startTime;
      let rawProgress = elapsed / over;
      let iterationChanged = false;

      // Handle iteration count and direction
      if (rawProgress >= 1) {
        self._iteration++;
        iterationChanged = true;
        if (self._iteration >= self._iterations) {
          rawProgress = 1;
          self.progress = 1;
          self._applyProperties(1, fromCache, to);
          self._completed = true;
          self._playing = false;
          if (self._callbacks.onComplete) self._callbacks.onComplete();
          self._onAnimationComplete();
          return;
        }
        // Check for repeatDelay before next iteration
        if (self._repeatDelay > 0) {
          clearTimeout(self._repeatTimer);
          self._repeatTimer = setTimeout(() => {
            // If repeatRefresh, re-read from values
            if (self._repeatRefresh) {
              for (const key of Object.keys(to)) {
                fromCache[key] = self._getCurrentValue(key);
              }
            }
            startTime = performance.now();
            rawProgress = 0;
            self.progress = 0;
            self._startTime = performance.now();
            self._rafId = requestAnimationFrame(function(t) { frame(t, self); });
          }, self._repeatDelay);
          return;
        }
        // If repeatRefresh, re-read from values at iteration boundary
        if (self._repeatRefresh) {
          for (const key of Object.keys(to)) {
            fromCache[key] = self._getCurrentValue(key);
          }
        }
        // Reset for next iteration
        startTime = time;
        rawProgress = 0;
        self.progress = 0;
        self._startTime = performance.now();
      }

      self.progress = rawProgress;
      const easedProgress = easing(rawProgress);
      self._applyProperties(easedProgress, fromCache, to);

      self._rafId = requestAnimationFrame(function(t) { frame(t, self); });
    }

    this._rafId = requestAnimationFrame(function(t) { frame(t, self); });
  }

  /* --- Property handling --- */

  _getCurrentValue(key) {
    if (!this.el) return 0;
    const cssKey = PROPERTY_MAP[key] || key;
    // For transform properties, detect current transform
    if (TRANSFORM_PROPS.has(key)) {
      const current = this.el.style.transform;
      if (current && current !== 'none') {
        const match = current.match(new RegExp(cssKey.replace(/[A-Z]/g, '-$&').toLowerCase() + '\\(([^)]+)\\)'));
        if (match) return parseFloat(match[1]) || 0;
      }
      return 0;
    }
    // For opacity and other simple numeric properties
    if (key === 'opacity') {
      const val = this.el.style.opacity;
      return val !== '' ? parseFloat(val) : 1;
    }
    // For filter properties (blur, brightness)
    if (key === 'blur' || key === 'brightness') {
      const current = this.el.style.filter;
      if (current) {
        const match = current.match(new RegExp(key + '\\(([^)]+)\\)'));
        if (match) return parseFloat(match[1]) || 0;
      }
      return key === 'blur' ? 0 : 1;
    }
    // For color/boxShadow properties, read computed value as string for interpolation
    if (key === 'color' || key === 'backgroundColor' || key === 'borderColor' || key === 'boxShadow') {
      const inline = this.el.style[cssKey];
      if (inline && inline !== '') return inline;
      const computed = getComputedStyle(this.el);
      return computed[cssKey] || '#000000';
    }
    // For other CSS properties
    const computed = getComputedStyle(this.el);
    const propKey = cssKey.replace(/[A-Z]/g, function(m) { return '-' + m.toLowerCase(); });
    const val = computed.getPropertyValue(propKey);
    const parsed = parseFloat(val);
    return isNaN(parsed) ? 0 : parsed;
  }

  _getCurrentCSSValue(propName) {
    if (!this.el) return propName === 'transform' ? 'none' : '0';
    const val = this.el.style[propName];
    if (val && val !== '') return val;
    const computed = getComputedStyle(this.el);
    const v = computed[propName];
    if (v && v !== '') return v;
    // For transform, 'none' is the correct zero value, not '0'
    if (propName === 'transform') return 'none';
    return '0';
  }

  _toCSSProperties(props) {
    const result = {};
    const transforms = [];

    for (const [key, value] of Object.entries(props)) {
      const cssKey = PROPERTY_MAP[key];
      if (!cssKey) continue;

      if (TRANSFORM_PROPS.has(key)) {
        transforms.push({ key, cssKey, value });
      } else if (key === 'blur') {
        result.filter = 'blur(' + value + 'px)';
      } else if (key === 'brightness') {
        result.filter = 'brightness(' + value + ')';
      } else {
        result[cssKey] = this._formatValue(key, value);
      }
    }

    // Compose transforms into single transform string
    if (transforms.length > 0) {
      const parts = transforms.map(function(t) {
        if (t.key === 'scale' || t.key === 'scaleX' || t.key === 'scaleY') {
          return t.cssKey + '(' + t.value + ')';
        }
        if (t.key === 'rotate' || t.key === 'rotateX' || t.key === 'rotateY' || t.key === 'rotateZ') {
          return t.cssKey + '(' + t.value + 'deg)';
        }
        if (t.key === 'x') {
          return 'translateX(' + t.value + 'px)';
        }
        if (t.key === 'y') {
          return 'translateY(' + t.value + 'px)';
        }
        return t.cssKey + '(' + t.value + ')';
      });
      result.transform = parts.join(' ');
    }

    return result;
  }

  _formatValue(key, value) {
    // If value is already a string (e.g., 'red', '50%'), return as-is
    if (typeof value === 'string') return value;
    // For pixel-based properties
    if (PIXEL_PROPS.has(key)) {
      return value + 'px';
    }
    // For unitless or automatically unit-resolved properties
    return String(value);
  }

  _applyProperties(progress, from, to) {
    if (!this.el) return;
    const transforms = [];
    const filters = [];

    for (const key of Object.keys(to)) {
      const fromVal = from[key] != null ? from[key] : this._getCurrentValue(key);
      const toVal = to[key];
      const interpolated = this._interpolate(fromVal, toVal, progress);
      const cssKey = PROPERTY_MAP[key];
      if (!cssKey) continue;

      if (TRANSFORM_PROPS.has(key)) {
        if (key === 'scale' || key === 'scaleX' || key === 'scaleY') {
          transforms.push(cssKey + '(' + interpolated + ')');
        } else if (key === 'rotate' || key === 'rotateX' || key === 'rotateY' || key === 'rotateZ') {
          transforms.push(cssKey + '(' + interpolated + 'deg)');
        } else if (key === 'x') {
          transforms.push('translateX(' + interpolated + 'px)');
        } else if (key === 'y') {
          transforms.push('translateY(' + interpolated + 'px)');
        }
      } else if (key === 'blur') {
        filters.push('blur(' + interpolated + 'px)');
      } else if (key === 'brightness') {
        filters.push('brightness(' + interpolated + ')');
      } else {
        this.el.style[cssKey] = this._formatValue(key, interpolated);
      }
    }

    if (transforms.length > 0) {
      this.el.style.transform = transforms.join(' ');
    }
    if (filters.length > 0) {
      this.el.style.filter = filters.join(' ');
    }
  }

  _interpolate(from, to, t) {
    if (typeof from === 'string' || typeof to === 'string') {
      // Handle hex color interpolation (#RRGGBB or #RGB)
      const hexMatch = /^#([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})$/;
      const hexShort = /^#([0-9a-fA-F])([0-9a-fA-F])([0-9a-fA-F])$/;
      // Handle rgb(r, g, b) format from computed styles
      const rgbMatch = /^rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)$/;
      var pr, pg, pb;
      if (typeof from === 'string' && typeof to === 'string') {
        // Parse 'from' color
        var m1 = from.match(hexMatch);
        if (m1) { pr = parseInt(m1[1],16); pg = parseInt(m1[2],16); pb = parseInt(m1[3],16); }
        else { m1 = from.match(hexShort);
          if (m1) { pr = parseInt(m1[1]+m1[1],16); pg = parseInt(m1[2]+m1[2],16); pb = parseInt(m1[3]+m1[3],16); }
          else { m1 = from.match(rgbMatch);
            if (m1) { pr = parseInt(m1[1]); pg = parseInt(m1[2]); pb = parseInt(m1[3]); }
          }
        }
        // Parse 'to' color
        var m2 = to.match(hexMatch);
        if (m2) { var tr = parseInt(m2[1],16), tg = parseInt(m2[2],16), tb = parseInt(m2[3],16); }
        else { m2 = to.match(hexShort);
          if (m2) { var tr = parseInt(m2[1]+m2[1],16), tg = parseInt(m2[2]+m2[2],16), tb = parseInt(m2[3]+m2[3],16); }
          else { m2 = to.match(rgbMatch);
            if (m2) { var tr = parseInt(m2[1]), tg = parseInt(m2[2]), tb = parseInt(m2[3]); }
          }
        }
        if (typeof pr !== 'undefined' && typeof tr !== 'undefined') {
          var r = Math.round(pr + (tr - pr) * t);
          var g = Math.round(pg + (tg - pg) * t);
          var b = Math.round(pb + (tb - pb) * t);
          return '#' + [r,g,b].map(function(c){ return c.toString(16).padStart(2,'0'); }).join('');
        }
      }
      // Fallback: snap at midpoint for unknown string types
      return t < 0.5 ? from : to;
    }
    return from + (to - from) * t;
  }

  _getWAAPIEasing() {
    // Convert OmniView easing names to WAAPI-compatible strings
    const map = {
      linear: 'linear',
      easeIn: 'ease-in',
      easeOut: 'ease-out',
      easeInOut: 'ease-in-out',
      ease: 'ease',
      power1In: 'ease-in',
      power1Out: 'ease-out',
      power1InOut: 'ease-in-out',
      power2In: 'ease-in',
      power2Out: 'ease-out',
      power2InOut: 'ease-in-out',
      easeInCubic: 'cubic-bezier(0.32, 0, 0.67, 0)',
      easeOutCubic: 'cubic-bezier(0.33, 1, 0.68, 1)',
      easeInOutCubic: 'cubic-bezier(0.65, 0, 0.35, 1)',
    };
    return map[this._easingName] || this._easingName;
  }
}

/* --- Property Name Mapping --- */

const PROPERTY_MAP = {
  x: 'translateX',
  y: 'translateY',
  scale: 'scale',
  scaleX: 'scaleX',
  scaleY: 'scaleY',
  rotate: 'rotate',
  rotateX: 'rotateX',
  rotateY: 'rotateY',
  rotateZ: 'rotateZ',
  opacity: 'opacity',
  width: 'width',
  height: 'height',
  backgroundColor: 'backgroundColor',
  background: 'backgroundColor',
  color: 'color',
  borderColor: 'borderColor',
  borderRadius: 'borderRadius',
  margin: 'margin',
  padding: 'padding',
  blur: 'blur',
  brightness: 'brightness',
  boxShadow: 'boxShadow',
};

const TRANSFORM_PROPS = new Set([
  'x', 'y', 'scale', 'scaleX', 'scaleY',
  'rotate', 'rotateX', 'rotateY', 'rotateZ',
]);

const PIXEL_PROPS = new Set([
  'width', 'height', 'margin', 'padding', 'borderRadius',
  'x', 'y',
]);


/* --- 4. OmniView Runtime Integration --- */

// Global animation registry
window.__omni_animations = new Map();
// Timeline labels dictionary (label name → delay offset)
window.__omni_timeline_labels = new Map();

// Create animation from motion IR config
function __omni_createAnimation(nid, motionConfig) {
  var el = __omni_el(nid);
  if (!el) return null;
  var player = new AnimationPlayer(el, motionConfig);
  var key = nid + '_' + (motionConfig.trigger || 'default');
  __omni_animations.set(key, player);
  return player;
}

// Get animation handle for ref API
function __omni_getAnimation(nid, trigger) {
  return __omni_animations.get(nid + '_' + (trigger || 'default'));
}

// Play @enter animations for all child elements within a parent node.
// Used when an @if branch becomes visible, to replay animations that
// were skipped during mount because the element was hidden.
function playBranchEnters(parent) {
  if (!parent) return;
  var children = parent.querySelectorAll('[data-nid]');
  children.forEach(function(child) {
    var nid = child.dataset.nid;
    if (!nid) return;
    var player = __omni_animations.get(nid + '_enter');
    if (player) player.play();
  });
}

// Trigger all animations matching a condition
function __omni_triggerAnimations(triggerName) {
  __omni_animations.forEach(function(player, key) {
    if (key.endsWith('_' + triggerName)) {
      player.play();
    }
  });
}

// InView observer registry for cleanup
var __omni_inview_observers = [];

// Parse scroll trigger config string (e.g. "top 80%", "center 50%", "bottom 20%")
// Returns the absolute scroll Y position where the given edge of the element
// reaches the given offset within the viewport.
function __omni_parseScrollTrigger(cfg, el, vh) {
  if (!cfg || typeof cfg !== 'string') return 0;
  var parts = cfg.trim().split(/\s+/);
  if (parts.length < 2) return 0;
  var edge = parts[0];
  var offset = parts[1];
  var rect = el.getBoundingClientRect();
  var scrollY = window.pageYOffset || window.scrollY;

  var elEdge;
  if (edge === 'top') elEdge = rect.top;
  else if (edge === 'center') elEdge = rect.top + rect.height / 2;
  else if (edge === 'bottom') elEdge = rect.bottom;
  else elEdge = rect.top;

  var absEdge = scrollY + elEdge;

  if (offset.endsWith('%')) {
    return absEdge - vh * (parseFloat(offset) / 100);
  } else if (offset === 'bottom') {
    return absEdge - vh;
  } else if (offset === 'top') {
    return absEdge;
  }
  return absEdge - vh * 0.5;
}

// Cleanup animation resources for a node
function __omni_removeAnimation(nid) {
  var prefix = nid + '_';
  __omni_animations.forEach(function(player, key) {
    if (key.startsWith(prefix)) {
      player.kill();
      __omni_animations.delete(key);
    }
  });
}

// Reduced motion detection
var __omni_reducedMotion = window.matchMedia &&
  window.matchMedia('(prefers-reduced-motion: reduce)').matches;

// Listen for changes to reduced motion preference
if (window.matchMedia) {
  window.matchMedia('(prefers-reduced-motion: reduce)').addEventListener('change', function(e) {
    __omni_reducedMotion = e.matches;
    // Reduce all active animations when user enables reduced motion
    if (e.matches) {
      __omni_animations.forEach(function(player) {
        if (player.playing) {
          // Seek to end instantly
          player.seek(1);
          player.kill();
        }
      });
    }
  });
}

// Export for codegen use
window.__omni_createAnimation = __omni_createAnimation;
window.__omni_getAnimation = __omni_getAnimation;
window.__omni_triggerAnimations = __omni_triggerAnimations;
window.__omni_removeAnimation = __omni_removeAnimation;
window.__omni_reducedMotion = __omni_reducedMotion;
window.__omni_inview_observers = __omni_inview_observers;
window.__omni_parseScrollTrigger = __omni_parseScrollTrigger;
window.playBranchEnters = playBranchEnters;

// FLIP animation for @for list reordering
window.__omni_flip_state = new Map();
function __omni_flip_before(containerSelector) {
  const container = document.querySelector(containerSelector);
  if (!container) return;
  container.querySelectorAll('[data-for-key]').forEach(function(el) {
    window.__omni_flip_state.set(el.getAttribute('data-for-key'), {
      rect: el.getBoundingClientRect(),
      element: el
    });
  });
}
window.__omni_flip_before = __omni_flip_before;

function __omni_flip_after(containerSelector, duration, easing) {
  duration = duration || 400;
  easing = easing || 'ease-out';
  const container = document.querySelector(containerSelector);
  if (!container) return;
  container.querySelectorAll('[data-for-key]').forEach(function(el) {
    const key = el.getAttribute('data-for-key');
    const prev = window.__omni_flip_state.get(key);
    if (!prev) return;
    const newRect = el.getBoundingClientRect();
    const dx = prev.rect.left - newRect.left;
    const dy = prev.rect.top - newRect.top;
    if (dx === 0 && dy === 0) return;
    el.style.transform = 'translate(' + dx + 'px, ' + dy + 'px)';
    el.style.transition = 'none';
    el.offsetHeight; // force reflow
    el.style.transition = 'transform ' + duration + 'ms ' + easing;
    el.style.transform = 'translate(0, 0)';
    el.addEventListener('transitionend', function() {
      el.style.transition = '';
      el.style.transform = '';
    }, { once: true });
  });
  window.__omni_flip_state.clear();
}
window.__omni_flip_after = __omni_flip_after;

// @hero shared element transition
window.__omni_hero_state = new Map();
function __omni_hero_before() {
  document.querySelectorAll('[data-hero]').forEach(function(el) {
    var tag = el.getAttribute('data-hero');
    var isVisible = el.offsetParent !== null || el.getBoundingClientRect().width > 0;
    if (!isVisible) return;
    if (!window.__omni_hero_state.has(tag)) {
      var r = el.getBoundingClientRect();
      window.__omni_hero_state.set(tag, {
        rect: r,
        element: el
      });
    }
  });
}

function __omni_hero_after() {
  window.__omni_hero_state.forEach(function(prev, tag) {
    // Find the VISIBLE element with this hero tag (display !== 'none')
    var visible = null;
    document.querySelectorAll('[data-hero="' + tag + '"]').forEach(function(el) {
      if (el.offsetParent !== null || el.getBoundingClientRect().width > 0) {
        visible = el;
      }
    });
    if (!visible || visible === prev.element) return;
    var newRect = visible.getBoundingClientRect();
    var dx = prev.rect.left - newRect.left;
    var dy = prev.rect.top - newRect.top;
    var dw = prev.rect.width / newRect.width || 1;
    var dh = prev.rect.height / newRect.height || 1;
    if (dx === 0 && dy === 0 && dw === 1 && dh === 1) return;
    var dur = 400;
    var ease = 'ease-out';
    var trans = visible.getAttribute('data-hero-transition');
    if (trans) {
      // Find the motion definition that matches this transition name
      window.__omni_animations.forEach(function(anim, key) {
        if (key.indexOf('_' + trans) !== -1 && anim.config && anim.config.over) {
          dur = anim.config.over;
          if (anim.config.easing) ease = anim.config.easing;
        }
      });
    }
    // Apply FLIP: start at old position/scale
    visible.style.transition = 'none';
    visible.style.transformOrigin = '0 0';
    visible.style.transform = 'translate(' + dx + 'px, ' + dy + 'px) scale(' + dw + ', ' + dh + ')';
    void visible.offsetHeight;
    // Use WAAPI for the animation instead of CSS transitions.
    // This avoids the browser optimization issues with CSS transitions
    // that cause the initial transform to be skipped.
    if (visible.animate) {
      try {
        var waapiEase = ease;
        var easeMap = {
          'easeInOut': 'ease-in-out', 'easeIn': 'ease-in', 'easeOut': 'ease-out',
          'ease': 'ease', 'linear': 'linear',
          'power1In': 'ease-in', 'power1Out': 'ease-out', 'power1InOut': 'ease-in-out',
          'power2In': 'ease-in', 'power2Out': 'ease-out', 'power2InOut': 'ease-in-out',
          'power3In': 'ease-in', 'power3Out': 'ease-out', 'power3InOut': 'ease-in-out',
          'power4In': 'ease-in', 'power4Out': 'ease-out', 'power4InOut': 'ease-in-out',
        };
        if (easeMap[waapiEase]) waapiEase = easeMap[waapiEase];
        // Use keyframe list with offset 0 and 1
        var kfs = [
          { transform: 'translate(' + dx + 'px, ' + dy + 'px) scale(' + dw + ', ' + dh + ')', offset: 0 },
          { transform: 'translate(0, 0) scale(1, 1)', offset: 1 }
        ];
        var flipAnim = visible.animate(kfs, {
          duration: dur,
          easing: waapiEase,
          fill: 'both'
        });
        flipAnim.onfinish = function() {
          visible.style.transition = '';
          visible.style.transform = '';
          visible.style.transformOrigin = '';
        };
      } catch(e) {}
    }
    visible.addEventListener('transitionend', function() {
      visible.style.transition = '';
      visible.style.transform = '';
      visible.style.transformOrigin = '';
    }, { once: true });
  });
  window.__omni_hero_state.clear();
}

// Watch the root for DOM changes involving data-hero elements
var __omni_hero_observer = new MutationObserver(function(mutations) {
  var hasHeroChange = false;
  mutations.forEach(function(m) {
    if (m.type === 'childList' || m.type === 'attributes') {
      m.addedNodes.forEach(function(n) {
        if (n.nodeType === 1 && (n.getAttribute && n.getAttribute('data-hero'))) hasHeroChange = true;
      });
    }
  });
  if (hasHeroChange) {
    __omni_hero_after();
  }
});
// Start observer once DOM is ready
(function() {
  var root = document.getElementById('root');
  if (root) {
    __omni_hero_observer.observe(root, { childList: true, subtree: true, attributes: true, attributeFilter: ['data-hero'] });
  }
})();

window.__omni_hero_before = __omni_hero_before;
window.__omni_hero_after = __omni_hero_after;
window.SpringSolver = SpringSolver;
window.AnimationPlayer = AnimationPlayer;
window.EASINGS = EASINGS;
window.resolveEasing = resolveEasing;

})();
