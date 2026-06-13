# Arrange nodes

> Source: [https://retejs.org/docs/guides/arrange](https://retejs.org/docs/guides/arrange)

# Arrange nodes

This guide is based on the Basic guide. It is recommended to review it for a comprehensive understanding of this guide.

ArrangeScopes[Arrange plugin](https://github.com/retejs/auto-arrange-plugin)[Elk.js](https://github.com/kieler/elkjs)

Architecture
## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
Prior to using this plugin, `elkjs` peer dependency must be installed separately.

```
bashnpm i rete-auto-arrange-plugin elkjs

```

Additionally, it may be necessary to install `web-worker` if your bundler doesn't recognize this dependency by default

```
bashnpm i web-worker

```

## Create the node base

The node's width and height need to be specified as `elkjs` requires these values, especially if the node sizes vary.

```
tsclass Node extends ClassicPreset.Node {
  width = 180;
  height = 120;
}

class Connection<N extends Node> extends ClassicPreset.Connection<N, N> {}

type Schemes = GetSchemes<Node, Connection<Node>>;

```

## Plugin connection

```
tsimport { AutoArrangePlugin, Presets as ArrangePresets } from "rete-auto-arrange-plugin";

const arrange = new AutoArrangePlugin<Schemes>();

arrange.addPreset(ArrangePresets.classic.setup());

area.use(arrange);

```

## Arrange added nodes

```
tsawait arrange.layout();

```

## Arrange nested nodes

Any node that has the `parent` property will be considered as a nested node within its corresponding parent node, as indicated by the id specified in this field. There are no additional settings required. The Scopes guide includes an example of how to use it in practice.

## Animated arrange

If you prefer a seamless transition of nodes position, you can apply an animated `TransitionApplier` with configurable animation duration and timing function

```
tsimport { ArrangeAppliers } from "rete-auto-arrange-plugin";

const applier = new ArrangeAppliers.TransitionApplier<Schemes, AreaExtra>({
  duration: 500,
  timingFunction: (t) => t,
  async onTick() {
    // called on every frame update
  }
});

await arrange.layout({ applier })

```

## Arrange options

Additionally, you can make use of [global options for elk.js](https://eclipse.dev/elk/reference/options.html). For instance, this empowers you to alter the spacing between nodes

```
tsawait arrange.layout({
  options: {
    'elk.spacing.nodeNode': 100,
    'elk.layered.spacing.nodeNodeBetweenLayers': 100
  }
});

```

Check out the complete result on the Arrange example page.
