// This file initializes the Vue 3 application, sets up the router, state management (Pinia), and Vuetify UI framework.
// It is referenced in index.html to bootstrap the app, configures the application with a dark theme, and mounts it to the DOM.

// Importing core dependencies and main component
import { createApp } from 'vue'
import App from '@/components/App.vue'

// Importing router and state management
import router from '@/route/routes'
import { createPinia } from 'pinia'

// Importing global styles and icons
import '@/style/common.scss'
import '@mdi/font/css/materialdesignicons.css'

// Importing Vuetify UI framework and configuration
import { createVuetify } from 'vuetify'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
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

    // Add password header if it exists
    if (shareStore.password) {
      config.headers.set('x-share-password', shareStore.password)
    }
  }

  return config
})

// Response interceptor
axios.interceptors.response.use(
  (response) => response,
  async (error: AxiosError) => {
    if (error.response?.status === 401) {
      const shareStore = useShareStore('mainId')

      // Check if this is a share context (albumId/shareId are present)
      // This prevents redirecting to admin login when trying to access a locked share
      if (shareStore.albumId && shareStore.shareId) {
        shareStore.isAuthFailed = true
        return Promise.reject(error)
      }

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
  components,
  directives,
  theme: {
    // 'light' | 'dark' | 'system'
    defaultTheme: constStore.theme === 'light' ? 'light' : 'dark',
    themes: {}
  }
})

// Apply necessary plugins and mount the app
app.use(router)
app.use(vuetify)
app.mount('#app')
