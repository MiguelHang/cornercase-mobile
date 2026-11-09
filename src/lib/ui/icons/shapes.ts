export type Rect = { kind: 'rect'; x: number; y: number; width: number; height: number; rx: number }
export type Line = { kind: 'line'; x1: number; y1: number; x2: number; y2: number }
export type Path = { kind: 'path'; d: string }
export type Shape = Rect | Line | Path

export const icons = {
  server: [
    { kind: 'rect', x: 2, y: 2, width: 20, height: 8, rx: 2 },
    { kind: 'rect', x: 2, y: 14, width: 20, height: 8, rx: 2 },
    { kind: 'line', x1: 6, y1: 6, x2: 6.01, y2: 6 },
    { kind: 'line', x1: 6, y1: 18, x2: 6.01, y2: 18 },
  ],
  'loader-circle': [{ kind: 'path', d: 'M21 12a9 9 0 1 1-6.219-8.56' }],
  'scan-qr-code': [
    { kind: 'path', d: 'M17 12v4a1 1 0 0 1-1 1h-4' },
    { kind: 'path', d: 'M17 3h2a2 2 0 0 1 2 2v2' },
    { kind: 'path', d: 'M17 8V7' },
    { kind: 'path', d: 'M21 17v2a2 2 0 0 1-2 2h-2' },
    { kind: 'path', d: 'M3 7V5a2 2 0 0 1 2-2h2' },
    { kind: 'path', d: 'M7 17h.01' },
    { kind: 'path', d: 'M7 21H5a2 2 0 0 1-2-2v-2' },
    { kind: 'rect', x: 7, y: 7, width: 5, height: 5, rx: 1 },
  ],
  'chevron-left': [{ kind: 'path', d: 'm15 18-6-6 6-6' }],
  'chevron-up': [{ kind: 'path', d: 'm18 15-6-6-6 6' }],
  'chevron-down': [{ kind: 'path', d: 'm6 9 6 6 6-6' }],
  'trash-2': [
    { kind: 'path', d: 'M3 6h18' },
    { kind: 'path', d: 'M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6' },
    { kind: 'path', d: 'M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2' },
    { kind: 'line', x1: 10, y1: 11, x2: 10, y2: 17 },
    { kind: 'line', x1: 14, y1: 11, x2: 14, y2: 17 },
  ],
} satisfies Record<string, Shape[]>

export type IconName = keyof typeof icons
