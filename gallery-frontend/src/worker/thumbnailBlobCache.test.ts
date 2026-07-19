import { describe, expect, it } from 'vitest'
import {
  LEGACY_THUMBNAIL_CACHE_NAME,
  THUMBNAIL_CACHE_NAME,
  ThumbnailBlobCache,
  type ThumbnailCacheStorage,
  type ThumbnailDiskCache,
  thumbnailCacheKey
} from './thumbnailBlobCache'

function requestUrl(request: RequestInfo | URL): string {
  return request instanceof Request ? request.url : new URL(request.toString()).toString()
}

class FakeDiskCache {
  readonly entries = new Map<string, Blob>()

  delete(request: RequestInfo | URL): Promise<boolean> {
    return Promise.resolve(this.entries.delete(requestUrl(request)))
  }

  keys(): Promise<readonly Request[]> {
    return Promise.resolve([...this.entries.keys()].map((url) => new Request(url)))
  }

  match(request: RequestInfo | URL): Promise<Response | undefined> {
    const blob = this.entries.get(requestUrl(request))
    return Promise.resolve(blob === undefined ? undefined : new Response(blob))
  }

  async put(request: RequestInfo | URL, response: Response): Promise<void> {
    this.entries.set(requestUrl(request), await response.blob())
  }
}

class FakeCacheStorage implements ThumbnailCacheStorage {
  readonly stores = new Map<string, FakeDiskCache>()

  delete(cacheName: string): Promise<boolean> {
    return Promise.resolve(this.stores.delete(cacheName))
  }

  open(cacheName: string): Promise<ThumbnailDiskCache> {
    let cache = this.stores.get(cacheName)
    if (cache === undefined) {
      cache = new FakeDiskCache()
      this.stores.set(cacheName, cache)
    }
    return Promise.resolve(cache)
  }
}

const hash = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'

describe('ThumbnailBlobCache', () => {
  it('isolates versions in memory and Cache Storage', async () => {
    const storage = new FakeCacheStorage()
    const cache = new ThumbnailBlobCache(storage)
    await cache.put(hash, 0, new Blob(['old']))
    await cache.put(hash, 1, new Blob(['new']))

    expect(await cache.get(hash, 0)).toBeUndefined()
    expect(await (await cache.get(hash, 1))?.text()).toBe('new')
    const disk = storage.stores.get(THUMBNAIL_CACHE_NAME)
    expect(disk?.entries.has(thumbnailCacheKey(hash, 0))).toBe(false)
    expect(disk?.entries.has(thumbnailCacheKey(hash, 1))).toBe(true)
  })

  it('does not let a late stale version evict a newer Blob', async () => {
    const storage = new FakeCacheStorage()
    const cache = new ThumbnailBlobCache(storage)
    await cache.put(hash, 2, new Blob(['newest']))
    await cache.put(hash, 1, new Blob(['stale']))

    expect(await cache.get(hash, 1)).toBeUndefined()
    expect(await (await cache.get(hash, 2))?.text()).toBe('newest')
  })

  it('removes the hash-only v1 Cache Storage on startup', async () => {
    const storage = new FakeCacheStorage()
    await storage.open(LEGACY_THUMBNAIL_CACHE_NAME)
    const cache = new ThumbnailBlobCache(storage)

    await cache.cleanupLegacy()

    expect(storage.stores.has(LEGACY_THUMBNAIL_CACHE_NAME)).toBe(false)
  })
})
