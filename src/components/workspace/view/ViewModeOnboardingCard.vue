<script setup lang="ts">
withDefaults(
  defineProps<{
    folderActionVariant?: "flat" | "tonal" | "text";
    folderCount: number;
    isCreatingFolder?: boolean;
    isCreatingTemplate?: boolean;
    showActions?: boolean;
    showGuideAction?: boolean;
    tableCount: number;
    title: string;
    description: string;
  }>(),
  {
    folderActionVariant: "flat",
    isCreatingFolder: false,
    isCreatingTemplate: false,
    showActions: true,
    showGuideAction: false
  }
);

const emit = defineEmits<{
  "create-folder": [];
  "create-template": [];
  "open-guide": [];
}>();

const onboardingFeatures = [
  {
    icon: "mdi-cards-outline",
    title: "カードを自由配置",
    text: "必要な情報をカード枠に置いて、資料ごとの見せ方を整えられます。"
  },
  {
    icon: "mdi-folder-multiple-outline",
    title: "フォルダーで資料整理",
    text: "左側の目次にフォルダーを作り、既存レコードをまとめて開けます。"
  },
  {
    icon: "mdi-content-copy",
    title: "テンプレートで表示を再利用",
    text: "フォルダー用テンプレートを使い、同じレイアウトを繰り返し使えます。"
  }
];
</script>

<template>
  <div class="view-empty-onboarding">
    <v-icon
      icon="mdi-folder-star-outline"
      color="primary"
      size="40"
      class="view-empty-onboarding-icon"
    />
    <div class="view-empty-onboarding-copy">
      <h3 class="text-on-surface">{{ title }}</h3>
      <p class="text-on-surface-variant">{{ description }}</p>
    </div>
    <div class="view-empty-summary">
      <v-chip size="small" color="primary" variant="tonal">
        テーブル {{ tableCount }}
      </v-chip>
      <v-chip size="small" variant="tonal">
        フォルダー {{ folderCount }}
      </v-chip>
    </div>
    <div class="view-empty-feature-grid">
      <v-card
        v-for="feature in onboardingFeatures"
        :key="feature.title"
        variant="outlined"
        rounded="lg"
        class="view-empty-feature"
      >
        <v-icon :icon="feature.icon" color="primary" size="24" />
        <div>
          <strong class="text-on-surface">{{ feature.title }}</strong>
          <p class="text-on-surface-variant">{{ feature.text }}</p>
        </div>
      </v-card>
    </div>
    <div v-if="showActions || showGuideAction" class="view-empty-actions">
      <v-btn
        v-if="showActions"
        prepend-icon="mdi-folder-plus-outline"
        color="primary"
        :variant="folderActionVariant"
        :loading="isCreatingFolder"
        @click="emit('create-folder')"
      >
        フォルダー追加
      </v-btn>
      <v-btn
        v-if="showActions"
        prepend-icon="mdi-plus"
        color="primary"
        variant="text"
        :loading="isCreatingTemplate"
        @click="emit('create-template')"
      >
        テンプレート追加
      </v-btn>
      <v-btn
        v-if="showGuideAction"
        prepend-icon="mdi-book-open-page-variant-outline"
        color="primary"
        variant="text"
        @click="emit('open-guide')"
      >
        閲覧モードの使い方を見る
      </v-btn>
    </div>
  </div>
</template>
