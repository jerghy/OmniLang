# API for rete-connection-plugin package

> Source: [https://retejs.org/docs/api/rete-connection-plugin](https://retejs.org/docs/api/rete-connection-plugin)

# API for rete-connection-plugin package

## Connection

[types.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/types.ts#L29)

Signal types produced by ConnectionPlugin instance

```
tstype Connection =
  | { data: Object; type: "connectionpick" }
  | { data: Object; type: "connectiondrop" };

```

## ConnectionPlugin

[index.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/index.ts#L35)

Connection plugin. Responsible for user interaction with connections (creation, deletion)

```
tsclass ConnectionPlugin<Schemes extends ClassicScheme, K extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ClassicScheme |  |
| K | unknown |  |

**Extends** Scope<Connection, Requires | K>

**Emits** `connectionpick` `connectiondrop`

**Listens** `pointermove` `pointerup` `render` `unmount`

### addPreset

[index.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/index.ts#L51)

Add preset to the plugin

```
tsaddPreset(preset: Preset<Schemes>): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| preset | Preset<Schemes> | Preset to add |

**Returns** `void`

### drop

[index.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/index.ts#L77)

Drop pseudo-connection if exists

```
tsdrop(): void

```

**Emits** `connectiondrop`

**Returns** `void`

## Presets

[presets/index.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/presets/index.ts#L1)

Built-in presets

### classic

[presets/classic.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/presets/classic.ts#L1)

#### setup

[presets/classic.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/presets/classic.ts#L6)

Classic preset. Uses `ClassicFlow` for managing connections by user

```
tssetup(): () => ClassicFlow<ClassicScheme, any[]>

```

**Returns** `() => ClassicFlow<ClassicScheme, any[]>`

## BidirectFlow

[flow/builtin/bidirect.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/flow/builtin/bidirect.ts#L63)

Bidirect flow. User can pick a socket and connect it by releasing mouse button.
More simple than classic flow, but less functional (can't remove connection by clicking on input socket).

```
tsclass BidirectFlow<Schemes extends ClassicScheme, K extends any[]>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ClassicScheme |  |
| K | any[] |  |

**Implements** StateContext<Schemes, K>, Flow<Schemes, K>

## ClassicFlow

[flow/builtin/classic/index.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/flow/builtin/classic/index.ts#L135)

Classic flow. User can pick/click a socket and connect it by releasing/clicking on another socket.
If connection already exists and user clicks on input socket, connection will be removed.

```
tsclass ClassicFlow<Schemes extends ClassicScheme, K extends any[]>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ClassicScheme |  |
| K | any[] |  |

**Implements** StateContext<Schemes, K>, Flow<Schemes, K>

## BidirectParams

[flow/builtin/bidirect.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/flow/builtin/bidirect.ts#L8)

Bidirect flow params

```
tstype BidirectParams<Schemes extends ClassicScheme> = {
  makeConnection: Function;
  pickByClick: boolean;
};

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ClassicScheme |  |

## ClassicParams

[flow/builtin/classic/index.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/flow/builtin/classic/index.ts#L12)

Classic flow params

```
tstype ClassicParams<Schemes extends ClassicScheme> = {
  canMakeConnection: Function;
  makeConnection: Function;
};

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ClassicScheme |  |

## createPseudoconnection

[pseudoconnection.ts](https://github.com/retejs/connection-plugin/blob/7d951e9/src/pseudoconnection.ts#L11)

Create pseudoconnection. Used to trigger rendering of connection that is being created by user.
Has additional `isPseudo` property in payload.

```
tscreatePseudoconnection(extra: Partial<Schemes["Connection"]>): { mount: Function, unmount: Function, isMounted: unknown, render: unknown }

```

| Parameter | Type | Description |
| --- | --- | --- |
| extra | Partial<Schemes["Connection"]> | Extra payload to add to connection |

**Returns** `{ mount: Function, unmount: Function, isMounted: unknown, render: unknown }`
