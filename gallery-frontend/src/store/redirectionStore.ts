import router from '@/route/routes'
import { defineStore } from 'pinia'

export const useRedirectionStore = defineStore('redirectionStore', {
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
})
