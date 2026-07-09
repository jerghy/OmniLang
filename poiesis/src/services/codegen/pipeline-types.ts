// poiesis/src/services/codegen/pipeline-types.ts
// Public type interfaces for the codegen pipeline.
// Extracted from pipeline.ts for type-safe usage without @ts-nocheck.

import type { ElementData, WireData, CodeTab } from "../../core/types";
import type { ComponentMeta, StageResult } from "./types";
import type { PidMap } from "../../core/types/pid";

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
