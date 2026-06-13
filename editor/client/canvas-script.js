(async function() {
  'use strict';

  const O = window.__OMNI__;
  O.switchMode('design');

  // ── 1. SCAN EXISTING CANVAS ──────────────────────────────────────
  const allElements = O.getElements();
  console.log('[OmniScript] Existing elements:', allElements.length);

  // Find the Card
  const cards = O.findByTag('Card');
  const card = cards[0];
  if (!card) throw new Error('[OmniScript] No Card found on canvas');

  // Find the Heading "User Profile"
  const headings = O.findByTag('Heading');
  const profileHeading = headings.find(h => h.text() === 'User Profile');
  if (!profileHeading) console.warn('[OmniScript] Heading "User Profile" not found');

  // Find the Text "Name: Alice"
  const texts = O.findByTag('Text');
  const nameText = texts.find(t => t.text() === 'Name: Alice');
  if (!nameText) console.warn('[OmniScript] Text "Name: Alice" not found');

  // Find the loadData action code block
  const allActions = O.find({ nodeType: 'codeblock', tag: 'action' });
  const loadData = allActions.find(a => a.getProp('name') === 'loadData');
  if (!loadData) throw new Error('[OmniScript] loadData action not found');
  console.log('[OmniScript] Found loadData action:', loadData.id);
  console.log('[OmniScript] Found Card:', card.id);

  // ── 2. ADD BUTTON "Refresh" INSIDE CARD (no duplicates) ─────────
  let refreshBtn = O.findByTag('Button').find(b => b.text() === 'Refresh');
  if (!refreshBtn) {
    refreshBtn = O.button('Refresh', { variant: 'primary' }, card.id);
    console.log('[OmniScript] Created Button "Refresh"');
  } else {
    console.log('[OmniScript] Button "Refresh" already exists');
  }

  // ── 3. CREATE EVENT HANDLER FOR BUTTON onPress ──────────────────
  //     createEventHandler auto-wires Button onPress → handler "call"
  const knownHandlers = O.find({ nodeType: 'codeblock', tag: 'action' });
  let refreshHandler = knownHandlers.find(h => h.getProp('name') === 'onRefresh');

  if (!refreshHandler) {
    const result = refreshBtn.createEventHandler('onPress', {
      handlerName: 'onRefresh',
      source: [
        'action onRefresh(event) {',
        '  // Called when the Refresh button is pressed',
        '  // Triggers loadData to refresh user data',
        '  // The result flows through: handler → loadData → status signal',
        '  return event;',
        '}',
      ].join('\n'),
      folded: false,
    });
    refreshHandler = result.handler;
    console.log('[OmniScript] Created event handler "onRefresh" + wire');
  } else {
    console.log('[OmniScript] Handler "onRefresh" already exists');
  }

  // ── 4. WIRE HANDLER "result" → loadData "call" ─────────────────
  const existingWires = O.getWires();
  const wireHandlerToLoadData = existingWires.find(
    w => w.fromId === refreshHandler.id && w.fromPort === 'result' && w.toId === loadData.id && w.toPort === 'call'
  );
  if (!wireHandlerToLoadData) {
    refreshHandler.connectTo(loadData, 'result', 'call');
    console.log('[OmniScript] Wired handler.result → loadData.call');
  } else {
    console.log('[OmniScript] Wire handler→loadData already exists');
  }

  // ── 5. ADD SIGNAL CODE BLOCK "status" (type: String) ────────────
  let statusSignal = O.find({ nodeType: 'codeblock', tag: 'signal' }).find(s => s.getProp('name') === 'status');
  if (!statusSignal) {
    statusSignal = O.codeBlock('signal', 'status', {
      typeAnnotation: 'String',
      source: [
        '// Status signal — shows current loading state',
        '// Initial state: "idle"',
        'signal status: String = "idle"',
      ].join('\n'),
      isFolded: true,
    });
    console.log('[OmniScript] Created signal "status" (type: String)');
  } else {
    console.log('[OmniScript] Signal "status" already exists');
  }

  // ── 6. WIRE loadData "result" → statusSignal "set" ──────────────
  //     This is the "wire it to show loading state" connection:
  //     when loadData completes, it sets the status signal value.
  const wireLoadDataToSignal = existingWires.find(
    w => w.fromId === loadData.id && w.fromPort === 'result' && w.toId === statusSignal.id && w.toPort === 'set'
  );
  if (!wireLoadDataToSignal) {
    loadData.connectTo(statusSignal, 'result', 'set');
    console.log('[OmniScript] Wired loadData.result → status.set (loading state)');
  } else {
    console.log('[OmniScript] Wire loadData→status already exists');
  }

  // ── 7. ADD A STATUS DISPLAY TEXT INSIDE THE CARD ────────────────
  let statusText = O.findByTag('Text').find(
    t => t.parent()?.id === card.id && t.text().startsWith('Status:')
  );
  if (!statusText) {
    statusText = O.text('Status: idle', { fontSize: '12px' }, card.id);
    console.log('[OmniScript] Created status display Text');
  } else {
    console.log('[OmniScript] Status Text already exists');
  }

  // Reorder children: keep Heading first, then original Text, then status, then button
  // (visual order in the Card — optional polish)

  // ── 8. UPDATE HANDLER SOURCE FOR THE FULL DATAFLOW ──────────────
  refreshHandler.setSource([
    'action onRefresh(event) {',
    '  // Refresh button pressed — trigger loadData and show loading state',
    '  // 1. Set status to "loading..."',
    '  // 2. Call loadData to fetch fresh data',
    '  // 3. loadData result flows to the status signal via wire',
    '  return "loading...";',
    '}',
  ].join('\n'));

  console.log('[OmniScript] Updated handler source code');

  // ── 9. VALIDATE CANVAS HEALTH ────────────────────────────────────
  const report = O.validate();
  console.log('');
  console.log('═══════════════════════════════════════════');
  console.log('  CANVAS VALIDATION REPORT');
  console.log('═══════════════════════════════════════════');
  console.log('  isValid:', report.isValid);
  if (report.orphanElements.length)      console.log('  orphanElements:', report.orphanElements);
  if (report.unwiredEvents.length)       console.log('  unwiredEvents:', JSON.stringify(report.unwiredEvents, null, 2));
  if (report.typeMismatches.length)      console.log('  typeMismatches:', JSON.stringify(report.typeMismatches, null, 2));
  if (report.unreachableBlocks.length)   console.log('  unreachableBlocks:', report.unreachableBlocks);

  if (report.isValid && report.unwiredEvents.length === 0 && report.unreachableBlocks.length === 0) {
    console.log('  STATUS: Canvas is healthy!');
  } else {
    console.warn('  STATUS: Check issues above');
  }
  console.log('═══════════════════════════════════════════');

  // ── 10. SUMMARY ──────────────────────────────────────────────────
  const finalElements = O.getElements();
  const finalWires = O.getWires();
  console.log('');
  console.log(`[OmniScript] Done. Elements: ${finalElements.length}, Wires: ${finalWires.length}`);
  console.log('[OmniScript] Dataflow: Button(onPress) → handler → loadData → statusSignal');

  return {
    elements: finalElements.length,
    wires: finalWires.length,
    validation: report,
  };
})();
