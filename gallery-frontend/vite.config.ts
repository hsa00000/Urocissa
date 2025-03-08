import { resolve } from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { VitePWA } from 'vite-plugin-pwa'
// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    vue(),
    VitePWA({
      srcDir: 'src', // 指定 Service Worker 位置
      filename: 'worker/serviceWorker.ts', // Service Worker 檔案名稱
      strategies: 'injectManifest', // 使用 injectManifest，不啟用 PWA
      /* injectRegister: false, // 不自動註冊 Service Worker */
      manifest: false, // 不啟用 Web App Manifest
      injectManifest: {
        injectionPoint: undefined // 不插入預快取清單
      },
      devOptions: {
        enabled: true, // 在開發模式啟用 Service Worker
        type: 'module' // 如果你的 SW 內有 import，要設為 "module"
      }
    })
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
      '@Menu': resolve(__dirname, 'src/components/Menu'),
      '@worker': resolve(__dirname, 'src/worker'),
      '@utils': resolve(__dirname, 'src/script/utils'),
      '@constants': resolve(__dirname, 'src/script/common/constants')
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
  },
  css: {
    preprocessorOptions: {
      scss: {
        api: 'modern'
      }
    }
  }
})
