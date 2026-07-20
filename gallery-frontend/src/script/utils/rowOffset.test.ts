import { describe, expect, it } from 'vitest'
import { projectRelativeTop, projectVirtualTop } from './rowOffset'

describe('projectVirtualTop', () => {
  it('keeps million-photo coordinates near the physical buffer viewport', () => {
    const bufferHeight = 600_000
    const logicalTop = 119_997_600
    const committedScrollTop = 119_997_000

    expect(projectVirtualTop(logicalTop, committedScrollTop, bufferHeight)).toBe(200_600)
  })

  it('includes dynamic row offsets before projecting the coordinate', () => {
    expect(projectVirtualTop(120_000_125, 120_000_000, 600_000)).toBe(200_125)
  })

  it('keeps rows close together inside a group with a huge logical origin', () => {
    expect(projectRelativeTop(120_002_525, 120_000_125)).toBe(2_400)
  })
})
