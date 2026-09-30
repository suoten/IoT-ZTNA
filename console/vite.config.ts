import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  server: {
    port: 3000,
    proxy: {
      '/api': {
        target: 'http://localhost:8443',
        changeOrigin: true,
      },
      '/ws': {
        target: 'ws://localhost:8443',
        ws: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    chunkSizeWarningLimit: 1500, // Naive UI 全量打包约 1.4MB (gzip: 380KB)，可接受
    rollupOptions: {
      output: {
        manualChunks: {
          // 将大型依赖拆分为独立 chunk
          'vendor-vue': ['vue', 'vue-router', 'pinia'],
          'vendor-naive-ui': ['naive-ui'],
          'vendor-echarts': ['echarts', 'vue-echarts'],
          'vendor-axios': ['axios'],
        },
      },
    },
  },
})
