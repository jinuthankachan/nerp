import { defineConfig } from 'vite'

// Build-only asset pipeline for the loco (server-rendered) app.
// Vite compiles frontend/main.{js,scss} into assets/static/dist/, which loco's
// static middleware serves at /static/dist/. There is no Vite dev server —
// loco renders the HTML (and hot-reloads Tera templates in debug).
export default defineConfig({
  // All emitted asset URLs (e.g. font url() in CSS) are prefixed with this.
  base: '/static/dist/',
  build: {
    outDir: 'assets/static/dist',
    emptyOutDir: true,
    manifest: false,
    rollupOptions: {
      input: { app: 'frontend/main.js' },
      // Stable filenames (no content hash): the Tera layout references
      // /static/dist/app.css + /static/dist/app.js directly. loco's ServeDir
      // sends Last-Modified/ETag, so browsers revalidate (no stale assets).
      output: {
        entryFileNames: '[name].js',
        chunkFileNames: '[name].js',
        assetFileNames: '[name][extname]',
      },
    },
  },
})
