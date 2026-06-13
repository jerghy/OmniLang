# Undo/Redo

> Source: [https://retejs.org/docs/guides/undo-redo](https://retejs.org/docs/guides/undo-redo)

# Undo/Redo

This guide is based on the Basic guide. It is recommended to review it for a comprehensive understanding of this guide.

History[Plugin](https://github.com/retejs/history-plugin)

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete-history-plugin

```

## Plugin connection

```
tsimport { HistoryPlugin, HistoryActions } from "rete-history-plugin";

const history = new HistoryPlugin<Schemes, HistoryActions<Schemes>>();

area.use(history);

```

## Classic preset

Similar to the majority of plugins, it relies on a preset to unlock its complete functionality.

```
tsimport { Presets } from "rete-history-plugin";

history.addPreset(Presets.classic.setup())

```

It is responsible for tracking the history of adding, removing, or translating nodes, as well as tracking the adding and removing of connections.

## Usage

The plugin doesn't track user actions by default, such as keyboard shortcuts. To enable user interaction, you can implement usage of `undo`/`redo` methods on your own

```
tsawait history.undo()
await history.redo()

```

Alternatively, you can use the default extension that tracks the `Ctrl+Z`/`Ctrl+Y` shortcuts

```
tsimport { HistoryExtensions } from "rete-history-plugin";

HistoryExtensions.keyboard(history);

```

## History management

If you need additional functionality beyond node and connection tracking, you have the option to create custom actions and add them to the history

```
tsclass MyAction implements HistoryAction {
  undo(): void | Promise<void> {
    // undo action
  }
  redo(): void | Promise<void> {
    // redo action
  }
}

const history = new HistoryPlugin<Schemes, HistoryActions<Schemes> | MyAction>();

const myAction = new MyAction()

history.add(myAction)

```

To retrieve a history list, use the `getHistorySnapshot` or `getRecent` methods (please note that modifying the history directly is not possible)

```
tsconst list = history.getHistorySnapshot()
const recentFor1s = history.getRecent(1000)

```

By using the latest method, we retrieve a list of actions within the past second. Since these actions are time-dependent, the plugin can intelligently group them together, reducing the need for users to repeatedly press 'Ctrl + Z' when multiple actions occur within a short time frame. This is particularly useful when a user deletes a node, triggering the automatic removal of its associated connections.

Additionally, you can configure the timing as follows, where `200` is the default time threshold in milliseconds. If the time interval between adjacent actions is shorter than the specified time, the plugin will consider them to be part of the same group.

```
tsconst history = new HistoryPlugin<Schemes>({ timing: 200 });

```

In order to make the undo or redo process independent of time constraints between actions, you can adopt the following method:

```
ts// action 1
// action 2
history.separate() // or myAction.separated = true
// action 3

```

In this way, when the `undo` command is triggered for the first time, it will only undo the third action, even the time interval between it and the preceding action is less than 200 milliseconds.

Check out the complete result on the History example page.
