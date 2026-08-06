import { describe, expect, it } from 'vitest'
import { projectRelativeTop, projectVirtualBottom, projectVirtualTop } from './rowOffset'

describe('projectVirtualTop', () => {
  it('keeps million-photo coordinates near the physical buffer viewport', () => {
    const logicalTop = 119_997_600
    const projectionOrigin = 200_000 - 119_997_000

    expect(projectVirtualTop(logicalTop, projectionOrigin)).toBe(200_600)
  })

  it('includes dynamic row offsets before projecting the coordinate', () => {
    expect(projectVirtualTop(120_000_125, 200_000 - 120_000_000)).toBe(200_125)
  })

  it('keeps rows close together inside a group with a huge logical origin', () => {
    expect(projectRelativeTop(120_002_525, 120_000_125)).toBe(2_400)
  })
})

describe('projectVirtualBottom', () => {
  it('keeps the projection bounded when logical height exceeds Chrome layout limits', () => {
    expect(projectVirtualBottom(119_996_371, 119_999_321, 671)).toBe(3_621)
  })

  it('absorbs an equal row and total-height shift without moving the bottom projection', () => {
    const before = projectVirtualBottom(914_720, 914_521, 809)
    const after = projectVirtualBottom(912_934, 912_735, 809)

    expect(after).toBe(before)
  })
})
