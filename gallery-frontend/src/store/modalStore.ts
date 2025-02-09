import { IsolationId } from '@/script/common/types'
import { defineStore } from 'pinia'

export const useModalStore = (isolationId: IsolationId) =>
  defineStore('modalStore' + isolationId, {
    state: (): {
      showEditTagsModal: boolean
      showBatchEditTagsModal: boolean
      showEditAlbumsModal: boolean
      showBatchEditAlbumsModal: boolean
      showUploadModal: boolean
      showIsolatedHomeModal: boolean
      showHomeTempModal: boolean
      showShareModal: boolean
    } => ({
      showEditTagsModal: false,
      showBatchEditTagsModal: false,
      showEditAlbumsModal: false,
      showBatchEditAlbumsModal: false,
      showUploadModal: false,
      showIsolatedHomeModal: false,
      showHomeTempModal: false,
      showShareModal: false
    }),
    actions: {}
  })()
