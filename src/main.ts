import { createPinia } from "pinia";
import { createApp } from "vue";

import App from "./App.vue";
import { vuetify } from "./plugins/vuetify";
import "./style.css";

// Vue アプリケーションを生成し、プラグインを注入してから画面に描画します。
const app = createApp(App);

app.use(createPinia());
app.use(vuetify);
app.mount("#app");
