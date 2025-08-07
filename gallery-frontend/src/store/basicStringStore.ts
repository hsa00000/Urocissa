/**
 * These settings are used exclusively for personal debugging purposes.
 * They are not intended for end users or other developers.
 */

import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useBasicStringStore = (isolationId: IsolationId) =>
  defineStore('basicStringStore' + isolationId, {
    state: (): {
      basicString: string | null
    } => ({
      basicString: null
    }),
    actions: {}
  })()
