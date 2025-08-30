// main.ts

import { createApp } from 'vue'
import App from '@/components/App.vue'

import router from '@/route/routes'
import { createPinia } from 'pinia'

import '@/style/common.scss'
import '@mdi/font/css/materialdesignicons.css'

import 'vuetify/styles'

import { createVuetify } from 'vuetify'

import axios, { AxiosError, InternalAxiosRequestConfig } from 'axios'
import { useRedirectionStore } from '@/store/redirectionStore'
import { useShareStore } from '@/store/shareStore'
import { useConstStore } from '@/store/constStore'

// Request interceptor
axios.interceptors.request.use((config: InternalAxiosRequestConfig) => {
  const shareStore = useShareStore('mainId')

  if (typeof shareStore.albumId === 'string' && typeof shareStore.shareId === 'string') {
    config.headers.set('x-album-id', shareStore.albumId)
    config.headers.set('x-share-id', shareStore.shareId)
  }

  return config
})

// Response interceptor
axios.interceptors.response.use(
  (response) => response,
  async (error: AxiosError) => {
    if (error.response && error.response.status === 401) {
      const redirectionStore = useRedirectionStore('mainId')
      await redirectionStore.redirectionToLogin()
    }
    return Promise.reject(error)
  }
)
// Create Vue application instance
const app = createApp(App)

// Setup state management (Pinia) early so stores can be used outside components
const pinia = createPinia()
app.use(pinia)

// Ensure const store is available and load theme preference before creating Vuetify
const constStore = useConstStore('mainId')
await constStore.loadTheme()

// Configure Vuetify and set default theme (use Vuetify's built-in theme palettes)
const vuetify = createVuetify({
  theme: {
    // 'light' | 'dark' | 'system'
    defaultTheme: constStore.theme === 'light' ? 'light' : 'dark'
  }
})

// Apply necessary plugins and mount the app
app.use(router)
app.use(vuetify)
app.mount('#app')
