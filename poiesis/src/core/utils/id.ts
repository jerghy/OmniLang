/** Unique ID generator — format: {prefix}-{timestamp}-{random}-{sequence} */

let _counter = 0;

export function uid(prefix = "el"): string {
  const ts = Date.now().toString(36);
  const rand = Math.random().toString(36).slice(2, 8);
  const seq = (_counter++).toString(36);
  return `${prefix}-${ts}-${rand}-${seq}`;
}
