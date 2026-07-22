import { describe, expect, it, vi } from 'vitest'
import { handlePriorityEscape, type PriorityEscapeEvent } from './priorityEscape'

function keyboardEvent(key: string) {
  return {
    key,
    preventDefault: vi.fn(),
    stopImmediatePropagation: vi.fn(),
    stopPropagation: vi.fn()
  } satisfies PriorityEscapeEvent
}

describe('priority Escape handling', () => {
  it('consumes Escape and closes before other keyboard handlers can run', () => {
    const event = keyboardEvent('Escape')
    const close = vi.fn()

    expect(handlePriorityEscape(event, close)).toBe(true)
    expect(event.preventDefault).toHaveBeenCalledOnce()
    expect(event.stopPropagation).toHaveBeenCalledOnce()
    expect(event.stopImmediatePropagation).toHaveBeenCalledOnce()
    expect(close).toHaveBeenCalledOnce()
  })

  it('leaves non-Escape keyboard events untouched', () => {
    const event = keyboardEvent('Enter')
    const close = vi.fn()

    expect(handlePriorityEscape(event, close)).toBe(false)
    expect(event.preventDefault).not.toHaveBeenCalled()
    expect(event.stopPropagation).not.toHaveBeenCalled()
    expect(event.stopImmediatePropagation).not.toHaveBeenCalled()
    expect(close).not.toHaveBeenCalled()
  })
})
