import { AlbumInfo, IsolationId } from '@/script/common/types'
import { albumInfoSchema } from '@/script/common/schemas'
import axios from 'axios'
import { defineStore } from 'pinia'
import { z, ZodError } from 'zod'

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
        try {
          const response = await axios.get('/get/get-albums')

          if (response.status !== 200) {
            throw new Error('Network response was not ok')
          }

          const albumsArraySchema = z.array(albumInfoSchema)
          const albums = albumsArraySchema.parse(response.data)

          albums.forEach((album) => {
            this.albums.set(album.albumId, album)
          })

          this.fetched = true
        } catch (error) {
          if (error instanceof ZodError) {
            console.error('Validation errors:', error.errors)
          } else {
            console.error('Failed to fetch tags:', error)
          }
        }
      }
    }
  })()
