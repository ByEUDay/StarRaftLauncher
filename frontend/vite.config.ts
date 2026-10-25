import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// 生产构建由 BackEnd 以静态文件形式提供；开发时运行 Vite，并通过 .env.development 指向后端
export default defineConfig({
  plugins: [react()],
  base: './',
  server: { port: 5173, strictPort: true },
  build: { outDir: 'dist', target: 'es2020', sourcemap: false },
});
