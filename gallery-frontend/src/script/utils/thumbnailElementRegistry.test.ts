import { beforeEach, describe, expect, it } from 'vitest'
import type { IsolationId } from '@type/types'
import {
  clearThumbnailElements,
  publishThumbnailElement,
  registerThumbnailElement,
  resetThumbnailElements,
  unregisterThumbnailElement
} from './thumbnailElementRegistry'

interface FakeImageElement {
  hidden: boolean
  attributes: Map<string, string>
  src: string
  getAttribute(name: string): string | null
  removeAttribute(name: string): void
  setAttribute(name: string, value: string): void
}

function createImageElement(): HTMLImageElement {
  const attributes = new Map<string, string>()
  const element: FakeImageElement = {
    hidden: false,
    attributes,
    get src() {
      return attributes.get('src') ?? ''
    },
    set src(value) {
      attributes.set('src', value)
    },
    getAttribute(name) {
      return attributes.get(name) ?? null
    },
    removeAttribute(name) {
      attributes.delete(name)
    },
    setAttribute(name, value) {
      attributes.set(name, value)
    }
  }
  return element as unknown as HTMLImageElement
}

const mainId = 'mainId' as IsolationId
const compareId = 'compareId' as IsolationId

describe('thumbnailElementRegistry', () => {
  beforeEach(() => {
    resetThumbnailElements(mainId)
    resetThumbnailElements(compareId)
  })

  it('applies a cached URL immediately when an image shell mounts', () => {
    const element = createImageElement()

    registerThumbnailElement(mainId, 4, element, 'blob:cached')

    expect(element.getAttribute('src')).toBe('blob:cached')
    expect(element.hidden).toBe(false)
  })

  it('keeps an uncached shell transparent and publishes only to its isolation and index', () => {
    const target = createImageElement()
    const otherIndex = createImageElement()
    const otherIsolation = createImageElement()
    registerThumbnailElement(mainId, 4, target, undefined)
    registerThumbnailElement(mainId, 5, otherIndex, undefined)
    registerThumbnailElement(compareId, 4, otherIsolation, undefined)

    publishThumbnailElement(mainId, 4, 'blob:ready')

    expect(target.getAttribute('src')).toBe('blob:ready')
    expect(target.hidden).toBe(false)
    expect(otherIndex.getAttribute('src')).toBeNull()
    expect(otherIndex.hidden).toBe(true)
    expect(otherIsolation.getAttribute('src')).toBeNull()
    expect(otherIsolation.hidden).toBe(true)
  })

  it('stops publishing to an unmounted or reassigned shell', () => {
    const element = createImageElement()
    registerThumbnailElement(mainId, 4, element, undefined)
    unregisterThumbnailElement(mainId, 4, element)
    registerThumbnailElement(mainId, 5, element, undefined)

    publishThumbnailElement(mainId, 4, 'blob:stale')
    expect(element.getAttribute('src')).toBeNull()
    expect(element.hidden).toBe(true)

    publishThumbnailElement(mainId, 5, 'blob:current')
    expect(element.getAttribute('src')).toBe('blob:current')
    expect(element.hidden).toBe(false)
  })

  it('keeps a replacement shell registered when the prior shell unmounts later', () => {
    const prior = createImageElement()
    const replacement = createImageElement()
    registerThumbnailElement(mainId, 4, prior, undefined)
    registerThumbnailElement(mainId, 4, replacement, undefined)

    unregisterThumbnailElement(mainId, 4, prior)
    publishThumbnailElement(mainId, 4, 'blob:replacement')

    expect(prior.getAttribute('src')).toBeNull()
    expect(replacement.getAttribute('src')).toBe('blob:replacement')
  })

  it('keeps registrations across store clears but releases them on reset', () => {
    const element = createImageElement()
    registerThumbnailElement(mainId, 4, element, 'blob:old')

    clearThumbnailElements(mainId)
    expect(element.getAttribute('src')).toBeNull()
    expect(element.hidden).toBe(true)

    publishThumbnailElement(mainId, 4, 'blob:resized')
    expect(element.getAttribute('src')).toBe('blob:resized')

    resetThumbnailElements(mainId)
    publishThumbnailElement(mainId, 4, 'blob:terminated')
    expect(element.getAttribute('src')).toBeNull()
    expect(element.hidden).toBe(true)
  })
})
