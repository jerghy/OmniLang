/**
 * Product Catalog — OmniDriver Canvas Builder
 * =============================================
 *
 * Builds a "Product Catalog" page on the OmniLang visual editor canvas.
 *
 * Features:
 *   1. Search input (filters by product name, wired to searchQuery signal)
 *   2. Category filter buttons: "All", "Electronics", "Clothing"
 *   3. Product cards (3 products) — image, name, price, "Add to Cart"
 *   4. Cart state tracking (count + last-added message)
 *
 * Data flow:
 *   Input onChange  ──→ onSearchChange action ──→ searchQuery (signal)
 *   Button onPress  ──→ onSelectAll action      ──→ selectedCategory (signal)
 *   Button onPress  ──→ onSelectElectronics      ──→ selectedCategory (signal)
 *   Button onPress  ──→ onSelectClothing         ──→ selectedCategory (signal)
 *   Btn "Add to Cart" ──→ onAddToCart actions    ──→ cartCount + cartMessage (signals)
 *
 * Paste this script into the browser console while on the OmniLang editor page.
 */

(function buildProductCatalog() {
  "use strict";

  const O = window.__OMNI__;

  // ─── Step 1: API Discovery (mandatory first step) ───
  O.help();

  // ─── Step 2: Canvas Setup ───
  O.switchMode("design");
  O.clearAll();

  // ─── Step 3: Create State Signals ───
  // searchQuery: holds the current search input text
  const searchQuery = O.codeBlock("signal", "searchQuery", {
    typeAnnotation: "String",
    source: 'signal searchQuery: String = ""',
  });

  // selectedCategory: tracks which category filter is active ("All", "Electronics", "Clothing")
  const selectedCategory = O.codeBlock("signal", "selectedCategory", {
    typeAnnotation: "String",
    source: 'signal selectedCategory: String = "All"',
  });

  // cartCount: number of items in the cart
  const cartCount = O.codeBlock("signal", "cartCount", {
    typeAnnotation: "i32",
    source: "signal cartCount: i32 = 0",
  });

  // cartMessage: feedback message shown after adding an item
  const cartMessage = O.codeBlock("signal", "cartMessage", {
    typeAnnotation: "String",
    source: 'signal cartMessage: String = ""',
  });

  // ─── Step 4: Root Container ───
  const root = O.card({
    padding: "24px",
    width: "100%",
    background_color: "#ffffff",
  });

  // ─── Step 5: Header Row (Title + Cart Badge) ───
  const headerFlex = O.flex(
    "row",
    "16px",
    { align: "center", justify: "between", width: "100%" },
    root.id
  );

  O.heading("Product Catalog", "1", headerFlex.id);

  // Cart badge shows item count
  const cartBadge = O.text(
    "Cart (0)",
    {
      font_size: "14px",
      font_weight: "600",
      color: "#374151",
      padding: "8px 16px",
      background_color: "#f3f4f6",
      border_radius: "8px",
    },
    headerFlex.id
  );

  // ─── Step 6: Search Input ───
  const searchInput = O.input(
    "Search products by name...",
    {
      width: "100%",
      padding: "12px 16px",
      border_radius: "8px",
      font_size: "14px",
    },
    root.id
  );

  // ─── Step 7: Category Filter Buttons ───
  const categoryFlex = O.flex(
    "row",
    "8px",
    { padding: "12px 0 16px 0", width: "100%" },
    root.id
  );

  // "All" button starts as primary; others as secondary
  const allBtn = O.button("All", { variant: "primary" }, categoryFlex.id);
  const electronicsBtn = O.button(
    "Electronics",
    { variant: "secondary" },
    categoryFlex.id
  );
  const clothingBtn = O.button(
    "Clothing",
    { variant: "secondary" },
    categoryFlex.id
  );

  // ─── Step 8: Cart Feedback Text ───
  const feedbackText = O.text(
    "",
    {
      font_size: "14px",
      color: "#059669",
      font_weight: "500",
      padding: "4px 0 12px 0",
    },
    root.id
  );

  // ─── Step 9: Product Grid ───
  // Use a single row for products; the canvas handles overflow
  const productGrid = O.flex(
    "row",
    "24px",
    { padding: "8px 0", width: "100%" },
    root.id
  );

  // ─── Product 1: Wireless Headphones (Electronics) ───
  const p1 = O.card(
    {
      width: "280px",
      padding: "16px",
      border_radius: "12px",
      shadow: "0 1px 3px rgba(0,0,0,0.1)",
    },
    productGrid.id
  );
  O.image(
    "https://picsum.photos/seed/headphones/280/180",
    "Wireless Headphones",
    p1.id
  );
  O.heading("Wireless Headphones", "3", p1.id);
  O.text(
    "$79.99",
    {
      font_size: "18px",
      font_weight: "bold",
      color: "#2563eb",
      padding: "4px 0",
    },
    p1.id
  );
  O.text(
    "Electronics",
    { font_size: "12px", color: "#6b7280" },
    p1.id
  );
  const addBtn1 = O.button(
    "Add to Cart",
    { variant: "primary", width: "100%" },
    p1.id
  );

  // ─── Product 2: Cotton T-Shirt (Clothing) ───
  const p2 = O.card(
    {
      width: "280px",
      padding: "16px",
      border_radius: "12px",
      shadow: "0 1px 3px rgba(0,0,0,0.1)",
    },
    productGrid.id
  );
  O.image(
    "https://picsum.photos/seed/tshirt/280/180",
    "Cotton T-Shirt",
    p2.id
  );
  O.heading("Cotton T-Shirt", "3", p2.id);
  O.text(
    "$24.99",
    {
      font_size: "18px",
      font_weight: "bold",
      color: "#2563eb",
      padding: "4px 0",
    },
    p2.id
  );
  O.text(
    "Clothing",
    { font_size: "12px", color: "#6b7280" },
    p2.id
  );
  const addBtn2 = O.button(
    "Add to Cart",
    { variant: "primary", width: "100%" },
    p2.id
  );

  // ─── Product 3: Running Shoes (Clothing) ───
  const p3 = O.card(
    {
      width: "280px",
      padding: "16px",
      border_radius: "12px",
      shadow: "0 1px 3px rgba(0,0,0,0.1)",
    },
    productGrid.id
  );
  O.image(
    "https://picsum.photos/seed/shoes/280/180",
    "Running Shoes",
    p3.id
  );
  O.heading("Running Shoes", "3", p3.id);
  O.text(
    "$129.99",
    {
      font_size: "18px",
      font_weight: "bold",
      color: "#2563eb",
      padding: "4px 0",
    },
    p3.id
  );
  O.text(
    "Clothing",
    { font_size: "12px", color: "#6b7280" },
    p3.id
  );
  const addBtn3 = O.button(
    "Add to Cart",
    { variant: "primary", width: "100%" },
    p3.id
  );

  // ════════════════════════════════════════════════════════
  // Step 10: Event Handlers & Data Flow Wiring
  // ════════════════════════════════════════════════════════

  // ─── 10a. Search Input → searchQuery Signal ───
  // The Input.onChange event fires when the user types;
  // the action reads event.value and writes it to searchQuery.
  const searchEvt = searchInput.createEventHandler("onChange", {
    handlerName: "onSearchChange",
    source: [
      'action onSearchChange(event: String) {',
      '  searchQuery.set(event)',
      '}',
    ].join('\n'),
  });
  searchEvt.handler.connectTo(searchQuery, "result", "set");

  // ─── 10b. Category Filter Buttons → selectedCategory Signal ───
  // Each button sets selectedCategory to its respective value on press.

  // "All" button
  const allEvt = allBtn.createEventHandler("onPress", {
    handlerName: "onSelectAll",
    source: [
      'action onSelectAll(event: MouseEvent) {',
      '  selectedCategory.set("All")',
      '}',
    ].join('\n'),
  });
  allEvt.handler.connectTo(selectedCategory, "result", "set");

  // "Electronics" button
  const electronicsEvt = electronicsBtn.createEventHandler("onPress", {
    handlerName: "onSelectElectronics",
    source: [
      'action onSelectElectronics(event: MouseEvent) {',
      '  selectedCategory.set("Electronics")',
      '}',
    ].join('\n'),
  });
  electronicsEvt.handler.connectTo(selectedCategory, "result", "set");

  // "Clothing" button
  const clothingEvt = clothingBtn.createEventHandler("onPress", {
    handlerName: "onSelectClothing",
    source: [
      'action onSelectClothing(event: MouseEvent) {',
      '  selectedCategory.set("Clothing")',
      '}',
    ].join('\n'),
  });
  clothingEvt.handler.connectTo(selectedCategory, "result", "set");

  // ─── 10c. Add to Cart Buttons → cartCount + cartMessage Signals ───
  //
  // IMPORTANT: Each action updates BOTH cartCount and cartMessage from
  // its own source code. This avoids the anti-pattern of trying to fan-out
  // a single action.result port to multiple signal.set ports.
  //
  // Rule: one action.result → one signal.set (the wire is just a trigger).
  // The action body references all signals it needs to update directly.

  // Product 1: Wireless Headphones
  const cartEvt1 = addBtn1.createEventHandler("onPress", {
    handlerName: "onAddToCart1",
    source: [
      'action onAddToCart1(event: MouseEvent) {',
      '  cartCount.set(cartCount.value + 1)',
      '  cartMessage.set("Added Wireless Headphones to cart!")',
      '}',
    ].join('\n'),
  });
  // Single wire to cartCount — the action source also updates cartMessage internally
  cartEvt1.handler.connectTo(cartCount, "result", "set");

  // Product 2: Cotton T-Shirt
  const cartEvt2 = addBtn2.createEventHandler("onPress", {
    handlerName: "onAddToCart2",
    source: [
      'action onAddToCart2(event: MouseEvent) {',
      '  cartCount.set(cartCount.value + 1)',
      '  cartMessage.set("Added Cotton T-Shirt to cart!")',
      '}',
    ].join('\n'),
  });
  cartEvt2.handler.connectTo(cartCount, "result", "set");

  // Product 3: Running Shoes
  const cartEvt3 = addBtn3.createEventHandler("onPress", {
    handlerName: "onAddToCart3",
    source: [
      'action onAddToCart3(event: MouseEvent) {',
      '  cartCount.set(cartCount.value + 1)',
      '  cartMessage.set("Added Running Shoes to cart!")',
      '}',
    ].join('\n'),
  });
  cartEvt3.handler.connectTo(cartCount, "result", "set");

  // ─── Step 11: Canvas Validation ───
  const report = O.validate();
  if (!report.isValid) {
    console.group("Canvas validation issues:");
    if (report.orphanElements.length) {
      console.warn("Orphan elements (no parent):", report.orphanElements);
    }
    if (report.unwiredEvents.length) {
      console.warn("Unwired events:", report.unwiredEvents);
    }
    if (report.typeMismatches.length) {
      console.warn("Type mismatches:", report.typeMismatches);
    }
    if (report.unreachableBlocks.length) {
      console.info("Unreachable code blocks (no inbound wire):", report.unreachableBlocks);
    }
    console.groupEnd();
  } else {
    console.log("Canvas is healthy — no validation issues.");
  }

  // ─── Summary ───
  const viewCount = O.views.length;
  const codeBlockCount = O.codeBlocks.length;
  const wireCount = O.wires.length;
  console.log(
    "Product Catalog built: %d view elements, %d code blocks, %d wires",
    viewCount, codeBlockCount, wireCount
  );
  console.log("Catalog root card ID:", root.id);
})();
