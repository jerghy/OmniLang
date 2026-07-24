/** @type { import('@storybook/react-vite').StorybookConfig } */
const config = {
  stories: [
    "../poiesis/src/**/*.stories.@(ts|tsx)",
  ],
  addons: [
    "@storybook/addon-essentials",
    "@storybook/addon-interactions",
  ],
  framework: "@storybook/react-vite",
  docs: {
    autodocs: true,
  },
};
export default config;
