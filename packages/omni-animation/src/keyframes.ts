import type { Keyframe, KeyframeStop, OmniProp } from './types'

/** Properties that must be mapped to CSS transform functions. */
const TRANSFORM_MAP: Record<string, string> = {
  x: 'translateX', y: 'translateY',
  scale: 'scale', scaleX: 'scaleX', scaleY: 'scaleY',
  rotate: 'rotate', rotateX: 'rotateX', rotateY: 'rotateY', rotateZ: 'rotateZ',
  skewX: 'skewX', skewY: 'skewY',
}

const TRANSFORM_PROPS = new Set(Object.keys(TRANSFORM_MAP))

/** Properties that need a "px" suffix when the value is a number. */
const PX_PROPS = new Set([
  'width', 'height', 'margin', 'padding', 'borderRadius', 'borderWidth',
  'marginTop', 'marginRight', 'marginBottom', 'marginLeft',
  'paddingTop', 'paddingRight', 'paddingBottom', 'paddingLeft',
  'borderTopLeftRadius', 'borderTopRightRadius',
  'borderBottomLeftRadius', 'borderBottomRightRadius',
  'gap', 'rowGap', 'columnGap', 'maxWidth', 'minWidth', 'maxHeight', 'minHeight',
  'top', 'left', 'right', 'bottom', 'fontSize', 'letterSpacing',
])

/** Properties that map to CSS filter functions. */
const FILTER_MAP: Record<string, string> = {
  blur: 'blur', brightness: 'brightness',
  contrast: 'contrast', saturate: 'saturate', hueRotate: 'hue-rotate',
}

/**
 * Convert Omni animation property names + values to CSS property names + formatted values.
 * Transform properties (x, y, scale, rotate, etc.) are combined into one `transform` string.
 * Filter properties (blur, brightness, etc.) are combined into one `filter` string.
 */
export function buildProperties(
  props: Partial<Record<OmniProp, number | string>>
): Record<string, string> {
  const cssProps: Record<string, string> = {}
  const transforms: string[] = []
  const filters: string[] = []

  if (!props) return cssProps

  for (const [key, value] of Object.entries(props)) {
    if (TRANSFORM_PROPS.has(key)) {
      const cssName = TRANSFORM_MAP[key]
      const num = typeof value === 'number' ? value : parseFloat(value as string)
      if (key === 'x' || key === 'y') {
        transforms.push(`${cssName}(${num}px)`)
      } else if (key.startsWith('rotate') || key.startsWith('skew')) {
        transforms.push(`${cssName}(${num}deg)`)
      } else {
        transforms.push(`${cssName}(${num})`)
      }
    } else if (key in FILTER_MAP) {
      const cssName = FILTER_MAP[key]
      const num = typeof value === 'number' ? value : parseFloat(value as string)
      if (key === 'blur') filters.push(`${cssName}(${num}px)`)
      else filters.push(`${cssName}(${num})`)
    } else {
      // Standard CSS property
      let formatted = value
      if (typeof value === 'number' && PX_PROPS.has(key)) {
        formatted = `${value}px`
      }
      cssProps[key] = String(formatted)
    }
  }

  if (transforms.length) cssProps.transform = transforms.join(' ')
  if (filters.length) cssProps.filter = filters.join(' ')
  return cssProps
}

/**
 * Convert Omni KeyframeStop[] to WAAPI Keyframe[].
 * Handles offset → CSS property mapping, transform/filter merging.
 */
export function buildKeyframes(stops: KeyframeStop[]): Keyframe[] {
  const sorted = [...stops].sort((a, b) => a.offset - b.offset)

  return sorted.map(stop => {
    const kf: Keyframe = { offset: stop.offset }
    const cssProps = buildProperties(stop.properties)
    Object.assign(kf, cssProps)
    return kf
  })
}
