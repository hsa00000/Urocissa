import { AlbumInfo, IsolationId } from '@type/types'
import { albumInfoSchema } from '@type/schemas'
import axios from 'axios'
import { defineStore } from 'pinia'
import { z } from 'zod'
import { tryWithMessageStore } from '@/script/utils/try_catch'

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
      async fetchAlbums() {
        await tryWithMessageStore('mainId', async () => {
          const response = await axios.get('/get/get-albums')

          if (response.status !== 200) {
            throw new Error('Network response was not ok')
          }

          const albums = z.array(albumInfoSchema).parse(response.data)

          albums.forEach((album) => {
            this.albums.set(album.albumId, album)
          })

          this.fetched = true
        })
      },
      clearAll() {
        this.albums.clear()
        this.fetched = false
      }
    }
  })()
