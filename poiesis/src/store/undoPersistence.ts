/** IndexedDB 撤销/画布持久化。
 *
 *  简化策略：每次 execute/undo/redo 后保存 elements + wires 快照。
 *  页面加载 / visibilitychange 时恢复。不序列化 Command 栈本身。 */

import { openDB, type IDBPDatabase } from "idb";
import type { ElementData, WireData } from "./editor";

const DB_NAME = "OmniLangEditor";
const STORE_NAME = "editorSnapshot";
const DB_VERSION = 1;
const SNAPSHOT_KEY = "latest";

let dbPromise: Promise<IDBPDatabase<unknown>> | null = null;

function getDb(): Promise<IDBPDatabase<unknown>> {
  if (!dbPromise) {
    dbPromise = openDB(DB_NAME, DB_VERSION, {
      upgrade(db) {
        if (!db.objectStoreNames.contains(STORE_NAME)) {
          db.createObjectStore(STORE_NAME);
        }
      },
    });
  }
  return dbPromise;
}

export interface EditorSnapshot {
  elements: ElementData[];
  wires: WireData[];
  timestamp: number;
}

export async function saveSnapshot(
  elements: ElementData[],
  wires: WireData[],
): Promise<void> {
  try {
    const db = await getDb();
    const snapshot: EditorSnapshot = {
      elements,
      wires,
      timestamp: Date.now(),
    };
    await db.put(STORE_NAME, snapshot, SNAPSHOT_KEY);
  } catch {
    // 尽力而为 —— 保存失败不中断用户操作
  }
}

export async function loadSnapshot(): Promise<EditorSnapshot | null> {
  try {
    const db = await getDb();
    const snapshot = await db.get(STORE_NAME, SNAPSHOT_KEY);
    return (snapshot as EditorSnapshot) ?? null;
  } catch {
    return null;
  }
}
