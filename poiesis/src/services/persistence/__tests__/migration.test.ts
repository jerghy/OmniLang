// poiesis/src/services/persistence/__tests__/migration.test.ts
import { describe, it, expect } from "vitest";
import { migrateV2ToV3, needsMigration } from "../migration";

describe("needsMigration", () => {
  it("returns true for v2", () => {
    expect(needsMigration({ version: 2 } as any)).toBe(true);
  });

  it("returns false for v3", () => {
    expect(needsMigration({ version: 3 } as any)).toBe(false);
  });
});

describe("migrateV2ToV3", () => {
  it("upgrades version and adds empty codeFlowGraphs", () => {
    const v2 = {
      version: 2 as const,
      nodes: {
        pid_1: { pos: { x: 100, y: 200 } },
      },
      wires: {},
      architecture: {
        mode: "overall" as const,
        expandedBubbles: [],
        focusComponent: null,
        camera: { x: 0, y: 0, zoom: 1 },
      },
      canvasStates: {},
      panels: { leftWidth: 240, rightWidth: 300 },
    };

    const v3 = migrateV2ToV3(v2 as Record<string, unknown> & { version: 2 });
    expect(v3.version).toBe(3);
    expect(v3.codeFlowGraphs).toEqual({});
    // Existing data preserved
    expect(v3.nodes["pid_1"].pos).toEqual({ x: 100, y: 200 });
  });
});
