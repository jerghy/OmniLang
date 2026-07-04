// core/types/index.ts
// Barrel export for v2 type system.

export * from "./pid";
export * from "./architecture";
export * from "./design-page";
export * from "./project-json";
export * from "./node-flow";

// Keep legacy types that are still needed for migration
export * from "./element";
export * from "./wire";
export * from "./style";

// Proper CodeTab interface from codegraph module
export * from "./codegraph";
