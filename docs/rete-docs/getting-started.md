# Getting started

> Source: [https://retejs.org/docs/getting-started](https://retejs.org/docs/getting-started)

# Getting started

ExamplesRete Kit[Codesandbox](https://codesandbox.io/s/rete-js-v2-yrzxe5)[Codepen](https://codepen.io/Ni55aN/pen/rNZKejd)

There are two easy ways to begin working with the framework: forking the examples on Codesandbox or creating a local application using Rete Kit. The first option allows for experimentation with functionality, which not always covered in the guides. Alternatively, the second option enables the creation of a local application with specific node editor features for a chosen version of **React.js**, **Vue.js**, **Angular**, **Svelte** or **Lit**. Afterward, following the guides will help familiarize yourself with the framework's features and capabilities.

## Prerequisites

Before diving into Rete.js, it's important to have an understanding of JavaScript or TypeScript fundamentals. The framework is primarily designed with TypeScript in mind, with examples and guides showcasing code in this language. However, for newcomers to TypeScript or those looking to quickly prototype, it's still possible to use Rete.js directly in JavaScript code.

If TypeScript is your preferred choice, make sure you have TypeScript version 4.7 or higher installed.

## Playgrounds

- [Codesandbox](https://codesandbox.io/s/rete-js-v2-yrzxe5)
- [Codepen](https://codepen.io/Ni55aN/pen/rNZKejd)

## AI Assistance

Get intelligent, context-aware help in your AI-powered IDE. Use your IDE's agent or command execution feature (available in Cursor, GitHub Copilot Chat, Windsurf, and similar tools) and simply ask your AI assistant:

```
promptRun npx rete-kit ai for [your intention]

```

Replace `[your intention]` with your actual goal, such as "learning Rete.js", "creating a new app", or "adding to my existing project". The AI agent will automatically determine the right options based on your intent. This generates IDE-specific instructions that help AI assistants understand Rete.js patterns and provide relevant guidance.

For more details, see the AI Assistance documentation.

## Creating an application using devkit

Use Rete Kit to quickly set up a Rete.js application. It lets you select a stack (**React.js**, **Vue.js**, **Angular**, **Svelte** or **Lit**) and the set of features.

## Adding Rete.js to your application

Framework packages are available on NPM and support common formats like ES (.esm.js), CommonJS (.common.js), and UMD (.min.js).

The command below provides an example of how to install the framework packages for the latest version.

```
bashnpm i rete rete-area-plugin rete-connection-plugin rete-render-utils rete-react-plugin react react-dom

```

For specific information on the required packages, refer one of the guides

## Usage from CDN

Framework packages are also available on numerous CDNs that serve npm packages. To add them to an HTML page, use the following example:

```
html<script src="https://cdn.jsdelivr.net/npm/rete/rete.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/rete-area-plugin/rete-area-plugin.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/rete-connection-plugin/rete-connection-plugin.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/rete-render-utils/rete-render-utils.min.js"></script>
<script src="https://cdn.jsdelivr.net/npm/rete-react-plugin/rete-react-plugin.min.js"></script>

```

Use these packages by accessing their namespace, which can be found in the `name` field of `rete.config.ts` file for each package. Make sure to add the required peer dependencies

```
jsRete
ReteAreaPlugin
ReteConnectionPlugin
ReteReactPlugin

```

Furthermore, you can integrate them on platforms like [Codepen](https://codepen.io) using [esm.sh](https://esm.sh).
