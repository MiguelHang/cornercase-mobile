export type Rect = { kind: 'rect'; x: number; y: number; width: number; height: number; rx: number }
export type Line = { kind: 'line'; x1: number; y1: number; x2: number; y2: number }
export type Path = { kind: 'path'; d: string }
export type Circle = { kind: 'circle'; cx: number; cy: number; r: number }
export type Shape = Rect | Line | Path | Circle

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
  'arrow-up': [
    { kind: 'path', d: 'm5 12 7-7 7 7' },
    { kind: 'path', d: 'M12 19V5' },
  ],
  'arrow-down': [
    { kind: 'path', d: 'M12 5v14' },
    { kind: 'path', d: 'm19 12-7 7-7-7' },
  ],
  'arrow-left': [
    { kind: 'path', d: 'm12 19-7-7 7-7' },
    { kind: 'path', d: 'M19 12H5' },
  ],
  'arrow-right': [
    { kind: 'path', d: 'M5 12h14' },
    { kind: 'path', d: 'm12 5 7 7-7 7' },
  ],
  'chevron-up': [{ kind: 'path', d: 'm18 15-6-6-6 6' }],
  'chevron-down': [{ kind: 'path', d: 'm6 9 6 6 6-6' }],
  'chevron-right': [{ kind: 'path', d: 'm9 18 6-6-6-6' }],
  'clipboard-paste': [
    { kind: 'path', d: 'M15 2H9a1 1 0 0 0-1 1v2c0 .6.4 1 1 1h6c.6 0 1-.4 1-1V3c0-.6-.4-1-1-1Z' },
    { kind: 'path', d: 'M8 4H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2M16 4h2a2 2 0 0 1 2 2v2M11 14h10' },
    { kind: 'path', d: 'm17 10 4 4-4 4' },
  ],
  'trash-2': [
    { kind: 'path', d: 'M3 6h18' },
    { kind: 'path', d: 'M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6' },
    { kind: 'path', d: 'M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2' },
    { kind: 'line', x1: 10, y1: 11, x2: 10, y2: 17 },
    { kind: 'line', x1: 14, y1: 11, x2: 14, y2: 17 },
  ],
  'image-plus': [
    { kind: 'path', d: 'M16 5h6' },
    { kind: 'path', d: 'M19 2v6' },
    { kind: 'path', d: 'M21 11.5V19a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7.5' },
    { kind: 'path', d: 'm21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21' },
    { kind: 'circle', cx: 9, cy: 9, r: 2 },
  ],
  check: [{ kind: 'path', d: 'M20 6 9 17l-5-5' }],
  'circle-alert': [
    { kind: 'circle', cx: 12, cy: 12, r: 10 },
    { kind: 'line', x1: 12, y1: 8, x2: 12, y2: 12 },
    { kind: 'line', x1: 12, y1: 16, x2: 12.01, y2: 16 },
  ],
  pencil: [
    {
      kind: 'path',
      d: 'M21.174 6.812a1 1 0 0 0-3.986-3.987L3.842 16.174a2 2 0 0 0-.5.83l-1.321 4.352a.5.5 0 0 0 .623.622l4.353-1.32a2 2 0 0 0 .83-.497z',
    },
    { kind: 'path', d: 'm15 5 4 4' },
  ],
  'rotate-ccw': [
    { kind: 'path', d: 'M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8' },
    { kind: 'path', d: 'M3 3v5h5' },
  ],
  smartphone: [
    { kind: 'rect', x: 5, y: 2, width: 14, height: 20, rx: 2 },
    { kind: 'path', d: 'M12 18h.01' },
  ],
  'git-compare': [
    { kind: 'circle', cx: 18, cy: 18, r: 3 },
    { kind: 'circle', cx: 6, cy: 6, r: 3 },
    { kind: 'path', d: 'M13 6h3a2 2 0 0 1 2 2v7' },
    { kind: 'path', d: 'M11 18H8a2 2 0 0 1-2-2V9' },
  ],
  square: [{ kind: 'rect', x: 3, y: 3, width: 18, height: 18, rx: 2 }],
  'square-check': [
    { kind: 'rect', x: 3, y: 3, width: 18, height: 18, rx: 2 },
    { kind: 'path', d: 'm9 12 2 2 4-4' },
  ],
  'square-terminal': [
    { kind: 'path', d: 'm7 11 2-2-2-2' },
    { kind: 'path', d: 'M11 13h4' },
    { kind: 'rect', x: 3, y: 3, width: 18, height: 18, rx: 2 },
  ],
} satisfies Record<string, Shape[]>

export type IconName = keyof typeof icons
