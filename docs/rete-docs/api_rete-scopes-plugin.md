# API for rete-scopes-plugin package

> Source: [https://retejs.org/docs/api/rete-scopes-plugin](https://retejs.org/docs/api/rete-scopes-plugin)

# API for rete-scopes-plugin package

## Scopes

[index.ts](https://github.com/retejs/scopes-plugin/blob/606dd5518c7326673eaec82b113fe502463d0cb5/src/index.ts#L33)

Signal types produced by ConnectionPlugin instance

## ScopesPlugin

[index.ts](https://github.com/retejs/scopes-plugin/blob/606dd5518c7326673eaec82b113fe502463d0cb5/src/index.ts#L46)

Scope plugin. Responsible for user interaction with scopes (nested nodes, groups)

```
tsclass ScopesPlugin<Schemes extends ExpectedScheme, T extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ExpectedScheme |  |
| T | unknown |  |

**Extends** Scope<Scopes, Requires<Schemes>, Root<Schemes>>

**Emits** `scopepicked` `scopereleased`

**Listens** `nodetranslated` `noderemoved`

### addPreset

[index.ts](https://github.com/retejs/scopes-plugin/blob/606dd5518c7326673eaec82b113fe502463d0cb5/src/index.ts#L136)

Adds a preset to the plugin.

```
tsaddPreset(preset: Preset): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| preset | Preset | Preset that is responsible for user interactions with scopes (e.g. assigning nodes to scopes) |

**Returns** `void`

## Presets

[presets/index.ts](https://github.com/retejs/scopes-plugin/blob/606dd5518c7326673eaec82b113fe502463d0cb5/src/presets/index.ts#L1)

Presets for scopes plugin.

### classic

[presets/classic/index.ts](https://github.com/retejs/scopes-plugin/blob/606dd5518c7326673eaec82b113fe502463d0cb5/src/presets/classic/index.ts#L1)

#### setup

[presets/classic/index.ts](https://github.com/retejs/scopes-plugin/blob/606dd5518c7326673eaec82b113fe502463d0cb5/src/presets/classic/index.ts#L13)

Classic preset allowing capturing a node by long-pressing it and dropping it onto another node to make it a nested.

```
tssetup(): Preset

```

**Emits** `scopepicked` `scopereleased`

**Listens** `nodepicked` `nodetranslated` `nodedragged`

**Returns** `Preset`
Preset

## Props

[index.ts](https://github.com/retejs/scopes-plugin/blob/606dd5518c7326673eaec82b113fe502463d0cb5/src/index.ts#L17)

Props for `ScopesPlugin` class.
