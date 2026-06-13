# API for rete-history-plugin package

> Source: [https://retejs.org/docs/api/rete-history-plugin](https://retejs.org/docs/api/rete-history-plugin)

# API for rete-history-plugin package

## HistoryExtensions

[extensions/index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/extensions/index.ts#L1)

History plugin extensions

### keyboard

[extensions/keyboard.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/extensions/keyboard.ts#L10)

Adds keyboard shortcuts for history undo/redo

```
tskeyboard(plugin: HistoryPlugin<Schemes, A>): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| plugin | HistoryPlugin<Schemes, A> | History plugin instance |

**Returns** `void`

## Presets

[presets/index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/presets/index.ts#L1)

Presets for the history plugin

### classic

[presets/classic/index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/presets/classic/index.ts#L1)

#### setup

[presets/classic/index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/presets/classic/index.ts#L121)

Classic preset for the history plugin. Tracks node adding/removing/translating, connection adding/removing.

```
tssetup(props: { timing: number }): Preset<S, HistoryActions<S>>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { timing: number } |  |

**Returns** `Preset<S, HistoryActions<S>>`

## HistoryPlugin

[index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/index.ts#L17)

History plugin. Allows to undo/redo actions such as adding/removing nodes, connections, etc.

```
tsclass HistoryPlugin<Schemes extends BaseSchemes, A extends HistoryAction>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |
| A | HistoryAction |  |

**Extends** Scope<never, BaseArea<Schemes>, Root<Schemes>>

### constructor

[index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/index.ts#L30)

```
tsconstructor(props: { timing: number }): HistoryPlugin<Schemes, A>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { timing: number } | History plugin properties |

**Returns** `HistoryPlugin<Schemes, A>`

### add

[index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/index.ts#L65)

Adds an action to the history

```
tsadd(action: A): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| action | A | Action instance |

**Returns** `void`

### addPreset

[index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/index.ts#L56)

Adds a preset to the history plugin

```
tsaddPreset(preset: Preset<Schemes, T>): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| preset | Preset<Schemes, T> | Preset that manages some actions, e.g. classic preset for adding/removing nodes, connections, etc. |

**Returns** `void`

### clear

[index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/index.ts#L89)

Clear history

```
tsclear(): void

```

**Returns** `void`

### getHistorySnapshot

[index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/index.ts#L73)

Get produced history records

```
tsgetHistorySnapshot(): HistoryRecord<A>[]

```

**Returns** `HistoryRecord<A>[]`
History snapshot

### getRecent

[index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/index.ts#L82)

Get recent history records produced in the last `ms` milliseconds

```
tsgetRecent(ms: number): HistoryRecord<A>[]

```

| Parameter | Type | Description |
| --- | --- | --- |
| ms | number | Delta time in milliseconds |

**Returns** `HistoryRecord<A>[]`
History records produced in the last

### redo

[index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/index.ts#L120)

Asynchronously redoes an actions until the `separated` flag is encountered or the time between the current and previous action exceeds `timing` property which is 200ms by default

```
tsredo(): Promise<void>

```

**Returns** `Promise<void>`

### separate

[index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/index.ts#L96)

```
tsseparate(): void

```

**Returns** `void`

### undo

[index.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/index.ts#L105)

Asynchronously undoes an actions until the `separated` flag is encountered or the time between the current and previous action exceeds `timing` property which is 200ms by default

```
tsundo(): Promise<void>

```

**Returns** `Promise<void>`

## HistoryAction

[types.ts](https://github.com/retejs/history-plugin/blob/a678dfb/src/types.ts#L12)

Base interface for all actions

**Examples**

```
tsexport class CustomAction<S extends BaseSchemes> implements HistoryAction {
 async undo() {
   // undo logic
 }
 async redo() {
   // redo logic
 }
}

```
