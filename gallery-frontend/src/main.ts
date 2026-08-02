// This file initializes the Vue 3 application, sets up the router, state management (Pinia), and Vuetify UI framework.
// It is referenced in index.html to bootstrap the app, configures the application with a dark theme, and mounts it to the DOM.

// Importing core dependencies and main component
import { createApp } from 'vue'
import App from '@/components/App.vue'

// Importing router and state management
import router, { prepareInitialNestedHistory } from '@/route/routes'
import { createPinia } from 'pinia'

// Importing global styles and icons
import '@/style/common.scss'
import '@mdi/font/css/materialdesignicons.css'

// Importing Vuetify UI framework and configuration
import { createVuetify } from 'vuetify'

import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import { useConstStore } from '@/store/constStore'
import { setupMainAxiosInterceptor } from '@/axiosInterceptor'

setupMainAxiosInterceptor()

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
  defaults: {
    global: {
      prependGap: 8
    }
  },
  theme: {
    // 'light' | 'dark' | 'system'
    defaultTheme: constStore.theme === 'light' ? 'light' : 'dark'
  }
})

// Apply necessary plugins and mount the app
app.use(router)
app.use(vuetify)
await prepareInitialNestedHistory()
app.mount('#app')
