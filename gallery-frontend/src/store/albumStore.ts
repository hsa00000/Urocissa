import { AlbumInfo, IsolationId } from '@type/types'
import { albumInfoSchema } from '@type/schemas'
import axios from 'axios'
import { defineStore } from 'pinia'
import { z } from 'zod'
import { tryWithMessageStore } from '@/script/utils/try_catch'

const pendingFetches = new Map<IsolationId, Promise<void>>()
const pendingRefreshes = new Map<IsolationId, Promise<void>>()

export const useAlbumStore = (isolationId: IsolationId) =>
  defineStore('albumStore' + isolationId, {
    state: (): {
      albums: Map<string, AlbumInfo> // id -> album
      fetched: boolean
      leaveAlbumPath: string | undefined
    } => ({
      albums: new Map(),
      fetched: false,
      leaveAlbumPath: undefined
    }),
    actions: {
      async _replaceAlbumsFromServer(): Promise<void> {
        await tryWithMessageStore('mainId', async () => {
          const response = await axios.get('/get/get-albums')

          if (response.status !== 200) {
            throw new Error('Network response was not ok')
          }

          const albums = z.array(albumInfoSchema).parse(response.data)
          this.albums = new Map(albums.map((album) => [album.albumId, album]))
          this.fetched = true
        })
      },
      async fetchAlbums() {
        const pendingRefresh = pendingRefreshes.get(isolationId)
        if (pendingRefresh !== undefined) return pendingRefresh
        if (this.fetched) return
        const pending = pendingFetches.get(isolationId)
        if (pending !== undefined) return pending

        const request = this._replaceAlbumsFromServer()
        pendingFetches.set(isolationId, request)

        try {
          await request
        } finally {
          if (pendingFetches.get(isolationId) === request) pendingFetches.delete(isolationId)
        }
      },
      async refreshAlbums(): Promise<void> {
        const pendingRefresh = pendingRefreshes.get(isolationId)
        if (pendingRefresh !== undefined) return pendingRefresh

        const pendingFetch = pendingFetches.get(isolationId)
        const request = (async () => {
          if (pendingFetch !== undefined) await pendingFetch
          await this._replaceAlbumsFromServer()
        })()
        pendingRefreshes.set(isolationId, request)

        try {
          await request
        } finally {
          if (pendingRefreshes.get(isolationId) === request) {
            pendingRefreshes.delete(isolationId)
          }
        }
      },
      clearAll() {
        this.albums.clear()
        this.fetched = false
      }
    }
  })()
