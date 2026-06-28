/** TypeScript mirrors of editor/server/annotation/types.py
 *  Types have been migrated to core/types/ — re-exported from there. */

export type {
  ViewRenderMode,
  ViewMarker,
  NodeEntry,
  WireEntry,
  Chain,
  AnnotationFile,
  AnnotatedLoadResponse,
  AnnotatedSaveResponse,
  EditorTomlState,
} from "../../core/types";

export type { CodeTabType_Annotation as CodeTabType } from "../../core/types";
