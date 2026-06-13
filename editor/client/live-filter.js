// ═══════════════════════════════════════════════════════════════════════
// Live Filter — OmniLang Signal→Computed Pipeline
//
// Data flow:
//   Input.onChange → Handler(action) → Signal.set(searchQuery)
//                                          ↓
//                              ┌ ─ ─ ─ ─┴─ ─ ─ ─ ┐
//                              ↓                   ↓
//                       filterCount(i32)    filterResult(String)
//                              ↓                   ↓
//                              └ ─ ─ → ─ ─ ─ ─ ─ ┘
//                                          ↓
//                                   Text.label (display)
//
// Type annotations:
//   searchQuery  : String  (signal)
//   filterCount  : i32     (computed)
//   filterResult : String  (computed)
//
// Usage: paste into browser console or evaluate via page.evaluate()
// ═══════════════════════════════════════════════════════════════════════
(async function buildLiveFilter() {
  'use strict';

  const O = window.__OMNI__;

  // ── Step 1: API discovery (required — OmniDriver is a Proxy) ──
  O.help();

  // ── Step 2: Fresh canvas in design mode ──
  O.switchMode('design');
  O.clearAll();

  // ═══════════════════════════════════════════════════════════════
  // UI LAYER — Card with Input + Display text
  // ═══════════════════════════════════════════════════════════════

  // Root card (no parentId since it sits at canvas root)
  const root = O.card({
    style: {
      width: '480px',
      borderRadius: '12px',
      boxShadow: '0 4px 20px rgba(0,0,0,0.08)',
      border: '1px solid #e8e8e8',
    },
  });

  // Vertical flex container inside the card
  const col = O.flex(
    'column',
    '12px',
    { style: { padding: '24px' } },
    root.id
  );

  // Header
  O.heading('Live Filter', '3', col.id);

  // Search input — fires onChange when the user types
  const input = O.input(
    'Search fruit...',
    {
      style: {
        width: '100%',
        padding: '10px 14px',
        border: '2px solid #e0e0e0',
        borderRadius: '8px',
        fontSize: '14px',
        boxSizing: 'border-box',
        outline: 'none',
      },
    },
    col.id
  );

  // Dynamic display — shows count + filtered items
  // Its label is updated reactively via the computed.value → label wire
  const output = O.text(
    [
      'Showing 5 of 5 items',
      '',
      '• Apple',
      '• Banana',
      '• Cherry',
      '• Date',
      '• Elderberry',
    ].join('\n'),
    {
      style: {
        fontSize: '14px',
        whiteSpace: 'pre-wrap',
        lineHeight: '1.6',
        color: '#333',
        backgroundColor: '#fafafa',
        padding: '14px 16px',
        borderRadius: '8px',
        border: '1px solid #eee',
        minHeight: '160px',
      },
    },
    col.id
  );

  // ═══════════════════════════════════════════════════════════════
  // DATA PIPELINE — Signal → Computed
  // ═══════════════════════════════════════════════════════════════

  // Signal: holds current search query (String, default "")
  const sig = O.codeBlock('signal', 'searchQuery', {
    typeAnnotation: 'String',
    source: ['signal searchQuery: String = ""'].join('\n'),
  });

  // Computed 1 — filterCount (i32): returns the count of matching items.
  // Demonstrates the i32 type annotation as required.
  const countComp = O.codeBlock('computed', 'filterCount', {
    typeAnnotation: 'i32',
    source: [
      'computed filterCount() -> i32 {',
      '  let items = ["Apple", "Banana", "Cherry", "Date", "Elderberry"]',
      '  let query = searchQuery.value',
      '  let count = 0',
      '  for (let i = 0; i < items.length; i++) {',
      '    if (items[i].toLowerCase().contains(query.toLowerCase())) {',
      '      count += 1',
      '    }',
      '  }',
      '  return count',
      '}',
    ].join('\n'),
  });

  // Computed 2 — filterResult (String): builds the display string.
  // References both searchQuery (signal) and filterCount (computed) by name.
  const comp = O.codeBlock('computed', 'filterResult', {
    typeAnnotation: 'String',
    source: [
      'computed filterResult() -> String {',
      '  let items = ["Apple", "Banana", "Cherry", "Date", "Elderberry"]',
      '  let query = searchQuery.value',
      '  let count = filterCount.value',
      '  let result = "Showing " + count + " of " + items.length + " items"',
      '  if query != "" {',
      '    result += " (filter: \\"" + query + "\\")"',
      '  }',
      '  result += "\\n\\n"',
      '  for (let i = 0; i < items.length; i++) {',
      '    if (items[i].toLowerCase().contains(query.toLowerCase())) {',
      '      result += "• " + items[i] + "\\n"',
      '    }',
      '  }',
      '  return result',
      '}',
    ].join('\n'),
  });

  // ═══════════════════════════════════════════════════════════════
  // EVENT HANDLER — Input.onChange → Action → Signal.set
  // ═══════════════════════════════════════════════════════════════

  // createEventHandler auto-wires: Input.onChange → Handler.call
  // The action source calls searchQuery.set() with the event value,
  // which updates the signal and triggers the computed cascade.
  const { handler } = input.createEventHandler('onChange', {
    handlerName: 'updateSearch',
    source: [
      'action updateSearch(event) {',
      '  searchQuery.set(event.value)',
      '}',
    ].join('\n'),
  });

  // ═══════════════════════════════════════════════════════════════
  // WIRING — Data flow connections
  // ═══════════════════════════════════════════════════════════════

  // 1) Handler.result → Signal.set
  //    When the input changes, the action writes the new value into the signal.
  handler.connectTo(sig, 'result', 'set');

  // 2) Signal.value → filterCount.deps (count computed depends on signal)
  sig.connectTo(countComp, 'value', 'deps');

  // 3) Signal.value → filterResult.deps (display computed depends on signal;
  //    filterCount is discovered as transitive dependency through source analysis)
  sig.connectTo(comp, 'value', 'deps');

  // 4) Computed.value → Text.label
  //    Pushes the formatted display string into the text element,
  //    updating the count and item list reactively.
  comp.connectTo(output, 'value', 'label');

  // ═══════════════════════════════════════════════════════════════
  // VALIDATION — Check canvas health
  // ═══════════════════════════════════════════════════════════════

  const report = O.validate();
  console.log('');
  console.log('═══════════════════════════════════════════');
  console.log('  LIVE FILTER — BUILD REPORT');
  console.log('═══════════════════════════════════════════');
  console.log('  isValid:', report.isValid);
  if (report.orphanElements && report.orphanElements.length) {
    console.log('  orphanElements:', report.orphanElements);
  }
  if (report.unwiredEvents && report.unwiredEvents.length) {
    console.log('  unwiredEvents:', JSON.stringify(report.unwiredEvents, null, 2));
  }
  if (report.typeMismatches && report.typeMismatches.length) {
    console.log('  typeMismatches:', JSON.stringify(report.typeMismatches, null, 2));
  }
  if (report.unreachableBlocks && report.unreachableBlocks.length) {
    console.log('  unreachableBlocks:', report.unreachableBlocks);
  }
  console.log('  Elements:', O.elements.length);
  console.log('  Wires:', O.wires.length);
  console.log('═══════════════════════════════════════════');
  console.log('');

  // ── Summary ──
  console.log('Live Filter data flow:');
  console.log('  Input.onChange → updateSearch(action) → searchQuery(signal).set');
  console.log('  searchQuery.value → filterCount(computed:i32).deps');
  console.log('  searchQuery.value → filterResult(computed:String).deps');
  console.log('  filterResult.value → display Text.label');
  console.log('  Type annotations: searchQuery=String, filterCount=i32, filterResult=String');

  return {
    status: report.isValid ? 'healthy' : 'issues-found',
    elements: O.elements.length,
    wires: O.wires.length,
    validation: report,
  };
})();
