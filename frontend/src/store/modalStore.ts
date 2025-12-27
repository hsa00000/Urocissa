import { IsolationId } from '@type/types'
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
      showEditShareModal: boolean
      showDeleteShareModal: boolean
      showSettingModal: boolean
      showShareLoginModal: boolean
      showConfigModal: boolean
    } => ({
      showEditTagsModal: false,
      showBatchEditTagsModal: false,
      showEditAlbumsModal: false,
      showBatchEditAlbumsModal: false,
      showUploadModal: false,
      showIsolatedHomeModal: false,
      showHomeTempModal: false,
      showShareModal: false,
      showEditShareModal: false,
      showDeleteShareModal: false,
      showSettingModal: false,
      showShareLoginModal: false,
      showConfigModal: false
    }),
    actions: {}
  })()
