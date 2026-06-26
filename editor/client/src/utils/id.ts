/** 可读 ID 生成器 — 格式: {prefix}_{6位短随机}
 *  例: div_a1b2c3, button_ff3a22, input_4d5e6f
 *  不产生 UUID 乱码。*/

function shortId(): string {
  return Math.random().toString(16).slice(2, 8);
}

export function uid(prefix = "el"): string {
  return `${prefix}_${shortId()}`;
}
