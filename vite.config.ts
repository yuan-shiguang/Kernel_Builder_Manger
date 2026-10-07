import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

// Tauri 在 Linux 下以 http://tauri.localhost 提供服务，dev 端口固定 1420。
const host = process.env.TAURI_DEV_HOST

export default defineConfig({
  plugins: [vue()],

  // 让 vite 可以编译 winuionweb 目录以及访问项目外的资源
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
      '@winui': fileURLToPath(new URL('./src/winuionweb', import.meta.url)),
    },
  },

  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: {
      ignored: ['**/src-tauri/**', '**/WinUIonWeb/**'],
    },
  },
  build: {
    target: process.env.TAURI_ENV_PLATFORM === 'windows' ? 'chrome105' : 'safari13',
    minify: !process.env.TAURI_ENV_DEBUG ? 'esbuild' : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    outDir: 'dist',
    chunkSizeWarningLimit: 3000,
  },
})
