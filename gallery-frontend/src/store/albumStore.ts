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
      },
      optimisticAddAlbums(_albumIds: string[]) {
        // 目前 AlbumInfo 沒有計數屬性 (count/number)，所以此處暫無計數更新邏輯。
        // 未來若 AlbumInfo 加入 itemCount，可在此處實作類似 tagStore 的邏輯：
        // albumIds.forEach(id => { const album = this.albums.get(id); if (album) album.itemCount++; })
      },
      optimisticRemoveAlbums(_albumIds: string[]) {
        // 目前 AlbumInfo 沒有計數屬性 (count/number)，所以此處暫無計數更新邏輯。
        // 未來若 AlbumInfo 加入 itemCount，可在此處實作類似 tagStore 的邏輯：
        // albumIds.forEach(id => { const album = this.albums.get(id); if (album && album.itemCount > 0) album.itemCount--; })
      }
    }
  })()
