import type { IsolationId } from '@type/types'

type ThumbnailElementsByIndex = Map<number, HTMLImageElement>

const thumbnailElements = new Map<IsolationId, ThumbnailElementsByIndex>()
export const transparentThumbnailSrc =
  'data:image/gif;base64,R0lGODlhAQABAAD/ACwAAAAAAQABAAACADs='

function applyThumbnailUrl(element: HTMLImageElement, url: string | undefined): void {
  if (url === undefined) {
    element.hidden = true
    element.removeAttribute('src')
    return
  }

  element.src = url
  element.hidden = false
}

export function registerThumbnailElement(
  isolationId: IsolationId,
  index: number,
  element: HTMLImageElement,
  cachedUrl: string | undefined
): void {
  let elementsByIndex = thumbnailElements.get(isolationId)
  if (elementsByIndex === undefined) {
    elementsByIndex = new Map()
    thumbnailElements.set(isolationId, elementsByIndex)
  }

  elementsByIndex.set(index, element)
  applyThumbnailUrl(element, cachedUrl)
}

export function unregisterThumbnailElement(
  isolationId: IsolationId,
  index: number,
  element: HTMLImageElement
): void {
  const elementsByIndex = thumbnailElements.get(isolationId)
  if (elementsByIndex?.get(index) !== element) {
    return
  }

  elementsByIndex.delete(index)
  if (elementsByIndex.size === 0) {
    thumbnailElements.delete(isolationId)
  }
}

export function publishThumbnailElement(
  isolationId: IsolationId,
  index: number,
  url: string
): void {
  const element = thumbnailElements.get(isolationId)?.get(index)
  if (element !== undefined) {
    applyThumbnailUrl(element, url)
  }
}

/**
 * Clears mounted image shells while retaining their registrations. This lets a
 * resize or store reset hide stale thumbnails immediately and accept fresh
 * worker results without waiting for a Vue render.
 */
export function clearThumbnailElements(isolationId: IsolationId): void {
  thumbnailElements.get(isolationId)?.forEach((element) => {
    applyThumbnailUrl(element, undefined)
  })
}

/** Clears mounted shells and releases all element references for an isolation. */
export function resetThumbnailElements(isolationId: IsolationId): void {
  clearThumbnailElements(isolationId)
  thumbnailElements.delete(isolationId)
}
