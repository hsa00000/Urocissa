/**
 * These settings are used exclusively for personal debugging purposes
 * or UI preferences (e.g., isMobile, showFilenameChip).
 * They are referred to as "Settings" in the Frontend UI.
 */

import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

// Refactor: Renamed usesettingsStore to useSettingsStore
// Store ID changed to 'settingsStore'
export const useSettingsStore = (isolationId: IsolationId) =>
  defineStore('settingsStore' + isolationId, {
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
