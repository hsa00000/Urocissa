import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { VitePWA } from 'vite-plugin-pwa'
import vueDevTools from 'vite-plugin-vue-devtools'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    vueDevTools(),
    VitePWA({
      srcDir: 'src/worker', // Specify Service Worker directory
      filename: 'serviceWorker.ts', // Service Worker filename
      strategies: 'injectManifest', // Use injectManifest; do not enable full PWA
      injectRegister: 'script', // Auto-inject a registration script (no manual code needed)
      manifest: false, // Disable Web App Manifest
      injectManifest: {
        injectionPoint: undefined // Do not inject a precache manifest
      },
      devOptions: {
        enabled: true, // Enable Service Worker in development mode
        type: 'module' // Set to "module" if your SW uses imports
      }
    })
  ],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@Menu': fileURLToPath(new URL('./src/components/Menu', import.meta.url)),
      '@worker': fileURLToPath(new URL('./src/worker', import.meta.url)),
      '@utils': fileURLToPath(new URL('./src/script/utils', import.meta.url)),
      '@type': fileURLToPath(new URL('./src/type', import.meta.url))
    }
  },
  build: {
    rollupOptions: {
      input: {
        app: './index.html' // Entry point
      }
    },
    chunkSizeWarningLimit: 1000 // Increase warning limit to 1MB if warnings are acceptable
  },
  server: {
    proxy: {
      '/json': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/assets': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/put': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/delete': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/edit_album': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/edit_sync_path': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/edit_priority_list': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/import_path': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/upload': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/create_album': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/query': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/get': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/post': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      },
      '/object': {
        target: 'http://127.0.0.1:5673',
        changeOrigin: true
      }
    }
  }
})
