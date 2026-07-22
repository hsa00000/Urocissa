import { describe, expect, it, vi } from 'vitest'
import {
  extractClipboardImageFiles,
  handleClipboardImagePaste,
  type ClipboardPasteEvent
} from './clipboardImageFiles'

function fileItem(file: File, type = file.type): DataTransferItem {
  return {
    kind: 'file',
    type,
    getAsFile: () => file
  } as DataTransferItem
}

function textItem(): DataTransferItem {
  return {
    kind: 'string',
    type: 'text/plain',
    getAsFile: () => null
  } as DataTransferItem
}

function clipboardData(items: DataTransferItem[], files: File[] = []): DataTransfer {
  return {
    items: items as unknown as DataTransferItemList,
    files: files as unknown as FileList
  } as DataTransfer
}

describe('clipboard image files', () => {
  it('extracts every image item in order and ignores text and video items', () => {
    const first = new File(['first'], 'first.jpg', {
      type: 'image/jpeg',
      lastModified: 10
    })
    const video = new File(['video'], 'clip.mp4', {
      type: 'video/mp4',
      lastModified: 20
    })
    const second = new File(['second'], 'second.png', {
      type: 'image/png',
      lastModified: 30
    })

    const files = extractClipboardImageFiles(
      clipboardData([fileItem(first), textItem(), fileItem(video), fileItem(second)])
    )

    expect(files).toEqual([first, second])
  })

  it('falls back to DataTransfer.files only when items contain no images', () => {
    const fallback = new File(['fallback'], 'fallback.webp', {
      type: 'image/webp',
      lastModified: 10
    })
    const itemImage = new File(['item'], 'item.png', {
      type: 'image/png',
      lastModified: 20
    })

    expect(extractClipboardImageFiles(clipboardData([textItem()], [fallback]))).toEqual([
      fallback
    ])
    expect(
      extractClipboardImageFiles(clipboardData([fileItem(itemImage)], [itemImage, fallback]))
    ).toEqual([itemImage])
  })

  it('generates supported names and timestamps for unnamed clipboard images', () => {
    const unnamed = new File(['pixels'], '', { type: 'image/png', lastModified: 0 })
    const mismatched = new File(['photo'], 'clipboard.tmp', {
      type: 'image/jpeg',
      lastModified: 42
    })

    const files = extractClipboardImageFiles(
      clipboardData([fileItem(unnamed), fileItem(mismatched)]),
      1_700_000_000_000
    )

    expect(files.map((file) => file.name)).toEqual([
      'clipboard-1700000000000-1.png',
      'clipboard-1700000000000-2.jpg'
    ])
    expect(files[0]?.type).toBe('image/png')
    expect(files[0]?.lastModified).toBe(1_700_000_000_000)
    expect(files[1]?.lastModified).toBe(42)
  })

  it('keeps unknown image formats unchanged so the upload queue can reject them', () => {
    const svg = new File(['<svg />'], 'vector.svg', {
      type: 'image/svg+xml',
      lastModified: 10
    })

    expect(extractClipboardImageFiles(clipboardData([fileItem(svg)]))).toEqual([svg])
  })

  it('prioritizes images and prevents native paste even when an input has focus', () => {
    const image = new File(['pixels'], 'pasted.png', {
      type: 'image/png',
      lastModified: 10
    })
    const preventDefault = vi.fn()
    const enqueueFiles = vi.fn()
    const event = {
      clipboardData: clipboardData([textItem(), fileItem(image)]),
      preventDefault,
      target: { tagName: 'INPUT' } as unknown as EventTarget
    } satisfies ClipboardPasteEvent

    expect(handleClipboardImagePaste(event, enqueueFiles)).toBe(true)
    expect(preventDefault).toHaveBeenCalledOnce()
    expect(enqueueFiles).toHaveBeenCalledWith([image])
  })

  it('leaves native paste untouched when the clipboard has no images', () => {
    const preventDefault = vi.fn()
    const enqueueFiles = vi.fn()
    const event = {
      clipboardData: clipboardData([textItem()]),
      preventDefault,
      target: { tagName: 'TEXTAREA' } as unknown as EventTarget
    } satisfies ClipboardPasteEvent

    expect(handleClipboardImagePaste(event, enqueueFiles)).toBe(false)
    expect(preventDefault).not.toHaveBeenCalled()
    expect(enqueueFiles).not.toHaveBeenCalled()
  })
})
