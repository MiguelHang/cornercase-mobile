export type Rect = { kind: 'rect'; x: number; y: number; width: number; height: number; rx: number }
export type Line = { kind: 'line'; x1: number; y1: number; x2: number; y2: number }
export type Shape = Rect | Line

export const icons = {
  server: [
    { kind: 'rect', x: 2, y: 2, width: 20, height: 8, rx: 2 },
    { kind: 'rect', x: 2, y: 14, width: 20, height: 8, rx: 2 },
    { kind: 'line', x1: 6, y1: 6, x2: 6.01, y2: 6 },
    { kind: 'line', x1: 6, y1: 18, x2: 6.01, y2: 18 },
  ],
} satisfies Record<string, Shape[]>

export type IconName = keyof typeof icons
