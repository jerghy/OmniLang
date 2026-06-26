/** Extract the component name from Omni DSL source. */

export function extractComponentName(source: string, fallback = "App"): string {
  const match = source.match(/component\s+(\w+)\s*\{/);
  return match ? match[1] : fallback;
}
