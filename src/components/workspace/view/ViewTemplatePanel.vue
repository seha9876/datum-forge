<script setup lang="ts">
import { computed, ref } from "vue";

import type { ViewLayoutTemplate } from "../../../types";

const props = defineProps<{
  layoutTemplates: ViewLayoutTemplate[];
  onCreateLayoutTemplate: (name: string) => Promise<void>;
  onDeleteLayoutTemplate: (templateId: number) => Promise<void>;
  onDuplicateLayoutTemplate: (
    templateId: number,
    name: string
  ) => Promise<void>;
  onRenameLayoutTemplate: (templateId: number, name: string) => Promise<void>;
  onSelectLayoutTemplate: (template: ViewLayoutTemplate) => void;
  selectedLayoutTemplateId: number | null;
}>();

const isCreateDialogOpen = ref(false);
const isRenameDialogOpen = ref(false);
const isDuplicateDialogOpen = ref(false);
const isDeleteDialogOpen = ref(false);
const busy = ref(false);
const formName = ref("");
const targetTemplate = ref<ViewLayoutTemplate | null>(null);

const sortedTemplates = computed(() =>
  [...props.layoutTemplates].sort((left, right) => {
    if (left.folderId === null && right.folderId !== null) {
      return -1;
    }
    if (left.folderId !== null && right.folderId === null) {
      return 1;
    }
    return left.name.localeCompare(right.name) || left.id - right.id;
  })
);

function scopeLabel(template: ViewLayoutTemplate) {
  return template.folderId === null ? "共有" : "フォルダー専用";
}

function openCreateDialog() {
  formName.value = "";
  isCreateDialogOpen.value = true;
}

function openRenameDialog(template: ViewLayoutTemplate) {
  targetTemplate.value = template;
  formName.value = template.name;
  isRenameDialogOpen.value = true;
}

function openDuplicateDialog(template: ViewLayoutTemplate) {
  targetTemplate.value = template;
  formName.value = `${template.name} コピー`;
  isDuplicateDialogOpen.value = true;
}

function openDeleteDialog(template: ViewLayoutTemplate) {
  targetTemplate.value = template;
  isDeleteDialogOpen.value = true;
}

async function createTemplate() {
  if (formName.value.trim().length === 0) {
    return;
  }
  busy.value = true;
  try {
    await props.onCreateLayoutTemplate(formName.value.trim());
    isCreateDialogOpen.value = false;
  } finally {
    busy.value = false;
  }
}

async function renameTemplate() {
  const template = targetTemplate.value;
  if (!template || formName.value.trim().length === 0) {
    return;
  }
  busy.value = true;
  try {
    await props.onRenameLayoutTemplate(template.id, formName.value.trim());
    isRenameDialogOpen.value = false;
  } finally {
    busy.value = false;
  }
}

async function duplicateTemplate() {
  const template = targetTemplate.value;
  if (!template || formName.value.trim().length === 0) {
    return;
  }
  busy.value = true;
  try {
    await props.onDuplicateLayoutTemplate(template.id, formName.value.trim());
    isDuplicateDialogOpen.value = false;
  } finally {
    busy.value = false;
  }
}

async function deleteTemplate() {
  const template = targetTemplate.value;
  if (!template) {
    return;
  }
  busy.value = true;
  try {
    await props.onDeleteLayoutTemplate(template.id);
    isDeleteDialogOpen.value = false;
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <v-card
    tag="section"
    color="surface"
    variant="elevated"
    rounded="xl"
    elevation="2"
    border
    class="view-sidebar-panel view-template-panel"
  >
    <div class="view-sidebar-panel-actions">
      <v-tooltip text="テンプレートを追加" location="bottom">
        <template #activator="{ props: tooltipProps }">
          <v-btn
            v-bind="tooltipProps"
            icon="mdi-plus"
            color="primary"
            variant="tonal"
            size="small"
            aria-label="テンプレートを追加"
            @click="openCreateDialog"
          />
        </template>
      </v-tooltip>
    </div>

    <v-list class="view-template-list" nav density="compact">
      <v-list-item
        v-for="template in sortedTemplates"
        :key="template.id"
        :active="template.id === selectedLayoutTemplateId"
        rounded="lg"
        class="view-template-list-item"
        @click="onSelectLayoutTemplate(template)"
      >
        <template #prepend>
          <v-icon icon="mdi-view-dashboard-outline" />
        </template>
        <v-list-item-title>{{ template.name }}</v-list-item-title>
        <v-list-item-subtitle>
          {{ scopeLabel(template) }}
        </v-list-item-subtitle>
        <template #append>
          <div class="manual-tree-actions">
            <v-tooltip text="テンプレートを複製" location="bottom">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-content-copy"
                  variant="text"
                  size="x-small"
                  aria-label="テンプレートを複製"
                  @click.stop="openDuplicateDialog(template)"
                />
              </template>
            </v-tooltip>
            <v-tooltip text="テンプレート名を変更" location="bottom">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-pencil"
                  variant="text"
                  size="x-small"
                  aria-label="テンプレート名を変更"
                  @click.stop="openRenameDialog(template)"
                />
              </template>
            </v-tooltip>
            <v-tooltip text="テンプレートを削除" location="bottom">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-delete-outline"
                  variant="text"
                  size="x-small"
                  color="error"
                  aria-label="テンプレートを削除"
                  @click.stop="openDeleteDialog(template)"
                />
              </template>
            </v-tooltip>
          </div>
        </template>
      </v-list-item>
    </v-list>

    <p v-if="sortedTemplates.length === 0" class="view-empty-hint mb-0">
      まだテンプレートがありません。
    </p>
  </v-card>

  <v-dialog v-model="isCreateDialogOpen" max-width="520" :persistent="busy">
    <v-card rounded="xl">
      <v-card-title>テンプレート追加</v-card-title>
      <v-card-text class="d-grid ga-4">
        <v-text-field
          v-model="formName"
          label="テンプレート名"
          variant="outlined"
          density="comfortable"
          :disabled="busy"
        />
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn
          variant="text"
          :disabled="busy"
          @click="isCreateDialogOpen = false"
        >
          閉じる
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          :disabled="formName.trim().length === 0 || busy"
          :loading="busy"
          @click="createTemplate"
        >
          作成
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

  <v-dialog v-model="isRenameDialogOpen" max-width="480" :persistent="busy">
    <v-card rounded="xl">
      <v-card-title>テンプレート名を変更</v-card-title>
      <v-card-text>
        <v-text-field
          v-model="formName"
          label="テンプレート名"
          variant="outlined"
          density="comfortable"
          :disabled="busy"
        />
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn
          variant="text"
          :disabled="busy"
          @click="isRenameDialogOpen = false"
        >
          閉じる
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          :disabled="formName.trim().length === 0 || busy"
          :loading="busy"
          @click="renameTemplate"
        >
          保存
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

  <v-dialog v-model="isDuplicateDialogOpen" max-width="480" :persistent="busy">
    <v-card rounded="xl">
      <v-card-title>テンプレートを複製</v-card-title>
      <v-card-text>
        <v-text-field
          v-model="formName"
          label="新しいテンプレート名"
          variant="outlined"
          density="comfortable"
          :disabled="busy"
        />
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn
          variant="text"
          :disabled="busy"
          @click="isDuplicateDialogOpen = false"
        >
          閉じる
        </v-btn>
        <v-btn
          color="primary"
          variant="flat"
          :disabled="formName.trim().length === 0 || busy"
          :loading="busy"
          @click="duplicateTemplate"
        >
          複製
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

  <v-dialog v-model="isDeleteDialogOpen" max-width="460" :persistent="busy">
    <v-card rounded="xl">
      <v-card-title>テンプレートを削除</v-card-title>
      <v-card-text>
        <p class="mb-0">
          「{{
            targetTemplate?.name
          }}」を削除します。割当中のフォルダーは未選択状態に戻ります。
        </p>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn
          variant="text"
          :disabled="busy"
          @click="isDeleteDialogOpen = false"
        >
          キャンセル
        </v-btn>
        <v-btn
          color="error"
          variant="flat"
          :loading="busy"
          @click="deleteTemplate"
        >
          削除
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>
