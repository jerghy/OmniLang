import { describe, it, expect } from "vitest";
import { parseOps } from "../code-op-parser";
import { emitOps } from "../code-op-emitter";

// Real Omni source patterns from learn-omniview production code
const REAL_SOURCES = [
  {
    name: "validate_not_empty",
    source: `for field in fields {
    if field == "" {
        return "错误: 发现空字段";
    }
}
return "通过";`,
  },
  {
    name: "calculate_average",
    source: `let sum = 0.0;
let mut i = 0;
while i < values.len() {
    sum = sum + values.get(i);
    i = i + 1;
}
if count != 0 {
    return sum / count;
}
return 0.0;`,
  },
  {
    name: "count_valid",
    source: `let mut count = 0;
for value in values {
    if value == "" {
        continue;
    }
    if value.len() < min_len {
        continue;
    }
    count = count + 1;
}
return count;`,
  },
  {
    name: "if_else_chain",
    source: `if score >= 90 {
    return "A";
} else if score >= 80 {
    return "B";
} else if score >= 70 {
    return "C";
} else {
    return "F";
}`,
  },
  {
    name: "nested_loops",
    source: `let mut result = 0;
for row in matrix {
    let mut row_sum = 0;
    for cell in row {
        row_sum = row_sum + cell;
    }
    result = result + row_sum;
}
return result;`,
  },
  {
    name: "multiple_method_calls",
    source: `let x = data.get(0);
let y = data.get(1);
x = x.trim();
y = y.trim();
if x.len() > 0 {
    return x;
}
return y;`,
  },
  {
    name: "while_with_method_in_condition",
    source: `let mut i = 0;
while i < items.len() {
    process(items.get(i));
    i = i + 1;
}`,
  },
  {
    name: "complex_nested_condition",
    source: `let mut count = 0;
for item in items {
    if item != "" {
        if item.len() > 3 {
            count = count + 1;
        }
    } else {
        continue;
    }
}
return count;`,
  },
];

describe("CodeOp round-trip with real Omni sources", () => {
  for (const { name, source } of REAL_SOURCES) {
    it(`round-trips ${name}`, () => {
      // First parse: source -> ops
      const ops = parseOps(source);
      expect(ops.length).toBeGreaterThan(0);

      // Emit: ops -> source
      const emitted = emitOps(ops);

      // Second parse: emitted source -> ops2
      const ops2 = parseOps(emitted);

      // Compare kinds (structural equivalence, PIDs differ)
      expect(ops2.map(o => o.kind)).toEqual(ops.map(o => o.kind));
    });

    it(`${name} re-parses emitted source without errors`, () => {
      const ops = parseOps(source);
      const emitted = emitOps(ops);

      // Re-parse emitted source - should produce valid ops
      const reOps = parseOps(emitted);
      expect(reOps.length).toBeGreaterThan(0);
    });
  }
});
