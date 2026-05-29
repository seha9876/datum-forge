<script setup lang="ts">
type MasterSection = "options" | "tags";

interface MasterSectionItem {
  id: MasterSection;
  label: string;
  subtitle: string;
  icon: string;
}

const LABELS = {
  title: "マスタ管理",
  sectionList: "管理項目"
} as const;

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
}>();

const emit = defineEmits<{
  "update:selectedSection": [MasterSection];
}>();
</script>

<template>
  <div class="sidebar-content" :class="{ rail }">
    <template v-if="rail">
      <div class="sidebar-rail-shell no-actions">
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
            <div class="sidebar-list-head-actions">
              <v-chip
                size="small"
                color="primary"
                variant="tonal"
                class="sidebar-count-chip"
              >
                {{ sections.length }}
              </v-chip>
            </div>
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
