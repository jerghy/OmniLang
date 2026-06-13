# API for rete-connection-path-plugin package

> Source: [https://retejs.org/docs/api/rete-connection-path-plugin](https://retejs.org/docs/api/rete-connection-path-plugin)

# API for rete-connection-path-plugin package

## Transformers

[transformers.ts](https://github.com/retejs/connection-path-plugin/blob/c87cbf0/src/transformers.ts#L1)

Built-in transformers, defining the auxiliary points for the path.

### classic

[transformers.ts](https://github.com/retejs/connection-path-plugin/blob/c87cbf0/src/transformers.ts#L29)

Classic transformer. Returns four points: start, auxiliary start, auxiliary end, end.

```
tsclassic(options: { curvature: number, vertical: boolean }): Transformer

```

| Parameter | Type | Description |
| --- | --- | --- |
| options | { curvature: number, vertical: boolean } | Options |

**Returns** `Transformer`
Four points

### linear

[transformers.ts](https://github.com/retejs/connection-path-plugin/blob/c87cbf0/src/transformers.ts#L13)

Linear transformer. Returns the same points.

```
tslinear(): Transformer

```

**Throws** Error if number of points is not equal to 2

**Returns** `Transformer`
Two points

## ConnectionPathPlugin

[index.ts](https://github.com/retejs/connection-path-plugin/blob/c87cbf0/src/index.ts#L32)

Connection path plugin. Allows to customize connection path and arrow.

```
tsclass ConnectionPathPlugin<Schemes extends BaseSchemes, K extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |
| K | unknown |  |

**Extends** Scope<never, RenderProduces<Schemes>, ...Area2DInherited<Schemes, K>>

**Listens** `connectionpath` `rendered`

### constructor

[index.ts](https://github.com/retejs/connection-path-plugin/blob/c87cbf0/src/index.ts#L37)

```
tsconstructor(props: Props<Schemes>): ConnectionPathPlugin<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | Props<Schemes> | Connection path plugin props |

**Returns** `ConnectionPathPlugin<Schemes, K>`

## Props

[index.ts](https://github.com/retejs/connection-path-plugin/blob/c87cbf0/src/index.ts#L18)

Connection path plugin props

```
tstype Props<Schemes extends BaseSchemes> = {
  arrow: Function;
  curve: Function;
  transformer: Function;
};

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |
