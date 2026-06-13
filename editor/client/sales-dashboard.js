const O = window.__OMNI__;

// 1. Discover the API (mandatory first step)
O.help();

// 2. Switch to design mode and clear
O.switchMode("design");
O.clearAll();

// 3. Root card wrapping everything
const root = O.card({ padding: "24px" });

// 4. Page title
O.heading("Sales Dashboard", "1", root.id);

// 5. Signal: lastUpdated (type String)
const lastUpdated = O.codeBlock("signal", "lastUpdated", {
  typeAnnotation: "String",
  source: 'signal lastUpdated: String = ""',
});

// 6. Flex row for the three metric cards
const row = O.flex("row", "16px", {}, root.id);

// 7. Metric card 1: Revenue
const revCard = O.card({ padding: "16px" }, row.id);
O.text("Revenue", {}, revCard.id);
O.heading("$12,345", "3", revCard.id);

// 8. Metric card 2: Orders
const ordCard = O.card({ padding: "16px" }, row.id);
O.text("Orders", {}, ordCard.id);
O.heading("156", "3", ordCard.id);

// 9. Metric card 3: Customers
const custCard = O.card({ padding: "16px" }, row.id);
O.text("Customers", {}, custCard.id);
O.heading("89", "3", custCard.id);

// 10. Refresh Data button
const btn = O.button("Refresh Data", { variant: "primary" }, root.id);

// 11. Event handler on Refresh button
const { handler } = btn.createEventHandler("onPress", {
  handlerName: "refreshData",
  source: `action refreshData(event) {
  lastUpdated.set("Just now")
}`,
});

// 12. Wire handler.result -> lastUpdated.set
handler.connectTo(lastUpdated, "result", "set");

// 13. Validate canvas
const report = O.validate();
if (!report.isValid) {
  console.error("Canvas issues:", report);
}
console.log("Elements:", O.elements.length, "Wires:", O.wires.length);
