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
    } => ({
      disableImg: false,
      isMobile: false,
      showFilenameChip: false
    })
  })()
