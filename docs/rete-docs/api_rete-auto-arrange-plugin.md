# API for rete-auto-arrange-plugin package

> Source: [https://retejs.org/docs/api/rete-auto-arrange-plugin](https://retejs.org/docs/api/rete-auto-arrange-plugin)

# API for rete-auto-arrange-plugin package

## AutoArrangePlugin

[index.ts](https://github.com/retejs/auto-arrange-plugin/blob/bc4a09f/src/index.ts#L23)

Auto arrange plugin. Layouts the graph using `elk.js`

```
tsclass AutoArrangePlugin<Schemes extends ExpectedSchemes, T extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ExpectedSchemes |  |
| T | unknown |  |

**Extends** Scope<never, BaseArea<Schemes> | T, Root<Schemes>>

### addPreset

[index.ts](https://github.com/retejs/auto-arrange-plugin/blob/bc4a09f/src/index.ts#L36)

Adds a preset to the plugin, which will be used to layout the node ports and customize the layout options

```
tsaddPreset(preset: Preset): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| preset | Preset | Preset to add |

**Returns** `void`

### layout

[index.ts](https://github.com/retejs/auto-arrange-plugin/blob/bc4a09f/src/index.ts#L180)

Method to layout the graph

```
tslayout(props: { applier: Applier<Schemes, T>, options: LayoutOptions } & Partial<Context<Schemes>>): Promise<{ demonstration: string, result: ElkNode, source: string }>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { applier: Applier<Schemes, T>, options: LayoutOptions } & Partial<Context<Schemes>> | Options for the layout |

**Returns** `Promise<{ demonstration: string, result: ElkNode, source: string }>`
Debug information about the layout

## ArrangeAppliers

[appliers/index.ts](https://github.com/retejs/auto-arrange-plugin/blob/bc4a09f/src/appliers/index.ts#L1)

### StandardApplier

[appliers/list/standard.ts](https://github.com/retejs/auto-arrange-plugin/blob/bc4a09f/src/appliers/list/standard.ts#L10)

Standard applier. Applies the layout to the nodes and their children immediately

```
tsclass StandardApplier<S extends ExpectedSchemes, K extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| S | ExpectedSchemes |  |
| K | unknown |  |

**Extended by** TransitionApplier

**Extends** Applier<S, K>

### TransitionApplier

[appliers/list/transition/index.ts](https://github.com/retejs/auto-arrange-plugin/blob/bc4a09f/src/appliers/list/transition/index.ts#L25)

Transition applier. Applies the layout to the nodes and their children with transition

```
tsclass TransitionApplier<S extends ExpectedSchemes, K extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| S | ExpectedSchemes |  |
| K | unknown |  |

**Extends** StandardApplier<S, K>

#### constructor

[appliers/list/transition/index.ts](https://github.com/retejs/auto-arrange-plugin/blob/bc4a09f/src/appliers/list/transition/index.ts#L33)

```
tsconstructor(props: TransitionApplierProps): TransitionApplier<S, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | TransitionApplierProps | Transition applier props |

**Returns** `TransitionApplier<S, K>`

### TransitionApplierProps

[appliers/list/transition/index.ts](https://github.com/retejs/auto-arrange-plugin/blob/bc4a09f/src/appliers/list/transition/index.ts#L11)

Transition applier props

```
tstype TransitionApplierProps = {
  duration: number;
  needsLayout: Function;
  onTick: Function;
  timingFunction: Function;
};

```

## Presets

[presets/index.ts](https://github.com/retejs/auto-arrange-plugin/blob/bc4a09f/src/presets/index.ts#L1)

Built-in presets. Responsible for node port positioning.

### classic

[presets/classic/index.ts](https://github.com/retejs/auto-arrange-plugin/blob/bc4a09f/src/presets/classic/index.ts#L1)

#### setup

[presets/classic/index.ts](https://github.com/retejs/auto-arrange-plugin/blob/bc4a09f/src/presets/classic/index.ts#L7)

Classic preset. Input ports are positioned on the left bottom side of the node, output ports are positioned on the right top side of the node.

```
tssetup(props: { bottom: number, spacing: number, top: number }): Preset

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { bottom: number, spacing: number, top: number } |  |

**Returns** `Preset`
