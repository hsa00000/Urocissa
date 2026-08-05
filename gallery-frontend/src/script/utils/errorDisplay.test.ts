import { afterEach, describe, expect, it, vi } from 'vitest'
import { errorDisplay, STALE_GENERATION_REFRESH_MESSAGE } from './errorDisplay'

function axiosErrorWithServerMessage(message: string): unknown {
  return {
    isAxiosError: true,
    message: 'Request failed with status code 409',
    response: {
      data: { message }
    }
  }
}

describe('errorDisplay', () => {
  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('asks the user to refresh when a selection generation is stale', () => {
    vi.spyOn(console, 'error').mockImplementation(() => undefined)

    expect(
      errorDisplay(axiosErrorWithServerMessage('selection index 4 has a stale generation'))
    ).toBe(STALE_GENERATION_REFRESH_MESSAGE)
  })

  it('preserves unrelated server error messages', () => {
    vi.spyOn(console, 'error').mockImplementation(() => undefined)

    expect(errorDisplay(axiosErrorWithServerMessage('album not found'))).toBe('album not found')
  })
})
