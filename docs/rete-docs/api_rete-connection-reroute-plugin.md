# API for rete-connection-reroute-plugin package

> Source: [https://retejs.org/docs/api/rete-connection-reroute-plugin](https://retejs.org/docs/api/rete-connection-reroute-plugin)

# API for rete-connection-reroute-plugin package

## RerouteProduces

[index.ts](https://github.com/retejs/connection-reroute-plugin/blob/a53d4a7/src/index.ts#L25)

Signal types produced by the plugin

```
tstype RerouteProduces =
  | { data: Object; type: "pintranslated" }
  | { data: Object; type: "pinselected" }
  | { data: Object; type: "pinunselected" };

```

## ReroutePlugin

[index.ts](https://github.com/retejs/connection-reroute-plugin/blob/a53d4a7/src/index.ts#L42)

Reroute plugin

```
tsclass ReroutePlugin<Schemes extends BaseSchemes>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |

**Extends** Scope<RerouteProduces, Requires<Schemes>, BaseArea<Schemes> | RerouteExtra>

**Emits** `pintranslated` `pinselected` `pinunselected`

**Listens** `rendered` `unmount` `reordered` `connectionpath` `pointerdown`

### add

[index.ts](https://github.com/retejs/connection-reroute-plugin/blob/a53d4a7/src/index.ts#L165)

Add a new pin to the connection

```
tsadd(connectionId: string, position: Position, index: number): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| connectionId | string | Connection id |
| position | Position | Pin position |
| index | number | Pin index, if not specified, the pin will be added to the end |

**Returns** `void`

### remove

[index.ts](https://github.com/retejs/connection-reroute-plugin/blob/a53d4a7/src/index.ts#L194)

Remove pin

```
tsremove(pinId: string): Promise<void>

```

| Parameter | Type | Description |
| --- | --- | --- |
| pinId | string | Pin id |

**Returns** `Promise<void>`

### select

[index.ts](https://github.com/retejs/connection-reroute-plugin/blob/a53d4a7/src/index.ts#L207)

Select pin

```
tsselect(pinId: string): Promise<void>

```

| Parameter | Type | Description |
| --- | --- | --- |
| pinId | string | Pin id |

**Returns** `Promise<void>`

### translate

[index.ts](https://github.com/retejs/connection-reroute-plugin/blob/a53d4a7/src/index.ts#L181)

Translate pin

```
tstranslate(pinId: string, dx: number, dy: number): Promise<void>

```

| Parameter | Type | Description |
| --- | --- | --- |
| pinId | string | Pin id |
| dx | number | Delta x |
| dy | number | Delta y |

**Returns** `Promise<void>`

### unselect

[index.ts](https://github.com/retejs/connection-reroute-plugin/blob/a53d4a7/src/index.ts#L221)

Unselect pin

```
tsunselect(pinId: string): Promise<void>

```

| Parameter | Type | Description |
| --- | --- | --- |
| pinId | string | Pin id |

**Returns** `Promise<void>`

### update

[index.ts](https://github.com/retejs/connection-reroute-plugin/blob/a53d4a7/src/index.ts#L235)

Update connection for the pin

```
tsupdate(pin: string | PinStorageRecord): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| pin | `string | PinStorageRecord` |

**Returns** `void`

## RerouteExtensions

[extensions/index.ts](https://github.com/retejs/connection-reroute-plugin/blob/a53d4a7/src/extensions/index.ts#L1)

Extensions for the connection reroute plugin

### selectablePins

[extensions/selectable.ts](https://github.com/retejs/connection-reroute-plugin/blob/a53d4a7/src/extensions/selectable.ts#L17)

Enables synchronization between pins and the selector

```
tsselectablePins(reroutePlugin: ReroutePlugin<S>, selector: Selector<SelectorEntity>, accumulating: { active: unknown }): void

```

**Listens** `pinselected` `pinunselected` `pintranslated`

| Parameter | Type | Description |
| --- | --- | --- |
| reroutePlugin | ReroutePlugin<S> | Reroute plugin instance |
| selector | Selector<SelectorEntity> | Selector instance |
| accumulating | { active: unknown } | Accumulating state |

**Returns** `void`

## RerouteExtra

[index.ts](https://github.com/retejs/connection-reroute-plugin/blob/a53d4a7/src/index.ts#L14)

Signal types consumed by the plugin

```
tstype RerouteExtra =
  | RenderSignal<"reroute-pins", { data: PinData }>
  | { data: Object; type: "unmount" };

```
