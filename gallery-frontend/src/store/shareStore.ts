import { IsolationId, ResolvedShare } from '@type/types'
import axios from 'axios'
import { defineStore } from 'pinia'
import { tryWithMessageStore } from '@/script/utils/try_catch'

export const useShareStore = (isolationId: IsolationId) =>
  defineStore('shareStore' + isolationId, {
    state: (): {
      albumId: null | string
      shareId: null | string
      password: null | string
      isAuthFailed: boolean // 您可以考慮保留或移除，目前我們主要用下面的
      isLinkExpired: boolean // <--- 新增這個狀態
      resolvedShare: null | ResolvedShare
      allShares: ResolvedShare[]
      fetched: boolean
    } => ({
      albumId: null,
      shareId: null,
      password: null,
      isAuthFailed: false,
      isLinkExpired: false, // <--- 初始化
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
      }
    }
  })()
