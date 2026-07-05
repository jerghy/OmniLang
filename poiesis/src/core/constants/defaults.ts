import { COMPONENT_PROTOCOL } from "../../data/componentProtocol";

export function getDefaultProps(tag: string): Record<string, unknown> {
  const meta = COMPONENT_PROTOCOL[tag];
  if (!meta) return {};
  const props: Record<string, unknown> = {};
  for (const [key, schema] of Object.entries(meta.props)) {
    if (schema.default !== undefined) props[key] = schema.default;
  }
  // Tag-specific defaults (props not covered by the schema `default` field)
  if (tag === "input" && !props.placeholder) props.placeholder = "输入...";
  if (tag === "textarea" && !props.placeholder) props.placeholder = "输入...";
  if (tag === "select" && !props.placeholder) props.placeholder = "选择...";
  if (tag === "img" && !props.alt) props.alt = "图片";
  return props;
}
