import router from '@/route/routes'
import { IsolationId } from '@type/types'
import { defineStore } from 'pinia'

export const useRedirectionStore = (isolationId: IsolationId) =>
  defineStore('redirectionStore' + isolationId, {
    state: (): {
      redirection: null | string
    } => ({
      redirection: null
    }),
    actions: {
      async redirectionToLogin() {
        if (router.currentRoute.value.name !== 'login') {
          this.redirection = router.currentRoute.value.fullPath
          await router.push({ name: 'login' })
        }
      }
    }
  })()
