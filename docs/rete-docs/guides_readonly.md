# Readonly

> Source: [https://retejs.org/docs/guides/readonly](https://retejs.org/docs/guides/readonly)

# Readonly

This guide is based on the Basic guide. It is recommended to review it for a comprehensive understanding of this guide.

Readonly[Plugin](https://github.com/retejs/readonly-plugin)

Prevent events
## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete-readonly-plugin

```

## Plugin connection

```
tsimport { ReadonlyPlugin } from "rete-readonly-plugin";

const readonly = new ReadonlyPlugin<Schemes>();

editor.use(readonly.root);
editor.use(area);
area.use(readonly.area);
area.use(render);

```

Make sure to follow the order to connect `readonly.root` and `readonly.area` before any other plugins.

## Enable

The plugin allows modifications to the editor by default, giving you the ability to establish the editor's initial appearance using methods such as `addNode`.

Use the following code to activate it:

```
tsreadonly.enable();

```

## Disable

Disabling can be done in a similar way:

```
tsreadonly.disable();

```

## Controls

When referring to the in-built controls provided by the framework, like `ClassicPreset.InputControl`, it's important to specify the readonly feature individually in the below manner:

```
tsnew ClassicPreset.InputControl('text', { readonly: true })

```

Since other controls can be custom, it is up to the developer to implement their "read-only" capability.

## Disable connection manipulation

Finally, if you have completed the Basic guide, make sure to remove the `ConnectionPlugin` import.

```
tsarea.use(connection); // should be removed

```

Check out the complete result on the Readonly example page.
