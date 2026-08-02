import { afterEach, describe, expect, it, vi } from 'vitest'
import axios from 'axios'
import { fetchRouteResource } from './fetchRouteResource'

const ID = 'f97b32636eedc084c47594fac0e74a24a15904cbab0f4ee458a6e44136cac2a0'

describe('route resource API', () => {
  afterEach(() => vi.restoreAllMocks())

  it('requests an authenticated direct snapshot with a bounded timeout', async () => {
    const get = vi.spyOn(axios, 'get').mockResolvedValue({
      data: {
        prefetch: { timestamp: 41, dataLength: 1, locateTo: 0 },
        token: 'snapshot-token',
        data: {
          abstractData: {
            type: 'image',
            id: ID,
            pending: false,
            width: 1920,
            height: 1080,
            ext: 'jpg',
            size: 100,
            tags: [],
            exifVec: {},
            albums: [],
            alias: [],
            isArchived: true
          },
          timestamp: 41,
          token: 'hash-token'
        }
      }
    })
    const controller = new AbortController()

    const snapshot = await fetchRouteResource(ID, controller.signal)

    expect(get).toHaveBeenCalledWith(`/get/resource/${ID}`, {
      signal: controller.signal,
      timeout: 15_000,
      headers: { 'x-silent-route-resource-error': 'true' }
    })
    expect(snapshot.prefetch).toEqual({ timestamp: 41, dataLength: 1, locateTo: 0 })
    expect(snapshot.data.abstractData).toMatchObject({
      id: ID,
      type: 'image',
      isArchived: true
    })
  })
})
