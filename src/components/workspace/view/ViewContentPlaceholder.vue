<script setup lang="ts">
import { computed, ref, watch } from "vue";

import ViewModeOnboardingCard from "./ViewModeOnboardingCard.vue";

import type {
  ViewLayoutTemplate,
  ViewNavFolderRecord,
  ViewSelection,
  ViewTableRecordSummary,
  ViewTableSection
} from "../../../types";

const props = defineProps<{
  folderCount: number;
  selectedFolderActiveTemplateId: number | null;
  selectedFolderLayoutTemplates: ViewLayoutTemplate[];
  selectedFolderRecords: ViewNavFolderRecord[];
  selectedItem: ViewSelection | null;
  tableCount: number;
  tableSections: ViewTableSection[];
  onAddFolderRecords: (
    folderId: number,
    section: ViewTableSection,
    records: ViewTableRecordSummary[]
  ) => Promise<void>;
  onAssignFolderLayoutTemplate: (
    folderId: number,
    templateId: number
  ) => Promise<void>;
  onCreateFolder: (parentId: number | null, name: string) => Promise<void>;
  onCreateFolderLayoutTemplate: (
    folderId: number | null,
    name: string
  ) => Promise<void>;
  onOpenModeHelp: () => void;
  onRemoveFolderRecord: (record: ViewNavFolderRecord) => Promise<void>;
}>();

const isAddDialogOpen = ref(false);
const selectedTableId = ref<number | null>(null);
const recordSearchQuery = ref("");
const selectedRecordIds = ref<number[]>([]);
const isAddingRecords = ref(false);
const isCreatingFolder = ref(false);
const isCreatingTemplate = ref(false);
const isAssigningTemplate = ref(false);
const isFolderDialogOpen = ref(false);
const isTemplateDialogOpen = ref(false);
const folderFormName = ref("");
const templateFormName = ref("");

const selectedTableSection = computed(
  () =>
    props.tableSections.find(
      (section) => section.tableId === selectedTableId.value
    ) ??
    props.tableSections[0] ??
    null
);

const tableSelectItems = computed(() =>
  props.tableSections.map((section) => ({
    title: section.displayName,
    value: section.tableId
  }))
);

const folderTemplateItems = computed(() =>
  props.selectedFolderLayoutTemplates.map((template) => ({
    title: template.name,
    value: template.id
  }))
);
const canManageFolderTemplates = computed(
  () => props.selectedItem?.type === "folder"
);
const hasNoFolders = computed(() => props.folderCount === 0);
const emptyStateTitle = computed(() =>
  hasNoFolders.value
    ? "まだ閲覧フォルダーがありません"
    : "目次から閲覧フォルダーを選択してください"
);
const emptyStateDescription = computed(() =>
  hasNoFolders.value
    ? "左側の目次からフォルダーを作成・選択すると、カードを自由配置して資料を整理できます。"
    : "左側の目次でフォルダーを選ぶと、登録済みレコードやフォルダー用テンプレートを確認できます。"
);

const filteredRecords = computed(() => {
  const section = selectedTableSection.value;
  if (!section) {
    return [];
  }

  const query = recordSearchQuery.value.trim().toLowerCase();
  if (!query) {
    return section.records;
  }

  return section.records.filter((record) =>
    record.label.toLowerCase().includes(query)
  );
});
const selectableFilteredRecords = computed(() => {
  const section = selectedTableSection.value;
  if (!section) {
    return [];
  }

  return filteredRecords.value.filter(
    (record) => !isRecordAdded(section, record)
  );
});
const selectedRecords = computed(() => {
  const section = selectedTableSection.value;
  if (!section) {
    return [];
  }

  const selectedIds = new Set(selectedRecordIds.value);
  return section.records.filter(
    (record) => selectedIds.has(record.id) && !isRecordAdded(section, record)
  );
});

watch(
  () => props.tableSections,
  (sections) => {
    if (selectedTableId.value === null && sections.length > 0) {
      selectedTableId.value = sections[0].tableId;
    }
  },
  { immediate: true }
);
watch(selectedTableId, () => {
  selectedRecordIds.value = [];
});
watch(isAddDialogOpen, (isOpen) => {
  if (!isOpen) {
    resetAddDialog();
  }
});
watch(isFolderDialogOpen, (isOpen) => {
  if (!isOpen) {
    folderFormName.value = "";
  }
});
watch(isTemplateDialogOpen, (isOpen) => {
  if (!isOpen) {
    templateFormName.value = "";
  }
});

function isRecordAdded(
  section: ViewTableSection,
  record: ViewTableRecordSummary
) {
  return props.selectedFolderRecords.some(
    (item) => item.tableId === section.tableId && item.recordId === record.id
  );
}

function isRecordSelected(record: ViewTableRecordSummary) {
  return selectedRecordIds.value.includes(record.id);
}

function toggleRecordSelection(record: ViewTableRecordSummary) {
  const section = selectedTableSection.value;
  if (!section || isRecordAdded(section, record) || isAddingRecords.value) {
    return;
  }

  selectedRecordIds.value = isRecordSelected(record)
    ? selectedRecordIds.value.filter((recordId) => recordId !== record.id)
    : [...selectedRecordIds.value, record.id];
}

function selectFilteredRecords() {
  if (isAddingRecords.value) {
    return;
  }

  selectedRecordIds.value = Array.from(
    new Set([
      ...selectedRecordIds.value,
      ...selectableFilteredRecords.value.map((record) => record.id)
    ])
  );
}

function clearSelectedRecords() {
  selectedRecordIds.value = [];
}

function resetAddDialog() {
  recordSearchQuery.value = "";
  selectedRecordIds.value = [];
}

async function addSelectedRecordsToSelectedFolder() {
  const selected = props.selectedItem;
  const section = selectedTableSection.value;
  const records = selectedRecords.value;
  if (selected?.type !== "folder" || !section || records.length === 0) {
    return;
  }

  isAddingRecords.value = true;
  try {
    await props.onAddFolderRecords(selected.folderId, section, records);
    isAddDialogOpen.value = false;
  } finally {
    isAddingRecords.value = false;
  }
}

async function createRootFolder() {
  const trimmedName = folderFormName.value.trim();
  if (!trimmedName) {
    return;
  }

  isCreatingFolder.value = true;
  try {
    await props.onCreateFolder(null, trimmedName);
    isFolderDialogOpen.value = false;
  } finally {
    isCreatingFolder.value = false;
  }
}

async function createFolderLayoutTemplate() {
  const selected = props.selectedItem;
  const trimmedName = templateFormName.value.trim();
  if (!trimmedName) {
    return;
  }

  isCreatingTemplate.value = true;
  try {
    await props.onCreateFolderLayoutTemplate(
      selected?.type === "folder" ? selected.folderId : null,
      trimmedName
    );
    isTemplateDialogOpen.value = false;
  } finally {
    isCreatingTemplate.value = false;
  }
}
async function assignFolderLayoutTemplate(templateId: number | null) {
  const selected = props.selectedItem;
  if (
    selected?.type !== "folder" ||
    !templateId ||
    templateId === props.selectedFolderActiveTemplateId
  ) {
    return;
  }

  isAssigningTemplate.value = true;
  try {
    await props.onAssignFolderLayoutTemplate(selected.folderId, templateId);
  } finally {
    isAssigningTemplate.value = false;
  }
}

function displayRecordLabel(label: string) {
  return label.replace(/^\d+:/, "").trim() || label;
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
    class="view-content-panel pa-4"
  >
    <div class="section-heading">
      <div>
        <h2>
          {{
            selectedItem?.type === "folder"
              ? "フォルダー内データ"
              : "閲覧モード"
          }}
        </h2>
        <p class="help-text">
          {{
            selectedItem?.type === "folder"
              ? "このフォルダーに既存レコードを追加し、閲覧用の表示を整理します。"
              : "フォルダーとテンプレートを使って、資料を見やすい自由配置ビューで整理します。"
          }}
        </p>
      </div>
    </div>

    <div v-if="selectedItem?.type === 'folder'" class="view-placeholder-card">
      <div class="folder-content-heading">
        <div>
          <v-chip size="small" color="primary" variant="tonal">
            カスタム目次
          </v-chip>
          <h3>{{ selectedItem.folderName }}</h3>
          <p>登録済みデータ: {{ selectedFolderRecords.length }} 件</p>
        </div>
        <div class="folder-content-actions">
          <v-select
            :disabled="!canManageFolderTemplates || isAssigningTemplate"
            :items="folderTemplateItems"
            :model-value="selectedFolderActiveTemplateId"
            class="folder-template-select"
            density="compact"
            hide-details
            item-title="title"
            item-value="value"
            label="レイアウトテンプレート"
            variant="outlined"
            @update:model-value="assignFolderLayoutTemplate"
          />
          <v-btn
            prepend-icon="mdi-plus"
            color="primary"
            variant="tonal"
            :loading="isCreatingTemplate"
            @click="isTemplateDialogOpen = true"
          >
            テンプレート追加
          </v-btn>
          <v-btn
            prepend-icon="mdi-database-plus-outline"
            color="primary"
            variant="tonal"
            @click="isAddDialogOpen = true"
          >
            データ追加
          </v-btn>
        </div>
      </div>

      <v-alert
        v-if="selectedFolderActiveTemplateId === null"
        type="info"
        color="primary"
        variant="tonal"
        density="comfortable"
        class="view-folder-template-hint"
      >
        <div class="view-folder-template-hint-content">
          <span>
            このフォルダーにはまだ有効なテンプレートがありません。テンプレートを追加または選択すると、レコード表示に利用できます。
          </span>
          <v-btn
            size="small"
            color="primary"
            variant="flat"
            :loading="isCreatingTemplate"
            @click="isTemplateDialogOpen = true"
          >
            テンプレート追加
          </v-btn>
        </div>
      </v-alert>

      <div v-if="selectedFolderRecords.length > 0" class="folder-record-list">
        <div
          v-for="record in selectedFolderRecords"
          :key="record.id"
          class="folder-record-row"
        >
          <div class="folder-record-main">
            <span class="view-tree-record-id">#{{ record.recordId }}</span>
            <span class="folder-record-label">
              {{ displayRecordLabel(record.recordLabel) }}
            </span>
            <small>{{ record.tableDisplayName }}</small>
          </div>
          <v-tooltip text="フォルダーから外す" location="bottom">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-link-off"
                size="small"
                variant="text"
                color="error"
                aria-label="フォルダーから外す"
                @click="onRemoveFolderRecord(record)"
              />
            </template>
          </v-tooltip>
        </div>
      </div>

      <p v-else class="view-empty-hint">
        このフォルダーにはまだデータがありません。既存レコードを追加すると、目次から開けるようになります。
      </p>
    </div>

    <div v-else class="view-placeholder-card view-placeholder-empty">
      <ViewModeOnboardingCard
        :folder-action-variant="hasNoFolders ? 'flat' : 'tonal'"
        :folder-count="folderCount"
        :is-creating-folder="isCreatingFolder"
        :is-creating-template="isCreatingTemplate"
        :show-guide-action="!hasNoFolders"
        :table-count="tableCount"
        :title="emptyStateTitle"
        :description="emptyStateDescription"
        @create-folder="isFolderDialogOpen = true"
        @create-template="isTemplateDialogOpen = true"
        @open-guide="onOpenModeHelp"
      />
    </div>

    <v-dialog
      v-model="isFolderDialogOpen"
      max-width="520"
      :persistent="isCreatingFolder"
    >
      <v-card rounded="xl">
        <v-card-title>フォルダー追加</v-card-title>
        <v-card-text class="d-grid ga-4">
          <v-text-field
            v-model="folderFormName"
            label="フォルダー名"
            variant="outlined"
            density="comfortable"
            :disabled="isCreatingFolder"
            @keydown.enter.prevent="createRootFolder"
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn
            variant="text"
            :disabled="isCreatingFolder"
            @click="isFolderDialogOpen = false"
          >
            閉じる
          </v-btn>
          <v-btn
            color="primary"
            variant="flat"
            :disabled="folderFormName.trim().length === 0 || isCreatingFolder"
            :loading="isCreatingFolder"
            @click="createRootFolder"
          >
            作成
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog
      v-model="isTemplateDialogOpen"
      max-width="520"
      :persistent="isCreatingTemplate"
    >
      <v-card rounded="xl">
        <v-card-title>レイアウトテンプレート追加</v-card-title>
        <v-card-text class="d-grid ga-4">
          <v-text-field
            v-model="templateFormName"
            label="テンプレート名"
            variant="outlined"
            density="comfortable"
            :disabled="isCreatingTemplate"
            @keydown.enter.prevent="createFolderLayoutTemplate"
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn
            variant="text"
            :disabled="isCreatingTemplate"
            @click="isTemplateDialogOpen = false"
          >
            閉じる
          </v-btn>
          <v-btn
            color="primary"
            variant="flat"
            :disabled="
              templateFormName.trim().length === 0 || isCreatingTemplate
            "
            :loading="isCreatingTemplate"
            @click="createFolderLayoutTemplate"
          >
            作成
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>

    <v-dialog
      v-if="selectedItem?.type === 'folder'"
      v-model="isAddDialogOpen"
      max-width="640"
      :persistent="isAddingRecords"
    >
      <v-card class="folder-record-dialog" rounded="xl">
        <v-card-title>データ追加</v-card-title>
        <v-card-text>
          <div class="folder-record-dialog-controls">
            <v-select
              v-model="selectedTableId"
              density="comfortable"
              :disabled="isAddingRecords"
              hide-details
              :items="tableSelectItems"
              label="テーブル"
              variant="outlined"
            />
            <v-text-field
              v-model="recordSearchQuery"
              density="comfortable"
              :disabled="isAddingRecords"
              hide-details
              label="検索"
              placeholder="レコード名やIDで検索"
              prepend-inner-icon="mdi-magnify"
              type="search"
              variant="outlined"
            />
          </div>

          <div class="folder-record-dialog-actions">
            <span>{{ selectedRecords.length }} 件選択中</span>
            <div>
              <v-btn
                size="small"
                variant="text"
                :disabled="
                  selectableFilteredRecords.length === 0 || isAddingRecords
                "
                @click="selectFilteredRecords"
              >
                表示中を選択
              </v-btn>
              <v-btn
                size="small"
                variant="text"
                :disabled="selectedRecordIds.length === 0 || isAddingRecords"
                @click="clearSelectedRecords"
              >
                選択解除
              </v-btn>
            </div>
          </div>

          <v-list class="folder-record-picker" density="compact" lines="two">
            <v-list-item
              v-for="record in filteredRecords"
              :key="record.id"
              class="folder-record-pick-row"
              :disabled="
                !selectedTableSection ||
                isRecordAdded(selectedTableSection, record) ||
                isAddingRecords
              "
              @click="toggleRecordSelection(record)"
            >
              <template #prepend>
                <v-checkbox-btn
                  :disabled="
                    !selectedTableSection ||
                    isRecordAdded(selectedTableSection, record) ||
                    isAddingRecords
                  "
                  :model-value="isRecordSelected(record)"
                  @click.stop="toggleRecordSelection(record)"
                />
              </template>
              <v-list-item-title>
                <span class="view-tree-record-id">#{{ record.id }}</span>
                {{ displayRecordLabel(record.label) }}
              </v-list-item-title>
              <template #append>
                <v-chip
                  v-if="
                    selectedTableSection &&
                    isRecordAdded(selectedTableSection, record)
                  "
                  size="x-small"
                  variant="tonal"
                >
                  追加済み
                </v-chip>
              </template>
            </v-list-item>

            <p v-if="filteredRecords.length === 0" class="view-empty-hint">
              該当するレコードがありません。
            </p>
          </v-list>
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn
            variant="text"
            :disabled="isAddingRecords"
            @click="isAddDialogOpen = false"
          >
            閉じる
          </v-btn>
          <v-btn
            color="primary"
            variant="flat"
            :disabled="selectedRecords.length === 0 || isAddingRecords"
            :loading="isAddingRecords"
            @click="addSelectedRecordsToSelectedFolder"
          >
            選択した{{ selectedRecords.length }}件を追加
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-card>
</template>
