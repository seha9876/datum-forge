<script setup lang="ts">
import { useConfirmDialog } from "../composables/useConfirmDialog";

import type { AppBootstrap, AppTableSummary } from "../types";

const LABELS = {
  openSidebar: "サイドメニューを開く",
  closeSidebar: "サイドメニューを閉じる",
  createTable: "テーブルを作成",
  workspaceCaption:
    "テーブル設計とデータ編集をモードごとに切り替えて扱えるワークスペースです。",
  tableList: "テーブル一覧",
  createNew: "新規作成",
  emptyHint: "まだテーブルがありません。まずは「新規作成」から始めましょう。",
  tableActions: "テーブル操作",
  deleteTable: "削除"
} as const;

const RAIL_TOGGLE_GLYPH = "☰";

const props = defineProps<{
  bootstrap: AppBootstrap | null;
  rail: boolean;
  selectedTableId: number | null;
  onDeleteTable: (tableId: number) => Promise<void>;
  onLoadTable: (tableId: number) => Promise<void>;
  onOpenCreateDialog: () => void;
  onToggleSidebar: () => void;
}>();

const confirmDialog = useConfirmDialog();

function tableInitial(name: string) {
  return name.slice(0, 1).toUpperCase();
}

async function handleDeleteTable(table: AppTableSummary) {
  const confirmed = await confirmDialog.open({
    title: "テーブルの削除",
    message: `テーブル「${table.displayName} (${table.tableName})」を削除します。テーブル内のレコード、カラム、閲覧ナビ配置、レイアウト差分も削除され、元に戻せません。削除しますか？`,
    confirmText: "削除",
    color: "error"
  });
  if (!confirmed) {
    return;
  }

  try {
    await props.onDeleteTable(table.id);
  } catch {
    // 詳細なエラーはアプリ上部のエラー表示に委ねます。
  }
}
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

          <div class="sidebar-rail-logo" aria-hidden="true">DF</div>

          <v-tooltip :text="LABELS.createTable" location="right">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-table-plus"
                variant="tonal"
                color="primary"
                :aria-label="LABELS.createTable"
                @click="onOpenCreateDialog"
              />
            </template>
          </v-tooltip>
        </div>

        <div class="sidebar-scroll-section rail">
          <v-list
            class="sidebar-list sidebar-list-rail"
            nav
            density="comfortable"
          >
            <v-tooltip
              v-for="table in props.bootstrap?.tables ?? []"
              :key="table.id"
              :text="`${table.displayName} (${table.tableName})`"
              location="right"
            >
              <template #activator="{ props: tooltipProps }">
                <v-list-item
                  v-bind="tooltipProps"
                  :active="table.id === selectedTableId"
                  rounded="xl"
                  class="sidebar-table-item rail"
                  @click="onLoadTable(table.id)"
                >
                  <template #prepend>
                    <v-avatar size="36" color="primary" variant="tonal">
                      {{ tableInitial(table.displayName) }}
                    </v-avatar>
                  </template>
                </v-list-item>
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
              <h1 class="sidebar-app-title">Datum Forge</h1>
              <p class="sidebar-app-caption">
                {{ LABELS.workspaceCaption }}
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
            <span class="sidebar-section-title">{{ LABELS.tableList }}</span>
            <v-chip
              size="small"
              color="primary"
              variant="tonal"
              class="sidebar-count-chip"
            >
              {{ props.bootstrap?.tables.length ?? 0 }}
            </v-chip>
          </div>

          <div class="sidebar-list-actions">
            <v-btn
              prepend-icon="mdi-table-plus"
              variant="tonal"
              color="primary"
              block
              class="sidebar-create-btn"
              @click="onOpenCreateDialog"
            >
              {{ LABELS.createNew }}
            </v-btn>
          </div>

          <v-list class="sidebar-list" nav density="comfortable" lines="two">
            <v-list-item
              v-for="table in props.bootstrap?.tables ?? []"
              :key="table.id"
              :active="table.id === selectedTableId"
              rounded="xl"
              class="sidebar-table-item"
              @click="onLoadTable(table.id)"
            >
              <template #prepend>
                <v-avatar size="36" color="primary" variant="tonal">
                  {{ tableInitial(table.displayName) }}
                </v-avatar>
              </template>
              <v-list-item-title class="sidebar-table-title">
                {{ table.displayName }}
              </v-list-item-title>
              <v-list-item-subtitle class="sidebar-table-subtitle">
                {{ table.tableName }}
              </v-list-item-subtitle>
              <template #append>
                <v-menu location="bottom end">
                  <template #activator="{ props: menuProps }">
                    <v-tooltip :text="LABELS.tableActions" location="bottom">
                      <template #activator="{ props: tooltipProps }">
                        <v-btn
                          v-bind="{ ...menuProps, ...tooltipProps }"
                          icon="mdi-dots-vertical"
                          variant="text"
                          density="comfortable"
                          :aria-label="LABELS.tableActions"
                          @click.stop
                        />
                      </template>
                    </v-tooltip>
                  </template>
                  <v-list density="compact">
                    <v-list-item
                      base-color="error"
                      prepend-icon="mdi-delete-outline"
                      @click.stop="handleDeleteTable(table)"
                    >
                      <v-list-item-title>
                        {{ LABELS.deleteTable }}
                      </v-list-item-title>
                    </v-list-item>
                  </v-list>
                </v-menu>
              </template>
            </v-list-item>
          </v-list>

          <p
            v-if="(props.bootstrap?.tables.length ?? 0) === 0"
            class="sidebar-empty-hint"
          >
            {{ LABELS.emptyHint }}
          </p>
        </v-card>
      </div>
    </template>
  </div>
</template>
