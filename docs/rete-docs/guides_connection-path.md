# Connection path

> Source: [https://retejs.org/docs/guides/connection-path](https://retejs.org/docs/guides/connection-path)

# Connection path

This guide is based on the Basic guide. It is recommended to review it for a comprehensive understanding of this guide.

Connection path[Plugin](https://github.com/retejs/connection-path-plugin)

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete-connection-path-plugin d3-shape

```

## Plugin connection

The `curveStep` method is used for all connections in this example. Keep in mind that the plugin must be linked to the render plugin, which must emit the `connectionpath` event.

```
tsimport { ConnectionPathPlugin } from "rete-connection-path-plugin";
import { curveStep } from "d3-shape";

const pathPlugin = new ConnectionPathPlugin<Schemes, Area2D<Schemes>>({
  curve: () => curveStep
});

render.use(pathPlugin);

```

## Connection-specific path

The `curve` option takes a callback function with the connection instance as the first parameter, allowing you to customize the path type.

```
tsimport { curveStep, curveMonotoneX, curveLinear, CurveFactory } from "d3-shape";

class Connection extends ClassicPreset.Connection<
  ClassicPreset.Node,
  ClassicPreset.Node
> {
  curve?: CurveFactory;
}

const pathPlugin = new ConnectionPathPlugin<Schemes, Area2D<Schemes>>({
  curve: (connection) => connection.curve || curveStep
});

const monotoneConnection = new Connection(a, "port", b, "port");
const linearConnection = new Connection(a, "port", b, "port");

monotoneConnection.curve = curveMonotoneX;
linearConnection.curve = curveLinear;

```

Check out the complete result on the Connection path example page.
