export const ZIndex = {
  base: 0,
  content: 1,
  navbar: 100,
  scrollbar: 200,
  componentOverlay: 500,
  dropdown: 800,
  modal: 2400,
  top: 3000,
  fullscreen: 10000,
  uploadModalMax: 20000
} as const

export type ZIndexKey = keyof typeof ZIndex
