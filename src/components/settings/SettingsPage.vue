<script setup lang="ts">
import { ref } from "vue";

import DatabaseSettingsPanel from "./DatabaseSettingsPanel.vue";
import DisplaySettingsPanel from "./DisplaySettingsPanel.vue";
import SettingsCategoryNav from "./SettingsCategoryNav.vue";

type SettingsCategoryId = "database" | "display";

interface SettingsCategory {
  id: SettingsCategoryId;
  label: string;
  icon: string;
}

const categories: SettingsCategory[] = [
  {
    id: "database",
    label: "データベース",
    icon: "mdi-database-cog-outline"
  },
  {
    id: "display",
    label: "表示",
    icon: "mdi-eye-settings-outline"
  }
];

const selectedCategory = ref<SettingsCategoryId>("database");

defineProps<{
  onBack: () => void;
}>();
</script>

<template>
  <!-- 設定画面全体のレイアウトです。左にカテゴリ、右に設定内容を表示します。 -->
  <v-layout class="settings-page-layout">
    <v-navigation-drawer
      permanent
      location="start"
      width="280"
      color="surface"
      class="settings-category-drawer"
    >
      <SettingsCategoryNav
        v-model:selected-category="selectedCategory"
        :categories="categories"
      />
    </v-navigation-drawer>

    <v-main class="settings-page-main">
      <div class="settings-page-scroll">
        <div class="settings-page-toolbar">
          <v-btn prepend-icon="mdi-arrow-left" variant="text" @click="onBack">
            戻る
          </v-btn>
        </div>

        <DatabaseSettingsPanel v-if="selectedCategory === 'database'" />
        <DisplaySettingsPanel v-else-if="selectedCategory === 'display'" />
      </div>
    </v-main>
  </v-layout>
</template>
