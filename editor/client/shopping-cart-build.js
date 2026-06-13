// ═══════════════════════════════════════════════════════════════
//  Shopping Cart — OmniDriver Build Script
//  Follows: O.help() first, Recipe Book, Semantic Naming,
//           Progressive Build-Test Loop, createEventHandler(),
//           O.validate() at the end
// ═══════════════════════════════════════════════════════════════

const O = window.__OMNI__;

// ─── Step 0: Discover API + Check Snippets ───
O.help();
O.switchMode("design");
O.clearAll();

// Check if there are any relevant snippets before building from scratch
const snippets = O.listSnippets();
console.log("Available snippets:", snippets ? snippets.length : "none");
console.log("→ Building Shopping Cart from scratch using Recipe Book patterns");

// ═══════════════════════════════════════════════════════════════
//  PHASE 1: Root Container + Header + Cart Counter Display
//  Elements: root Card, Heading, cart header row (2 Text nodes)
// ═══════════════════════════════════════════════════════════════

const root = O.card({ padding: "24px" });

// Page heading
O.heading("Shopping Cart", "1", root.id);

// Cart counter row — shows total items
const cartHeaderRow = O.flex("row", "8px", { alignItems: "center", marginBottom: "16px" }, root.id);
O.text("Cart:", { style: { fontWeight: "bold", fontSize: "16px" } }, cartHeaderRow.id);
const cartCountDisplay = O.text("0 items", { style: { fontSize: "16px" } }, cartHeaderRow.id);

// -- Validate Phase 1 --
let validation = O.validate();
console.log("Phase 1 [Root + Header]:", validation.isValid ? "PASS" : "FAIL");
if (!validation.isValid) { console.error("Phase 1 issues:", validation); }

// ═══════════════════════════════════════════════════════════════
//  PHASE 2: Search Filter
//  Elements: heading, Input, signal (searchQuery), handler,
//            computed (filteredProducts)
//  Pattern: Recipe Book — Search Filter
// ═══════════════════════════════════════════════════════════════

O.heading("Products", "2", root.id);

// Search input
const searchInput = O.input("Search products...", { type: "search" }, root.id);

// Signal: stores the search query string
const searchQuery = O.codeBlock("signal", "searchQuery", {
  typeAnnotation: "String",
  source: 'signal searchQuery: String = ""',
});

// Event handler: input onChange → updates searchQuery signal
const { handler: searchHandler } = searchInput.createEventHandler("onChange", {
  handlerName: "handleSearch",
  source: `action handleSearch(event: ChangeEvent) {
  searchQuery.set(event.value)
}`,
});
searchHandler.connectTo(searchQuery, "result", "set");

// Computed: derives filtered product list from search query
const filteredProducts = O.codeBlock("computed", "filteredProducts", {
  typeAnnotation: "String",
  source: `computed filteredProducts() -> String {
  searchQuery.value
}`,
});
searchQuery.connectTo(filteredProducts, "value", "deps");

// -- Validate Phase 2 --
validation = O.validate();
console.log("Phase 2 [Search Filter]:", validation.isValid ? "PASS" : "FAIL");
if (!validation.isValid) { console.error("Phase 2 issues:", validation); }

// ═══════════════════════════════════════════════════════════════
//  PHASE 3: Product Grid — 3 Product Cards
//  Elements: Flex row (grid), 3 Cards, 2 Text nodes per card
// ═══════════════════════════════════════════════════════════════

const productGrid = O.flex("row", "16px", {}, root.id);

// --- Product 1: Laptop $999 ---
const laptopCard = O.card({ flex: "1", padding: "16px" }, productGrid.id);
O.text("Laptop", { style: { fontWeight: "bold", fontSize: "18px" } }, laptopCard.id);
O.text("$999", { style: { color: "#666", fontSize: "16px", marginTop: "8px" } }, laptopCard.id);

// --- Product 2: Mouse $29 ---
const mouseCard = O.card({ flex: "1", padding: "16px" }, productGrid.id);
O.text("Mouse", { style: { fontWeight: "bold", fontSize: "18px" } }, mouseCard.id);
O.text("$29", { style: { color: "#666", fontSize: "16px", marginTop: "8px" } }, mouseCard.id);

// --- Product 3: Keyboard $79 ---
const keyboardCard = O.card({ flex: "1", padding: "16px" }, productGrid.id);
O.text("Keyboard", { style: { fontWeight: "bold", fontSize: "18px" } }, keyboardCard.id);
O.text("$79", { style: { color: "#666", fontSize: "16px", marginTop: "8px" } }, keyboardCard.id);

// -- Validate Phase 3 --
validation = O.validate();
console.log("Phase 3 [Product Grid]:", validation.isValid ? "PASS" : "FAIL");
if (!validation.isValid) { console.error("Phase 3 issues:", validation); }

// ═══════════════════════════════════════════════════════════════
//  PHASE 4: Cart Logic — Signals, Add to Cart Buttons, Checkout
//  Elements: 1 signal (cartCount), 3 "Add to Cart" buttons,
//            1 "Checkout" button, 1 computed (cartDisplayText)
//  Pattern: Recipe Book — Counter
// ═══════════════════════════════════════════════════════════════

// --- Shared cart count signal ---
const cartCount = O.codeBlock("signal", "cartCount", {
  typeAnnotation: "i32",
  source: "signal cartCount: i32 = 0",
});

// --- Product 1: Add to Cart button ---
const laptopAddBtn = O.button("Add to Cart", { variant: "primary" }, laptopCard.id);
const { handler: laptopHandler } = laptopAddBtn.createEventHandler("onPress", {
  handlerName: "addLaptop",
  source: `action addLaptop(event) {
  cartCount.set(cartCount.value + 1)
}`,
});
laptopHandler.connectTo(cartCount, "result", "set");

// --- Product 2: Add to Cart button ---
const mouseAddBtn = O.button("Add to Cart", { variant: "primary" }, mouseCard.id);
const { handler: mouseHandler } = mouseAddBtn.createEventHandler("onPress", {
  handlerName: "addMouse",
  source: `action addMouse(event) {
  cartCount.set(cartCount.value + 1)
}`,
});
mouseHandler.connectTo(cartCount, "result", "set");

// --- Product 3: Add to Cart button ---
const keyboardAddBtn = O.button("Add to Cart", { variant: "primary" }, keyboardCard.id);
const { handler: keyboardHandler } = keyboardAddBtn.createEventHandler("onPress", {
  handlerName: "addKeyboard",
  source: `action addKeyboard(event) {
  cartCount.set(cartCount.value + 1)
}`,
});
keyboardHandler.connectTo(cartCount, "result", "set");

// --- Checkout button: resets cart to 0 ---
const checkoutRow = O.flex("row", "16px", { alignItems: "center", marginTop: "24px", paddingTop: "16px" }, root.id);
const checkoutBtn = O.button("Checkout", { variant: "primary" }, checkoutRow.id);
const { handler: checkoutHandler } = checkoutBtn.createEventHandler("onPress", {
  handlerName: "handleCheckout",
  source: `action handleCheckout(event) {
  cartCount.set(0)
}`,
});
checkoutHandler.connectTo(cartCount, "result", "set");

// --- Computed: formats cart count for display ---
const cartDisplayText = O.codeBlock("computed", "cartDisplayText", {
  typeAnnotation: "String",
  source: `computed cartDisplayText() -> String {
  cartCount.value
}`,
});
cartCount.connectTo(cartDisplayText, "value", "deps");

// -- Validate Phase 4 --
validation = O.validate();
console.log("Phase 4 [Cart Logic]:", validation.isValid ? "PASS" : "FAIL");
if (!validation.isValid) { console.error("Phase 4 issues:", validation); }

// ═══════════════════════════════════════════════════════════════
//  PHASE 5: Final Validation & Summary
// ═══════════════════════════════════════════════════════════════

const finalReport = O.validate();

console.log("");
console.log("═══════════════════════════════════════════════════");
console.log("  SHOPPING CART — BUILD COMPLETE");
console.log("═══════════════════════════════════════════════════");
console.log("  Valid:         ", finalReport.isValid ? "✅ YES" : "❌ NO");
console.log("  View elements: ", O.elements.length);
console.log("  Wires:         ", O.wires.length);
console.log("  Is healthy:    ", finalReport.isValid ? "✅ Canvas is healthy!" : "❌ Issues found");

if (!finalReport.isValid) {
  console.error("");
  console.error("  Issues found:");
  if (finalReport.orphanElements?.length) {
    console.error("   - Orphan elements:", finalReport.orphanElements.length);
  }
  if (finalReport.unwiredEvents?.length) {
    console.error("   - Unwired events:", finalReport.unwiredEvents.length);
  }
  if (finalReport.typeMismatches?.length) {
    console.error("   - Type mismatches:", finalReport.typeMismatches.length);
  }
  if (finalReport.unreachableBlocks?.length) {
    console.error("   - Unreachable code blocks:", finalReport.unreachableBlocks.length);
  }
}

console.log("───────────────────────────────────────────────────");
console.log("  Element breakdown by tag:");
const tagCounts = {};
for (const el of O.elements) {
  tagCounts[el.tag] = (tagCounts[el.tag] || 0) + 1;
}
for (const [tag, count] of Object.entries(tagCounts)) {
  console.log("   ", tag + ":", count);
}
console.log("═══════════════════════════════════════════════════");
