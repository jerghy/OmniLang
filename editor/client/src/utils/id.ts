/** UUID 生成器。*/

export function uid(): string {
  return crypto.randomUUID();
}
