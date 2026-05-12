import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";
import vuetify from "vite-plugin-vuetify";

export default defineConfig({
  plugins: [vue(), vuetify({ autoImport: true })],
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (id.includes("node_modules/vuetify")) {
            return "vuetify";
          }
          if (
            id.includes("node_modules/vue") ||
            id.includes("node_modules/@vue")
          ) {
            return "vue";
          }
          if (id.includes("node_modules/@mdi")) {
            return "mdi";
          }
        }
      }
    }
  },
  server: {
    port: 1420,
    strictPort: true
  }
});
