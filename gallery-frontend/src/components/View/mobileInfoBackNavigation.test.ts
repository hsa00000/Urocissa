import { describe, expect, it, vi } from 'vitest'
import { interceptMobileInfoBackNavigation } from './mobileInfoBackNavigation'

describe('interceptMobileInfoBackNavigation', () => {
  it('closes an open mobile info panel and cancels route navigation', async () => {
    const closeInfo = vi.fn().mockResolvedValue(undefined)
    const onCloseError = vi.fn()

    const result = await interceptMobileInfoBackNavigation({
      isMobile: true,
      isInfoOpen: true,
      closeInfo,
      onCloseError
    })

    expect(result).toBe(false)
    expect(closeInfo).toHaveBeenCalledOnce()
    expect(onCloseError).not.toHaveBeenCalled()
  })

  it.each([
    { isMobile: false, isInfoOpen: true },
    { isMobile: true, isInfoOpen: false }
  ])('allows navigation when the panel is not an open mobile panel', async (state) => {
    const closeInfo = vi.fn().mockResolvedValue(undefined)

    const result = await interceptMobileInfoBackNavigation({
      ...state,
      closeInfo,
      onCloseError: vi.fn()
    })

    expect(result).toBeUndefined()
    expect(closeInfo).not.toHaveBeenCalled()
  })

  it('still cancels navigation if persisting the closed state fails', async () => {
    const error = new Error('failed to persist')
    const onCloseError = vi.fn()

    const result = await interceptMobileInfoBackNavigation({
      isMobile: true,
      isInfoOpen: true,
      closeInfo: vi.fn().mockRejectedValue(error),
      onCloseError
    })

    expect(result).toBe(false)
    expect(onCloseError).toHaveBeenCalledWith(error)
  })
})
