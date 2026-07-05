/**
 * OmniLang 内置代码片段定义。
 *
 * 按上下文分类：
 *   fn_body     — 函数体内可用片段
 *   store_action — store action 体内
 *   computed    — computed 块内
 *   view_interpolation — view 模板插值
 *   if_condition — if 条件表达式
 *   top_level   — 顶层声明
 *   data_fetch  — 数据获取
 *   state_mgmt  — 状态管理
 */

export interface OmniSnippet {
  id: string;
  label: string;
  description: string;
  /** Context where this snippet is relevant */
  context: SnippetContext;
  /** The code to insert (may contain $1, $2 placeholders) */
  body: string;
  /** Target frameworks this works for (empty = all) */
  targets?: string[];
}

export type SnippetContext =
  | "fn_body"
  | "store_action"
  | "computed"
  | "view_interpolation"
  | "if_condition"
  | "top_level"
  | "data_fetch"
  | "state_mgmt";

export const BUILTIN_SNIPPETS: OmniSnippet[] = [
  // ── Top Level ──────────────────────────────────
  {
    id: "comp-react",
    label: "React Component",
    description: "OmniView component targeting React",
    context: "top_level",
    body: `component $1 {
    view {
        Container {
            Text { label: "$2" }
        }
    }
}`,
  },
  {
    id: "store-counter",
    label: "Counter Store",
    description: "Signal + actions store (like zustand/pinia)",
    context: "top_level",
    body: `store $1 {
    signal count: int = 0;

    computed doubleCount: int {
        return this.count * 2;
    }

    action increment() {
        this.count += 1;
    }

    action decrement() {
        this.count -= 1;
    }

    action reset() {
        this.count = 0;
    }
}`,
  },
  {
    id: "fn-async",
    label: "Async Function",
    description: "async fn with error handling",
    context: "top_level",
    body: `fn $1($2) {
    try {
        const response = await fetch("$3");
        const data = await response.json();
        return data;
    } catch (err) {
        throw err;
    }
}`,
  },

  // ── Function Body ──────────────────────────────
  {
    id: "try-catch",
    label: "Try-Catch",
    description: "Error handling block",
    context: "fn_body",
    body: `try {
    $1
} catch (err) {
    $2
}`,
  },
  {
    id: "if-else-chain",
    label: "If-Else Chain",
    description: "Multi-branch conditional",
    context: "fn_body",
    body: `if ($1) {
    $2
} else if ($3) {
    $4
} else {
    $5
}`,
  },
  {
    id: "match-expr",
    label: "Match Expression",
    description: "Pattern matching",
    context: "fn_body",
    body: `match $1 {
    case $2 => $3;
    case $4 => $5;
    default => $6;
}`,
  },
  {
    id: "for-each",
    label: "ForEach Loop",
    description: "Iterate over collection",
    context: "fn_body",
    body: `for ($1 in $2) {
    $3
}`,
  },
  {
    id: "map-filter",
    label: "Map + Filter",
    description: "Functional data pipeline",
    context: "fn_body",
    body: `$1
    .filter(($2) => $3)
    .map(($4) => $5)$6`,
  },

  // ── Store Action ───────────────────────────────
  {
    id: "action-crud",
    label: "CRUD Action",
    description: "Async store action with loading state",
    context: "store_action",
    body: `action $1($2) {
    this.loading = true;
    try {
        $3
    } catch (err) {
        this.error = err.message;
    } finally {
        this.loading = false;
    }
}`,
  },
  {
    id: "action-toggle",
    label: "Toggle Action",
    description: "Boolean toggle action",
    context: "store_action",
    body: `action $1() {
    this.$2 = !this.$2;
}`,
  },
  {
    id: "action-setter",
    label: "Setter Action",
    description: "Simple setter with validation",
    context: "store_action",
    body: `action $1(value: $2) {
    if (value != null) {
        this.$3 = value;
    }
}`,
  },

  // ── Computed ───────────────────────────────────
  {
    id: "computed-filter",
    label: "Filtered List",
    description: "Derived filtered list from signal",
    context: "computed",
    body: `computed $1: List<$2> {
    return this.$3.filter(($4) => $5);
}`,
  },
  {
    id: "computed-count",
    label: "Count / Aggregation",
    description: "Derived count/aggregate",
    context: "computed",
    body: `computed $1: int {
    return this.$2.length;
}`,
  },
  {
    id: "computed-fullname",
    label: "Full Name",
    description: "Concatenate fields",
    context: "computed",
    body: `computed $1: String {
    return "\${this.$2} \${this.$3}";
}`,
  },

  // ── View Interpolation ─────────────────────────
  {
    id: "view-bind",
    label: "Bind Value",
    description: "Two-way binding via bind:value",
    context: "view_interpolation",
    body: `Input {
    bind:value={$1}
    placeholder="$2"
    test-id="$3"
}`,
  },
  {
    id: "view-event",
    label: "Event Handler",
    description: "onPress with inline fn",
    context: "view_interpolation",
    body: `Button {
    label: "$1"
    onPress={fn() {
        $2
    }}
    test-id="$3"
}`,
  },
  {
    id: "view-for",
    label: "For Directive",
    description: "List rendering with @for",
    context: "view_interpolation",
    body: `List {
    @for ($1 in $2) {
        ListItem {
            Text { label: "{$1.$3}" }
        }
    }
}`,
  },
  {
    id: "view-if",
    label: "Conditional Render",
    description: "Conditional with @if",
    context: "view_interpolation",
    body: `@if ($1) {
    Text { label: "$2" }
} @else {
    Text { label: "$3" }
}`,
  },
  {
    id: "view-flex-row",
    label: "Flex Row",
    description: "Horizontal flex container",
    context: "view_interpolation",
    body: `Flex {
    direction: "row";
    gap: "$1";
    align: "center";
    $2
}`,
  },
  {
    id: "view-card",
    label: "div (Card)",
    description: "Container div with shadow styling",
    context: "view_interpolation",
    body: `div {
    padding: "$1";
    border_radius: "$2";
    $3
}`,
  },

  // ── Data Fetch ─────────────────────────────────
  {
    id: "data-fetch-get",
    label: "GET Request",
    description: "fetch GET with JSON parse",
    context: "data_fetch",
    body: `const response = await fetch("$1");
if (!response.ok) {
    throw Exception("HTTP $2: \${response.status}");
}
const $3 = await response.json();`,
  },
  {
    id: "data-fetch-post",
    label: "POST Request",
    description: "fetch POST with JSON body",
    context: "data_fetch",
    body: `const response = await fetch("$1", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify($2),
});
if (!response.ok) {
    throw Exception("POST failed: \${response.status}");
}
const $3 = await response.json();`,
  },
  {
    id: "data-fetch-poll",
    label: "Poll / Interval",
    description: "Periodic data polling",
    context: "data_fetch",
    body: `const interval = setInterval(async () => {
    try {
        $1
    } catch (err) {
        clearInterval(interval);
    }
}, $2);`,
  },

  // ── State Management ───────────────────────────
  {
    id: "state-loading",
    label: "Loading State Pattern",
    description: "Async loading / error / data trio",
    context: "state_mgmt",
    body: `signal $1: $2 | null = null;
signal ${1}Loading: bool = false;
signal ${1}Error: String | null = null;`,
  },
  {
    id: "state-persist",
    label: "Persisted Signal",
    description: "Signal with localStorage persistence",
    context: "state_mgmt",
    body: `signal $1: $2 = _load$1();

fn _load$1(): $2 {
    const raw = localStorage.getItem("$1");
    return raw ? JSON.parse(raw) : $3;
}

action _$1(value: $2) {
    this.$1 = value;
    localStorage.setItem("$1", JSON.stringify(value));
}`,
  },
];

/** Context labels for display */
export const CONTEXT_LABELS: Record<SnippetContext, string> = {
  top_level: "顶层声明",
  fn_body: "函数体",
  store_action: "Store Action",
  computed: "Computed",
  view_interpolation: "View 模板",
  if_condition: "条件表达式",
  data_fetch: "数据获取",
  state_mgmt: "状态管理",
};

/** Context icon emoji */
export const CONTEXT_ICONS: Record<SnippetContext, string> = {
  top_level: "📦",
  fn_body: "⚡",
  store_action: "🔄",
  computed: "🧮",
  view_interpolation: "🎨",
  if_condition: "🔀",
  data_fetch: "🌐",
  state_mgmt: "🗄️",
};
