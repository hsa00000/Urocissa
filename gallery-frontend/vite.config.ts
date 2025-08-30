// vite.config.ts

import { resolve } from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { VitePWA } from 'vite-plugin-pwa'

import vuetify, { transformAssetUrls } from 'vite-plugin-vuetify'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    vue({
      template: { transformAssetUrls }
    }),

    vuetify({
      autoImport: true
    }),

    VitePWA({
      srcDir: 'src/worker', // Specify Service Worker location
      filename: 'serviceWorker.ts', // Service Worker filename
      strategies: 'injectManifest', // Use injectManifest, disable PWA
      /* injectRegister: false, // Do not auto-register Service Worker */
      manifest: false, // Disable Web App Manifest
      injectManifest: {
        injectionPoint: undefined // Do not insert precache manifest
      },
      devOptions: {
        enabled: true, // Enable Service Worker in development mode
        type: 'module' // Set to "module" if SW contains imports
      }
    })
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '@Menu': resolve(__dirname, 'src/components/Menu'),
      '@worker': resolve(__dirname, 'src/worker'),
      '@utils': resolve(__dirname, 'src/script/utils'),
      '@type': resolve(__dirname, 'src/type')
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
      '/json': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/assets': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/put': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/delete': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/edit_album': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/edit_sync_path': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/edit_priority_list': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/import_path': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/upload': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/create_album': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/query': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/get': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/post': { target: 'http://127.0.0.1:5673', changeOrigin: true },
      '/object': { target: 'http://127.0.0.1:5673', changeOrigin: true }
    }
  }
})
