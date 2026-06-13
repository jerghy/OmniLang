# API for rete-readonly-plugin package

> Source: [https://retejs.org/docs/api/rete-readonly-plugin](https://retejs.org/docs/api/rete-readonly-plugin)

# API for rete-readonly-plugin package

## ReadonlyPlugin

[index.ts](https://github.com/retejs/readonly-plugin/blob/6e94be3/src/index.ts#L20)

Readonly plugin, disables all user interactions

```
tsclass ReadonlyPlugin<Schemes extends BaseSchemes>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |

**Listens** `nodecreate` `connectioncreate` `nodetranslate` `connectionpick`

**Examples**

```
tsconst readonly = new ReadonlyPlugin<Schemes>();
editor.use(readonly.root);
editor.use(area);
area.use(readonly.area);
area.use(render);
connection.use(readonly.connection);
area.use(connection);

```

### enabled

[index.ts](https://github.com/retejs/readonly-plugin/blob/6e94be3/src/index.ts#L28)

Readonly mode state

```
tsenabled: boolean;

```

### disable

[index.ts](https://github.com/retejs/readonly-plugin/blob/6e94be3/src/index.ts#L59)

Disable the readonly mode

```
tsdisable(): void

```

**Returns** `void`

### enable

[index.ts](https://github.com/retejs/readonly-plugin/blob/6e94be3/src/index.ts#L52)

Enable the readonly mode

```
tsenable(): void

```

**Returns** `void`
