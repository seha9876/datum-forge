import "vuetify/styles";
import "@mdi/font/css/materialdesignicons.css";

import { createVuetify } from "vuetify";
import { aliases, mdi } from "vuetify/iconsets/mdi";

/** アプリ全体で共有する Vuetify のアイコン・テーマ設定です。 */
export const vuetify = createVuetify({
  icons: {
    defaultSet: "mdi",
    aliases,
    sets: {
      mdi
    }
  },
  theme: {
    defaultTheme: "datumForge",
    themes: {
      datumForge: {
        dark: false,
        colors: {
          background: "#f7f1e6",
          "on-background": "#2f261d",
          surface: "#fffaf4",
          "on-surface": "#2f261d",
          "surface-variant": "#eadfcf",
          "on-surface-variant": "#756454",
          outline: "#756454",
          "outline-variant": "#d8d3cc",
          primary: "#b2552d",
          "primary-container": "#fff1eb",
          "on-primary-container": "#9a3412",
          secondary: "#7b4b2f",
          error: "#b91c1c",
          "error-container": "#fff1eb",
          "on-error-container": "#9a3412",
          shadow: "#51321b"
        }
      }
    }
  }
});
