# Comments

> Source: [https://retejs.org/docs/guides/comments](https://retejs.org/docs/guides/comments)

# Comments

This guide is based on the Basic guide. It is recommended to review it for a comprehensive understanding of this guide.

Comments[Plugin](https://github.com/retejs/comment-plugin)

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete-comment-plugin

```

## Prepare nodes

```
tsclass Node extends ClassicPreset.Node {
  width = 180;
  height = 140;
}
class Connection<N extends Node> extends ClassicPreset.Connection<N, N> {}

type Schemes = GetSchemes<Node, Connection<Node>>;

```

## Plugin connection

```
tsimport { CommentPlugin } from "rete-comment-plugin";

const comment = new CommentPlugin<Schemes, AreaExtra>();

area.use(comment);

```

## Add comments programmatically

Adding an inline comment

```
tscomment.addInline("Inline comment text", [360, -20], node.dd);

```

where `[360, -20]` is the position of the comment, which can be placed freely in relation to the `node` it is attached to.

Adding a frame comment

```
tscomment.addFrame("Frame comment text", [node.id]);

```

where `[node.id]` refers to the nodes that are covered by this comment

## Selectable comments

```
tsimport { CommentExtensions } from "rete-comment-plugin";

const selector = AreaExtensions.selector();
const accumulating = AreaExtensions.accumulateOnCtrl();

CommentExtensions.selectable(comment, selector, accumulating);

```

## Edit comment text

The comment text can be edited by RMB. By default, a prompt with an input field will be displayed, but you can customize this.

```
tsconst comment = new CommentPlugin<Schemes, AreaExtra>({
  edit: async (comment) => {
    return prompt('Edit comment', comment.text)
  }
});

```

where the `edit` property must accept an asynchronous function that returns the text for the comment.

Check out the complete result on the Comments example page.
