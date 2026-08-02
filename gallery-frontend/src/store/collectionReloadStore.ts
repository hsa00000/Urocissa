import type { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

/**
 * Invalidates collection snapshots without remounting their route hosts.
 * Keeping this separate from render keys prevents data refreshes from
 * destroying higher-priority nested overlays.
 */
export const useCollectionReloadStore = (isolationId: IsolationId) =>
  defineStore('collectionReloadStore' + isolationId, {
    state: (): {
      mainCollectionReload: number
      subCollectionReload: number
    } => ({
      mainCollectionReload: 0,
      subCollectionReload: 0
    }),
    actions: {
      requestMainCollectionReload() {
        this.mainCollectionReload += 1
      },
      requestSubCollectionReload() {
        this.subCollectionReload += 1
      }
    }
  })()
