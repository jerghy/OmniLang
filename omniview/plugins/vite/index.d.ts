/**
 * OmniView Vite Plugin type declarations.
 */
import type { Plugin } from "vite";

export interface OmniViewPluginOptions {
  /** Target framework: 'react' (default), 'vue', 'dioxus' */
  target?: "react" | "vue" | "dioxus";
  /** Path to omnic CLI executable (auto-detected if not specified) */
  omnicPath?: string;
}

/**
 * Create OmniView Vite plugins.
 *
 * @example
 * // vite.config.ts
 * import omniView from '@omnilang/vite-plugin-omniview';
 *
 * export default defineConfig({
 *   plugins: [omniView({ target: 'react' })],
 * });
 */
declare function omniView(options?: OmniViewPluginOptions): Plugin[];

export = omniView;
