# API for rete-context-menu-plugin package

> Source: [https://retejs.org/docs/api/rete-context-menu-plugin](https://retejs.org/docs/api/rete-context-menu-plugin)

# API for rete-context-menu-plugin package

## ContextMenuExtra

[index.ts](https://github.com/retejs/context-menu-plugin/blob/5dcf64e/src/index.ts#L24)

Signal types produced by ContextMenuPlugin instance

```
tstype ContextMenuExtra = RenderSignal<
  "contextmenu",
  { items: Item[]; searchBar: boolean; onHide: unknown }
>;

```

## ContextMenuPlugin

[index.ts](https://github.com/retejs/context-menu-plugin/blob/5dcf64e/src/index.ts#L46)

Plugin for context menu.
Responsible for initialing rendering of context menu with predefined items.

```
tsclass ContextMenuPlugin<Schemes extends BaseSchemes>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |

**Extends** Scope<never, Requires<Schemes> | ContextMenuExtra>

**Emits** `render` `unmount`

**Listens** `unmount` `contextmenu` `pointerdown`

### constructor

[index.ts](https://github.com/retejs/context-menu-plugin/blob/5dcf64e/src/index.ts#L50)

```
tsconstructor(props: Props<Schemes>): ContextMenuPlugin<Schemes>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | Props<Schemes> | Properties |

**Returns** `ContextMenuPlugin<Schemes>`

## Props

[index.ts](https://github.com/retejs/context-menu-plugin/blob/5dcf64e/src/index.ts#L12)

Context menu plugin props

```
tstype Props<Schemes extends BaseSchemes> = {
  delay: number;
  items: Items<Schemes>;
};

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |

## Presets

[presets/index.ts](https://github.com/retejs/context-menu-plugin/blob/5dcf64e/src/presets/index.ts#L1)

Presets that create items for root and node specific context menu.

### classic

[presets/classic/index.ts](https://github.com/retejs/context-menu-plugin/blob/5dcf64e/src/presets/classic/index.ts#L1)

#### setup

[presets/classic/index.ts](https://github.com/retejs/context-menu-plugin/blob/5dcf64e/src/presets/classic/index.ts#L18)

Classic context menu preset.
Configures nodes/actions items for root and Delete/Clone items for nodes

```
tssetup(nodes: ItemDefinition<Schemes>[]): Items<Schemes>

```

**Examples**

```
tsPresets.classic.setup([
 ["Math", [
   ["Number", () => new NumberNode()],
 ]]
])

```

| Parameter | Type | Description |
| --- | --- | --- |
| nodes | ItemDefinition<Schemes>[] | List of items |

**Returns** `Items<Schemes>`
