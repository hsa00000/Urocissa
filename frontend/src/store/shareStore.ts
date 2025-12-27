import { IsolationId, ResolvedShare } from '@type/types'
import axios from 'axios'
import { defineStore } from 'pinia'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import { storeShareInfo, clearShareInfo } from '@/db/db'

export const useShareStore = (isolationId: IsolationId) =>
  defineStore('shareStore' + isolationId, {
    state: (): {
      albumId: null | string
      shareId: null | string
      password: null | string
      isAuthFailed: boolean
      isLinkExpired: boolean
      resolvedShare: null | ResolvedShare
      allShares: ResolvedShare[]
      fetched: boolean
    } => ({
      albumId: null,
      shareId: null,
      password: null,
      isAuthFailed: false,
      isLinkExpired: false,
      resolvedShare: null,
      allShares: [],
      fetched: false
    }),
    actions: {
      async fetchAllShares() {
        await tryWithMessageStore('mainId', async () => {
          const response = await axios.get('/get/get-all-shares')

          if (response.status !== 200) {
            throw new Error('Network response was not ok')
          }

          this.allShares = response.data as ResolvedShare[]
          this.fetched = true
        })
      },
      async syncShareInfoToIndexedDB() {
        if (this.albumId && this.shareId) {
          await storeShareInfo({
            albumId: this.albumId,
            shareId: this.shareId,
            password: this.password
          })
        }
      },
      async clearShareInfoFromIndexedDB() {
        if (this.albumId && this.shareId) {
          await clearShareInfo(this.albumId, this.shareId)
        }
      }
    }
  })()
