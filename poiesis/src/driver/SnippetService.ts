/** SnippetService — snippet template registry and built-in templates.
 *  Receives an OmniDriver-like facade for element creation.
 *  Built-in templates: "login-form", "data-card", "crud-table". */

import { ElementHandle } from "./ElementHandle";
import type { OmniDriver } from "./OmniDriver";

// ═══════════════════════════════════════════════════════
// SnippetService
// ═══════════════════════════════════════════════════════

export class SnippetService {
  /** Registry for user-registered snippet builders. */
  private _snippetRegistry: Map<string, (options?: Record<string, unknown>) => ElementHandle> = new Map();

  constructor(
    /** The OmniDriver facade used for element creation. */
    private _driver: OmniDriver,
  ) {}

  /** Register a custom snippet template.
   *  The builder function receives an optional options object and must return the root ElementHandle.
   *
   *  @example
   *  O.registerSnippet("product-card", (opts) => {
   *    const card = O.component("div", {}, undefined);
   *    O.image(opts.image, "Product", card.id);
   *    O.heading(opts.title, "3", card.id);
   *    O.text(`$${opts.price}`, {}, card.id);
   *    const btn = O.button("Buy", {}, card.id);
   *    btn.bindEvent("onPress", "handlePurchase");
   *    return card;
   *  });
   *  O.snippet("product-card", { image: "url", title: "Widget", price: "9.99" });
   */
  registerSnippet(
    name: string,
    builder: (options?: Record<string, unknown>) => ElementHandle,
  ): void {
    this._snippetRegistry.set(name, builder);
  }

  /** Remove a registered snippet template. */
  unregisterSnippet(name: string): boolean {
    return this._snippetRegistry.delete(name);
  }

  /** List all registered snippet names (built-in + custom). */
  listSnippets(): { builtin: string[]; custom: string[] } {
    return {
      builtin: ["login-form", "data-card", "crud-table"],
      custom: Array.from(this._snippetRegistry.keys()),
    };
  }

  /** Create a pre-built UI pattern.
   *  Built-in templates: "login-form", "data-card", "crud-table".
   *  Use registerSnippet() to add custom templates. */
  snippet(
    template: string,
    options?: Record<string, unknown>,
  ): ElementHandle {
    // Check custom registry first (allows overriding built-ins)
    const customBuilder = this._snippetRegistry.get(template);
    if (customBuilder) {
      return customBuilder(options);
    }

    switch (template) {
      case "login-form": {
        const title = (options?.title as string) || "Sign In";
        const card = this._driver.component("div", {}, undefined);
        this._driver.heading(title, "2", card.id);
        this._driver.input("Email", { type: "email" }, card.id);
        this._driver.input("Password", { type: "password" }, card.id);
        const submitBtn = this._driver.button("Sign In", {}, card.id);
        submitBtn.bindEvent("onPress", "handleSignIn");
        return card;
      }
      case "data-card": {
        const title = (options?.title as string) || "Card Title";
        const desc = (options?.description as string) || "Card description here.";
        const card = this._driver.component("div", {}, undefined);
        this._driver.heading(title, "3", card.id);
        this._driver.text(desc, {}, card.id);
        if (options?.actionLabel) {
          const actionBtn = this._driver.button(options.actionLabel as string, {}, card.id);
          actionBtn.bindEvent("onPress", "handleAction");
        }
        return card;
      }
      case "crud-table": {
        const title = (options?.title as string) || "Data";
        const card = this._driver.component("div", {}, undefined);
        this._driver.heading(title, "2", card.id);
        const toolbar = this._driver.flex(card.id);
        this._driver.input("Search...", {}, toolbar.id);
        const addBtn = this._driver.button("Add", {}, toolbar.id);
        addBtn.bindEvent("onPress", "handleAdd");
        const columns = (options?.columns as string[]) || ["Name", "Status", "Actions"];
        const headerRow = this._driver.flex(card.id);
        for (const col of columns) {
          this._driver.heading(col, "4", headerRow.id);
        }
        return card;
      }
      default:
        throw new Error(
          `Unknown snippet "${template}". Built-in: "login-form", "data-card", "crud-table". ` +
          `Custom: ${Array.from(this._snippetRegistry.keys()).join(", ") || "(none)"}. ` +
          `Use O.registerSnippet(name, builder) to add custom templates.`,
        );
    }
  }
}
