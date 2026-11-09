import type { Colour, Colours, Style } from '../bindings'

export type Rgb = [number, number, number]

export type Css = Record<string, string>

export const FOREGROUND: Rgb = [192, 202, 245]
export const BACKGROUND: Rgb = [26, 27, 38]

const RED: Rgb = [247, 118, 142]
const GREEN: Rgb = [158, 206, 106]
const YELLOW: Rgb = [224, 175, 104]
const BLUE: Rgb = [122, 162, 247]
const MAGENTA: Rgb = [187, 154, 247]
const CYAN: Rgb = [125, 207, 255]

const ANSI: Rgb[] = [
  [21, 22, 30],
  RED,
  GREEN,
  YELLOW,
  BLUE,
  MAGENTA,
  CYAN,
  [169, 177, 214],
  [86, 95, 137],
  RED,
  GREEN,
  YELLOW,
  BLUE,
  MAGENTA,
  CYAN,
  FOREGROUND,
]

const DIM = 0.6

const UNDERLINES: Partial<Record<string, string>> = {
  double: 'double',
  curly: 'wavy',
  dotted: 'dotted',
  dashed: 'dashed',
}

const level = (step: number) => (step === 0 ? 0 : 55 + step * 40)

const cube = (index: number): Rgb => {
  const step = index - 16
  return [level(Math.floor(step / 36)), level(Math.floor(step / 6) % 6), level(step % 6)]
}

const grey = (index: number): Rgb => {
  const value = 8 + (index - 232) * 10
  return [value, value, value]
}

export const tokyoNight = (index: number): Rgb => {
  if (index < 16) return ANSI[index]
  return index < 232 ? cube(index) : grey(index)
}

export const resolve = (colour: Colour, colours: Colours): Rgb => {
  if (colour.rgb) return colour.rgb
  return colours.palette?.[String(colour.index)] ?? tokyoNight(colour.index)
}

export const paneColours = (colours: Colours) => {
  const fg = colours.fg ?? FOREGROUND
  return { fg, bg: colours.bg ?? BACKGROUND, cursor: colours.cursor ?? fg }
}

export const rgb = ([red, green, blue]: Rgb, alpha = 1) =>
  alpha === 1 ? `rgb(${red} ${green} ${blue})` : `rgb(${red} ${green} ${blue} / ${alpha})`

const ends = (style: Style, colours: Colours) => {
  const pane = paneColours(colours)
  const fg = style.fg ? resolve(style.fg, colours) : pane.fg
  const bg = style.bg ? resolve(style.bg, colours) : pane.bg
  return style.inverse ? { fg: bg, bg: fg } : { fg, bg }
}

const alpha = (style: Style) => {
  if (style.invisible) return 0
  return style.dim ? DIM : 1
}

const underline = (style: Style, colours: Colours): Css => {
  const css: Css = {}
  const kind = UNDERLINES[style.underline_style ?? '']
  if (kind) css['text-decoration-style'] = kind
  if (style.underline_colour) css['text-decoration-color'] = rgb(resolve(style.underline_colour, colours))
  return css
}

const decoration = (style: Style, colours: Colours): Css => {
  const lines = [style.underline && 'underline', style.overline && 'overline', style.strikethrough && 'line-through']
    .filter(Boolean)
  if (lines.length === 0) return {}
  const css: Css = { 'text-decoration-line': lines.join(' ') }
  return style.underline ? { ...css, ...underline(style, colours) } : css
}

export const runCss = (style: Style, colours: Colours): Css => {
  const { fg, bg } = ends(style, colours)
  const css: Css = { color: rgb(fg, alpha(style)) }
  if (style.bg || style.inverse) css.background = rgb(bg)
  if (style.bold) css['font-weight'] = 'bold'
  if (style.italic) css['font-style'] = 'italic'
  return { ...css, ...decoration(style, colours) }
}

export const inline = (css: Css) =>
  Object.entries(css)
    .map(([property, value]) => `${property}: ${value}`)
    .join('; ')
