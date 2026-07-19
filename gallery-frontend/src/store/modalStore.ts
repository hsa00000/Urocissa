import type { IsolationId } from '@type/types'
import type { SelectionDescriptor } from '@/type/selection'
import { defineStore } from 'pinia'

export interface ReindexRequestContext {
  selection: SelectionDescriptor
  timestamp: number
  isolationId: IsolationId
  targetCount: number
}

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
      reindexContext: ReindexRequestContext | null
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
      reindexContext: null
    }),
    actions: {
      openReindex(context: ReindexRequestContext) {
        const selection: SelectionDescriptor =
          context.selection.mode === 'explicit'
            ? { mode: 'explicit', indices: [...context.selection.indices] }
            : { mode: 'allExcept', excludedIndices: [...context.selection.excludedIndices] }
        this.reindexContext = { ...context, selection }
      },
      closeReindex() {
        this.reindexContext = null
      }
    }
  })()
