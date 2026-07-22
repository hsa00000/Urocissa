import axios, { type AxiosProgressEvent, type AxiosResponse } from 'axios'
import { createPinia, setActivePinia } from 'pinia'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { useModalStore } from './modalStore'
import { useUploadStore } from './uploadStore'

interface Deferred<T> {
  promise: Promise<T>
  resolve: (value: T) => void
  reject: (reason?: unknown) => void
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise
    reject = rejectPromise
  })
  return { promise, resolve, reject }
}

function response(): AxiosResponse {
  return {} as AxiosResponse
}

function mediaFile(name: string, size = 8): File {
  return new File([new Uint8Array(size)], name, {
    type: name.endsWith('.mp4') ? 'video/mp4' : 'image/jpeg',
    lastModified: 1234
  })
}

function uploadedFileName(call: unknown[]): string | undefined {
  const body = call[1]
  if (!(body instanceof FormData)) return undefined
  const file = body.get('file')
  return file instanceof File ? file.name : undefined
}

describe('upload queue store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  afterEach(() => {
    vi.restoreAllMocks()
  })

  it('uploads one file at a time in strict FIFO order', async () => {
    const firstRequest = deferred<AxiosResponse>()
    const post = vi
      .spyOn(axios, 'post')
      .mockImplementationOnce(() => firstRequest.promise)
      .mockResolvedValue(response())
    const store = useUploadStore('mainId')

    store.enqueueFiles([mediaFile('first.jpg'), mediaFile('second.jpg')])

    await vi.waitFor(() => {
      expect(post).toHaveBeenCalledTimes(1)
    })
    expect(uploadedFileName(post.mock.calls[0] ?? [])).toBe('first.jpg')
    expect(store.items.map((item) => item.status)).toEqual(['uploading', 'pending'])

    firstRequest.resolve(response())

    await vi.waitFor(() => {
      expect(post).toHaveBeenCalledTimes(2)
    })
    expect(uploadedFileName(post.mock.calls[1] ?? [])).toBe('second.jpg')
    await vi.waitFor(() => {
      expect(store.successItems).toHaveLength(2)
    })
  })

  it('reports per-file progress and continues after an error', async () => {
    const secondRequest = deferred<AxiosResponse>()
    const post = vi
      .spyOn(axios, 'post')
      .mockRejectedValueOnce(new Error('first failed'))
      .mockImplementationOnce((_url, _body, config) => {
        config?.onUploadProgress?.({ loaded: 8, total: 8 } as AxiosProgressEvent)
        return secondRequest.promise
      })
    const store = useUploadStore('mainId')

    store.enqueueFiles([mediaFile('broken.jpg'), mediaFile('working.jpg')])

    await vi.waitFor(() => {
      expect(post).toHaveBeenCalledTimes(2)
    })
    expect(store.items[0]?.status).toBe('error')
    expect(store.items[0]?.errorReason).toBe('first failed')
    expect(store.items[1]?.status).toBe('processing')

    secondRequest.resolve(response())
    await vi.waitFor(() => {
      expect(store.items[1]?.status).toBe('success')
    })
    expect(store.currentRunProgressPercent).toBe(100)
  })

  it('cancels active and pending files without starting the pending request', async () => {
    const post = vi.spyOn(axios, 'post').mockImplementation((_url, _body, config) => {
      return new Promise<AxiosResponse>((_resolve, reject) => {
        config?.signal?.addEventListener?.(
          'abort',
          () => {
            reject(new Error('aborted'))
          },
          { once: true }
        )
      })
    })
    const store = useUploadStore('mainId')

    store.enqueueFiles([mediaFile('active.jpg'), mediaFile('queued.jpg')])
    await vi.waitFor(() => {
      expect(post).toHaveBeenCalledTimes(1)
    })

    const activeId = store.items[0]?.id
    const queuedId = store.items[1]?.id
    expect(activeId).toBeDefined()
    expect(queuedId).toBeDefined()
    if (activeId === undefined || queuedId === undefined) return

    store.cancelItem(queuedId)
    store.cancelItem(activeId)

    await vi.waitFor(() => {
      expect(store.currentRunIsComplete).toBe(true)
    })
    expect(store.items.map((item) => item.status)).toEqual(['canceled', 'canceled'])
    expect(post).toHaveBeenCalledTimes(1)
  })

  it('puts a retry at the end of the current FIFO queue', async () => {
    const secondRequest = deferred<AxiosResponse>()
    const post = vi
      .spyOn(axios, 'post')
      .mockRejectedValueOnce(new Error('retry me'))
      .mockImplementationOnce(() => secondRequest.promise)
      .mockResolvedValue(response())
    const store = useUploadStore('mainId')

    store.enqueueFiles([
      mediaFile('first.jpg'),
      mediaFile('second.jpg'),
      mediaFile('third.jpg')
    ])

    await vi.waitFor(() => {
      expect(post).toHaveBeenCalledTimes(2)
    })
    const failedId = store.items[0]?.id
    expect(failedId).toBeDefined()
    if (failedId === undefined) return
    store.retryItem(failedId)

    secondRequest.resolve(response())
    await vi.waitFor(() => {
      expect(post).toHaveBeenCalledTimes(4)
    })

    expect(post.mock.calls.map((call) => uploadedFileName(call))).toEqual([
      'first.jpg',
      'second.jpg',
      'third.jpg',
      'first.jpg'
    ])
  })

  it('does not reveal the summary panel for upload-page enqueue or retry actions', async () => {
    vi.spyOn(axios, 'post')
      .mockRejectedValueOnce(new Error('retry silently'))
      .mockResolvedValue(response())
    const store = useUploadStore('mainId')
    const modalStore = useModalStore('mainId')
    const silentSummary = { showSummaryPanel: false }

    store.enqueueFiles([mediaFile('page-upload.jpg')], undefined, silentSummary)
    expect(modalStore.showUploadModal).toBe(false)

    await vi.waitFor(() => {
      expect(store.issueItems).toHaveLength(1)
    })
    const failedId = store.issueItems[0]?.id
    expect(failedId).toBeDefined()
    if (failedId === undefined) return

    store.retryItem(failedId, silentSummary)
    expect(modalStore.showUploadModal).toBe(false)

    await vi.waitFor(() => {
      expect(store.successItems).toHaveLength(1)
    })

    store.enqueueFiles([mediaFile('home-drop.jpg')])
    expect(modalStore.showUploadModal).toBe(true)
  })

  it('captures multiple album/tag presets, share context, and rejects unsupported files', async () => {
    const post = vi.spyOn(axios, 'post').mockResolvedValue(response())
    const store = useUploadStore('mainId')
    const target = {
      albums: [
        { id: 'album one', name: 'Holiday' },
        { id: 'album-two', name: 'Family' }
      ],
      tags: ['summer', 'favorite']
    }

    store.enqueueFiles([mediaFile('photo.jpg')], target)
    store.enqueueFiles([mediaFile('shared.jpg')], {
      albums: [{ id: 'shared-album', name: 'Shared' }],
      share: {
        albumId: 'shared-album',
        shareId: 'share-id',
        password: 'secret'
      }
    })
    store.enqueueFiles([mediaFile('notes.txt')])
    target.albums.push({ id: 'late-album', name: 'Too late' })
    target.tags.push('too-late')

    await vi.waitFor(() => {
      expect(store.successItems).toHaveLength(2)
    })
    expect(post).toHaveBeenCalledTimes(2)

    const presetUrl = new URL(String(post.mock.calls[0]?.[0]), 'https://example.test')
    expect(JSON.parse(presetUrl.searchParams.get('presigned_album_ids_opt') ?? '[]')).toEqual([
      'album one',
      'album-two'
    ])
    expect(JSON.parse(presetUrl.searchParams.get('presigned_tags_opt') ?? '[]')).toEqual([
      'summer',
      'favorite'
    ])
    expect(post.mock.calls[1]?.[0]).toBe(
      '/upload?presigned_album_id_opt=shared-album'
    )
    expect(post.mock.calls[1]?.[2]?.headers).toMatchObject({
      'x-album-id': 'shared-album',
      'x-share-id': 'share-id',
      'x-share-password': 'secret'
    })
    expect(store.issueItems).toHaveLength(1)
    expect(store.issueItems[0]?.errorReason).toBe('Unsupported file format')

    store.clearCompleted()
    store.clearIssues()
    expect(store.items).toHaveLength(0)
  })
})
