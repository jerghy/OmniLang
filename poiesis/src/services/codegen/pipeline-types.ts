// poiesis/src/services/codegen/pipeline-types.ts
// Public type interfaces for the codegen pipeline.
// Extracted from pipeline.ts for type-safe usage without @ts-nocheck.

import type { ElementData, WireData, CodeTab } from "../../core/types";
import type { ComponentMeta, StageResult } from "./types";
import type { PidMap } from "../../core/types/pid";
import type { StoreDefinition } from "../../core/types/store";
import type { CodeFlowGroup } from "../../core/types/project-json";

/**
 * Options for generating a single OmniLang source string from design data.
 */
export interface GenerateOmniSourceOptions {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
  rootOrder?: string[];
  componentMeta?: ComponentMeta;
  /** Existing PID map to reuse PIDs across regenerations for stability. */
  existingPidMap?: PidMap;
  /** Design system name for class reference prefixing and import emission. */
  designName?: string;
  /** Inline store definitions to emit inside the component body. */
  inlineStores?: StoreDefinition[];
  /**
   * 已知 store 名（src/stores/*.omni）：语法节点 props/source 里的 "Store.信号"
   * 引用据此收集进 storeRefs → 生成 import（否则 WASM 门禁 Undeclared variable 拦截保存）。
   */
  knownStores?: string[];
  /** 组注释数据（键=fn 名）：保存时从 store.codeFlowGroups 传入。 */
  codeFlowGroups?: Record<string, CodeFlowGroup[]>;
}

/**
 * Options for generating multi-file OmniLang output.
 */
export interface GenerateOmniFilesOptions {
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  componentName: string;
  rootOrder?: string[];
  componentMeta?: ComponentMeta;
  existingPidMap?: PidMap;
  /** Design system name for class reference prefixing and import emission. */
  designName?: string;
  /** Inline store definitions to emit inside the component body. */
  inlineStores?: StoreDefinition[];
  /** 已知 store 名（src/stores/*.omni），见 GenerateOmniSourceOptions。 */
  knownStores?: string[];
  /** 组注释数据（键=fn 名）：保存时从 store.codeFlowGroups 传入。 */
  codeFlowGroups?: Record<string, CodeFlowGroup[]>;
}

/**
 * Options for generating a single-component OmniLang fragment.
 */
export interface GenerateOmniFragmentOptions {
  componentName: string;
  designElements: ElementData[];
  designWires: WireData[];
  codeTabs: CodeTab[];
  rootOrder?: string[];
  existingPidMap?: PidMap;
  /** Design system name for class reference prefixing and import emission. */
  designName?: string;
  /** When true, elements already belong to this component (from ComponentDef). Skip filtering. */
  fromDefinition?: boolean;
}

// ── Public function signatures (re-exported from pipeline.ts) ──

/**
 * Generate complete OmniLang source for a component.
 * Runs the full 3-phase pipeline and returns the emitted source string.
 */
export type GenerateOmniSourceFn = (options: GenerateOmniSourceOptions) => StageResult<string>;

/**
 * Generate multi-file OmniLang output.
 * Maps the pipeline result to src/main.omni, src/stores/<Name>.omni, etc.
 */
export type GenerateOmniFilesFn = (options: GenerateOmniFilesOptions) => StageResult<Record<string, string>>;

/**
 * Generate .omni source fragment for a single component.
 * Filters elements by componentRef then runs the full pipeline.
 */
export type GenerateOmniFragmentFn = (options: GenerateOmniFragmentOptions) => StageResult<string>;
