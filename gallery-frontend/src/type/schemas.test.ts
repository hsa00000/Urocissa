import { describe, expect, it } from 'vitest'
import { BackendDataParser, PublicConfigSchema } from './schemas'

const config = {
  address: '127.0.0.1',
  port: 5673,
  limits: { json: '10MiB' },
  syncPaths: [],
  readOnlyMode: false,
  disableImg: false,
  writeBehind: {
    flushIntervalMs: 1_000,
    softLimitMiB: 16,
    hardLimitMiB: 32
  }
}

describe('public write-behind config schema', () => {
  it('accepts the defaults', () => {
    expect(PublicConfigSchema.parse(config).writeBehind).toEqual(config.writeBehind)
  })

  it('rejects invalid interval and memory limits', () => {
    expect(() =>
      PublicConfigSchema.parse({
        ...config,
        writeBehind: { flushIntervalMs: 99, softLimitMiB: 16, hardLimitMiB: 16 }
      })
    ).toThrow()
    expect(() =>
      PublicConfigSchema.parse({
        ...config,
        writeBehind: { flushIntervalMs: 1_000, softLimitMiB: 32, hardLimitMiB: 257 }
      })
    ).toThrow()
  })
})

describe('thumbnail cacheVersion schema', () => {
  const image = {
    type: 'image',
    id: 'image',
    pending: false,
    width: 100,
    height: 80,
    ext: 'jpg',
    size: 42
  }

  it('defaults old API payloads to version zero', () => {
    expect(BackendDataParser.parse(image).cacheVersion).toBe(0)
  })

  it('preserves a mirrored album cover version', () => {
    const album = BackendDataParser.parse({
      type: 'album',
      id: 'album',
      pending: false,
      title: null,
      startTime: null,
      endTime: null,
      lastModifiedTime: 1,
      cover: 'image',
      itemCount: 1,
      itemSize: 42,
      cacheVersion: 7
    })
    if (album.type !== 'album') throw new Error('expected album')
    expect(album.cover).toBe('image')
    expect(album.cacheVersion).toBe(7)
  })

  it('only accepts checked u32 values', () => {
    for (const cacheVersion of [-1, 1.5, 0x1_0000_0000]) {
      expect(BackendDataParser.safeParse({ ...image, cacheVersion }).success).toBe(false)
    }
  })
})
