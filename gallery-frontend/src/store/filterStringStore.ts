/**
 * These settings are used exclusively for personal debugging purposes.
 * They are not intended for end users or other developers.
 */

import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useFilterStringStore = (isolationId: IsolationId) =>
  defineStore('filterStringStore' + isolationId, {
    state: (): {
      filterString: string | null
    } => ({
      filterString: null
    }),
    actions: {}
  })()
