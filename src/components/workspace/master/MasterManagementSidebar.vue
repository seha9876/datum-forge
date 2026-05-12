<script setup lang="ts">
type MasterSection = "options" | "tags";

interface MasterSectionItem {
  id: MasterSection;
  label: string;
  subtitle: string;
  icon: string;
}

const LABELS = {
  openSidebar: "サイドメニューを開く",
  closeSidebar: "サイドメニューを閉じる",
  title: "マスタ管理",
  caption: "共通マスタをカテゴリごとに管理します。",
  sectionList: "管理項目"
} as const;

const RAIL_TOGGLE_GLYPH = "◀";

const sections: MasterSectionItem[] = [
  {
    id: "options",
    label: "選択肢",
    subtitle: "単一選択グループ",
    icon: "mdi-format-list-bulleted-square"
  },
  {
    id: "tags",
    label: "タグ",
    subtitle: "レコードタグ",
    icon: "mdi-tag-multiple-outline"
  }
];

defineProps<{
  rail: boolean;
  selectedSection: MasterSection;
  onToggleSidebar: () => void;
}>();

const emit = defineEmits<{
  "update:selectedSection": [MasterSection];
}>();
</script>

<template>
  <div class="sidebar-content" :class="{ rail }">
    <template v-if="rail">
      <div class="sidebar-rail-shell">
        <div class="sidebar-rail-actions">
          <v-tooltip :text="LABELS.openSidebar" location="right">
            <template #activator="{ props: tooltipProps }">
              <button
                v-bind="tooltipProps"
                type="button"
                class="sidebar-rail-toggle"
                :aria-label="LABELS.openSidebar"
                @click="onToggleSidebar"
              >
                <span class="sidebar-rail-toggle-glyph" aria-hidden="true">
                  {{ RAIL_TOGGLE_GLYPH }}
                </span>
              </button>
            </template>
          </v-tooltip>

          <div class="sidebar-rail-logo" aria-hidden="true">M</div>
        </div>

        <div class="sidebar-scroll-section rail">
          <v-list
            class="sidebar-list sidebar-list-rail"
            nav
            density="comfortable"
          >
            <v-tooltip
              v-for="section in sections"
              :key="section.id"
              :text="section.label"
              location="right"
            >
              <template #activator="{ props: tooltipProps }">
                <v-list-item
                  v-bind="tooltipProps"
                  :active="selectedSection === section.id"
                  :prepend-icon="section.icon"
                  rounded="xl"
                  class="sidebar-table-item rail"
                  @click="emit('update:selectedSection', section.id)"
                />
              </template>
            </v-tooltip>
          </v-list>
        </div>
      </div>
    </template>

    <template v-else>
      <div class="sidebar-fixed-section">
        <v-card
          class="sidebar-card sidebar-brand-card"
          color="surface"
          variant="elevated"
          border
          rounded="xl"
          elevation="2"
        >
          <div class="sidebar-brand-header">
            <div class="sidebar-brand-copy">
              <h1 class="sidebar-app-title">{{ LABELS.title }}</h1>
              <p class="sidebar-app-caption">
                {{ LABELS.caption }}
              </p>
            </div>
            <v-tooltip :text="LABELS.closeSidebar" location="bottom">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-chevron-left"
                  variant="text"
                  size="small"
                  class="sidebar-collapse-btn"
                  :aria-label="LABELS.closeSidebar"
                  @click="onToggleSidebar"
                />
              </template>
            </v-tooltip>
          </div>
        </v-card>
      </div>

      <div class="sidebar-scroll-section">
        <v-card
          class="sidebar-card sidebar-list-card"
          color="surface"
          variant="elevated"
          border
          rounded="xl"
          elevation="2"
        >
          <div class="sidebar-list-head">
            <span class="sidebar-section-title">{{ LABELS.sectionList }}</span>
            <v-chip
              size="small"
              color="primary"
              variant="tonal"
              class="sidebar-count-chip"
            >
              {{ sections.length }}
            </v-chip>
          </div>

          <v-list class="sidebar-list" nav density="comfortable" lines="two">
            <v-list-item
              v-for="section in sections"
              :key="section.id"
              :active="selectedSection === section.id"
              :prepend-icon="section.icon"
              :subtitle="section.subtitle"
              :title="section.label"
              rounded="xl"
              class="sidebar-table-item"
              @click="emit('update:selectedSection', section.id)"
            />
          </v-list>
        </v-card>
      </div>
    </template>
  </div>
</template>
