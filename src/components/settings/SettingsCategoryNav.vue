<script setup lang="ts">
type SettingsCategoryId = "database" | "display";

interface SettingsCategory {
  id: SettingsCategoryId;
  label: string;
  icon: string;
}

defineProps<{
  categories: SettingsCategory[];
  selectedCategory: SettingsCategoryId;
}>();

const emit = defineEmits<{
  "update:selectedCategory": [SettingsCategoryId];
}>();
</script>

<template>
  <div class="settings-nav">
    <div class="settings-nav-heading">
      <strong>設定</strong>
      <small>カテゴリ</small>
    </div>

    <v-list class="settings-nav-list" nav density="comfortable">
      <v-list-item
        v-for="category in categories"
        :key="category.id"
        :active="selectedCategory === category.id"
        :prepend-icon="category.icon"
        rounded="lg"
        @click="emit('update:selectedCategory', category.id)"
      >
        <v-list-item-title>{{ category.label }}</v-list-item-title>
      </v-list-item>
    </v-list>
  </div>
</template>
