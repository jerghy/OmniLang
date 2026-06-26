// core/events/bus.ts
import mitt from 'mitt';

export type EditorEvents = {
  'element:added': { elementId: string };
  'element:removed': { elementId: string };
  'element:selected': { elementIds: string[] };
  'wire:added': { wireId: string };
  'wire:removed': { wireId: string };
  'project:loaded': { path: string };
  'project:saved': { path: string };
  'compile:completed': { errors: Array<{ code: string; message: string; line: number }> };
  'mode:changed': { mode: 'design' | 'code' };
  'tab:switched': { docId: string | null };
};

export const editorBus = mitt<EditorEvents>();
