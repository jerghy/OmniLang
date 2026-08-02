// Ambient declarations for generated OmniLang React output.
// OmniLang emits `<Component>.module.css` imports next to each generated
// TSX; TypeScript needs a module declaration for those (vite would inject
// one, but the gate runs tsc standalone).

declare module '*.module.css' {
  const classes: Record<string, string>;
  export default classes;
}
