import { resolve } from 'path'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [vue()],
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
