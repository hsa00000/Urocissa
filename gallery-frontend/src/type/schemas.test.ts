import { describe, expect, it } from 'vitest'
import { PublicConfigSchema } from './schemas'

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
