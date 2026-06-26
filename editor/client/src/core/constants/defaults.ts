import { COMPONENT_PROTOCOL } from "../../data/componentProtocol";

export function getDefaultProps(tag: string): Record<string, unknown> {
  const meta = COMPONENT_PROTOCOL[tag];
  if (!meta) return {};
  const props: Record<string, unknown> = {};
  for (const [key, schema] of Object.entries(meta.props)) {
    if (schema.default !== undefined) props[key] = schema.default;
  }
  if (tag === "Input" && !props.placeholder) props.placeholder = "输入...";
  if (tag === "TextArea" && !props.placeholder) props.placeholder = "输入...";
  if (tag === "Select" && !props.placeholder) props.placeholder = "选择...";
  if (tag === "Image" && !props.alt) props.alt = "图片";
  if (tag === "Progress" && props.value === "0") props.value = "50";
  if (tag === "Alert" && !props.label) props.label = "提示信息";
  return props;
}
