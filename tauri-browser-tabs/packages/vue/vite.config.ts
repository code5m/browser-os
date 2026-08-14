import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import { resolve } from 'path';

export default defineConfig({
  plugins: [vue()],
  build: {
    lib: {
      entry: resolve(__dirname, 'src/index.ts'),
      name: 'TauriBrowserTabsVue',
      fileName: (format) => `index.${format === 'es' ? 'js' : format}`,
      formats: ['es'],
    },
    rollupOptions: {
      external: ['vue', '@tauri-apps/api', '@tauri-browser-tabs/core'],
      output: {
        globals: {
          vue: 'Vue',
          '@tauri-apps/api': 'TauriApi',
          '@tauri-browser-tabs/core': 'TauriBrowserTabsCore',
        },
      },
    },
  },
});
