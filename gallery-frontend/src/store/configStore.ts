/**
 * These settings are used exclusively for personal debugging purposes.
 * They are not intended for end users or other developers.
 */

import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useConfigStore = (isolationId: IsolationId) =>
  defineStore('configStore' + isolationId, {
    state: (): {
      disableImg: boolean
      isMobile: boolean
      showFilenameChip: boolean
      viewBarOverlay: boolean
    } => ({
      disableImg: false,
      isMobile: false,
      showFilenameChip: false,
      // 新增：控制 ViewBar 是否覆蓋圖片，預設 true 保持原本行為
      viewBarOverlay: true
    }),
    actions: {
      toggleViewBarOverlay() {
        // @ts-ignore
        this.viewBarOverlay = !this.viewBarOverlay
      }
    }
  })()
