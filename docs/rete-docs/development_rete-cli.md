# Rete CLI

> Source: [https://retejs.org/docs/development/rete-cli](https://retejs.org/docs/development/rete-cli)

# Rete CLI

[Rete CLI](https://github.com/retejs/rete-cli)[Rollup](https://rollupjs.org/)[ESLint](https://eslint.org/)[Jest](https://jestjs.io/)[TypeDoc](https://typedoc.org/)

Architecture
Rete CLI is a building tool with built-in support for TypeScript and ESLint along with predefined rules. Additionally, it includes Jest. These features make it simple to start building new plugins without having to setup your own environment for building, linting, and testing.

The build feature is based on Rollup and comes with pre-configured Babel presets for TypeScript support.

## Installation

```
bashnpm i -g rete-cli

```

## Building the project

The first step is to create a configuration file called `rete.config.ts`

```
tsimport { ReteOptions } from 'rete-cli'

export default<ReteOptions>{
  input: 'src/index.ts', // path to entry script
  name: 'Namespace' // namespace for UMD bundles
}

```

Run the command

```
bashrete build --config rete.config.ts

```

The generated `dist` directory is publish-ready and includes all required bundles, type definitions, README.md, and package.json files with their corresponding paths.

The `--watch` option can be used to trigger automatic project building upon save, while the `--outputs` option allows you to specify multiple output paths separated by commas for the build destination.

## Create an advanced configuration

Let's take a look at several configuration options that are supported:

- connect Rollup plugins
- specify third-party dependencies that shouldn't be bundled
- specify output path
- connect Babel plugins and presets

```
tsimport { ReteOptions } from 'rete-cli'

export default <ReteOptions>[ // config with multiple entries
  {
    input: 'src/foo/index.ts',
    name: 'Namespace'
    babel: {
      presets: [
        require('@babel/preset-env'), // used by default, but should be declared when you specifies 'presets'
        require('@babel/preset-typescript'), // used by default
        require('@babel/preset-react'),
      ]
    }
  },
  {
    input: 'src/bar/index.ts',
    name: 'Namespace'
    globals: {
      'rete': 'Rete',
    },
    plugins: [ // specify Rollup plugins
      commonjs(),
    ],
    babel: {
      plugins: [
        // include Babel plugins
      ]
    }
  }
]

```

## Linting

By default, running `rete build` command includes a linting step prior to generating the bundles. However, you can also perform linting independently by running a separate command.

```
bashrete lint

```
