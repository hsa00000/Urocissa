import { AlbumInfo } from '@/script/common/types'
import { albumInfoSchema } from '@/script/common/schemas'
import axios from 'axios'
import { defineStore } from 'pinia'
import { z, ZodError } from 'zod'

export const useAlbumStore = defineStore({
  id: 'albumStore',
  state: (): {
    albums: AlbumInfo[]
    fetched: boolean
  } => ({
    albums: [],
    fetched: false
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

        this.albums = albums
        this.albums.sort((a, b) => a.albumName.localeCompare(b.albumName))

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
})