<script setup lang="ts">
import { computed, ref, watch } from "vue";

import ViewModeOnboardingCard from "./workspace/view/ViewModeOnboardingCard.vue";

import type { WorkspaceMode } from "../composables/useWorkspaceMode";

type HelpMode = Exclude<WorkspaceMode, "settings">;

type HelpSection = {
  title: string;
  items: string[];
};

type ModeHelp = {
  title: string;
  icon: string;
  lead: string;
  sections: HelpSection[];
};

const props = defineProps<{
  folderCount?: number;
  modelValue: boolean;
  mode: HelpMode;
  onCreateFolder?: (parentId: number | null, name: string) => Promise<void>;
  onCreateFolderLayoutTemplate?: (
    folderId: number | null,
    name: string
  ) => Promise<void>;
  tableCount?: number;
}>();

const emit = defineEmits<{
  "update:modelValue": [boolean];
}>();

const MODE_HELP: Record<HelpMode, ModeHelp> = {
  view: {
    title: "閲覧モード",
    icon: "mdi-book-open-page-variant-outline",
    lead: "フォルダーでレコードを整理し、カード枠テンプレートに沿って閲覧する画面です。",
    sections: [
      {
        title: "このモードでできること",
        items: [
          "フォルダーを作って、複数テーブルのレコードを見やすく整理できます。",
          "レコードごとにカードの位置やサイズ、見た目を調整できます。"
        ]
      },
      {
        title: "最初にやること",
        items: [
          "左サイドバーの目次でフォルダーを作成します。",
          "フォルダーを選び、テーブルから表示したいレコードを追加します。"
        ]
      },
      {
        title: "よく使う操作",
        items: [
          "テンプレートタブでカード枠を作り、表示レイアウトを整えます。",
          "レコードを開いて、カードの移動・リサイズ・スタイル変更を行います。"
        ]
      }
    ]
  },
  design: {
    title: "設計モード",
    icon: "mdi-ruler-square-compass",
    lead: "選択中テーブルのカラム構成を作る画面です。",
    sections: [
      {
        title: "このモードでできること",
        items: [
          "テーブルに必要なカラムを追加し、データの形を決めます。",
          "カラムの型、必須設定、主表示カラムを設定できます。"
        ]
      },
      {
        title: "最初にやること",
        items: [
          "管理したい情報に合わせてカラム名と表示名を入力します。",
          "文字列、数値、日付、参照、単一選択など適切な型を選びます。"
        ]
      },
      {
        title: "よく使う操作",
        items: [
          "カラム一覧で並び替え、編集、削除を行います。",
          "主表示カラムを設定して、一覧や閲覧画面で分かりやすい名前を表示します。"
        ]
      }
    ]
  },
  data: {
    title: "データモード",
    icon: "mdi-table-large",
    lead: "選択中テーブルのレコードを登録・編集する画面です。",
    sections: [
      {
        title: "このモードでできること",
        items: [
          "設計モードで作ったカラムに沿ってレコードを入力できます。",
          "登録済みレコードを検索し、必要に応じて編集・削除できます。"
        ]
      },
      {
        title: "最初にやること",
        items: [
          "レコード編集フォームに値を入力して保存します。",
          "入力項目が足りない場合は、設計モードでカラムを追加します。"
        ]
      },
      {
        title: "よく使う操作",
        items: [
          "レコード一覧の検索対象を選び、IDや表示値で絞り込みます。",
          "一覧の編集ボタンから既存レコードをフォームへ読み込みます。"
        ]
      }
    ]
  },
  master: {
    title: "マスタ管理",
    icon: "mdi-format-list-bulleted-square",
    lead: "複数テーブルで使う共通データを管理する画面です。",
    sections: [
      {
        title: "このモードでできること",
        items: [
          "単一選択カラムで使う選択肢グループを作成・整理できます。",
          "閲覧モードなどで使うタグとタググループを管理できます。"
        ]
      },
      {
        title: "最初にやること",
        items: [
          "単一選択グループを作り、選択肢を追加します。",
          "タグを分類したい場合は、タググループを先に用意します。"
        ]
      },
      {
        title: "よく使う操作",
        items: [
          "選択肢はドラッグして並び順を調整できます。",
          "作成したマスタは設計モードや閲覧モードで再利用します。"
        ]
      }
    ]
  }
};

const help = computed(() => MODE_HELP[props.mode]);
const isFolderDialogOpen = ref(false);
const isTemplateDialogOpen = ref(false);
const folderFormName = ref("");
const templateFormName = ref("");
const isCreatingFolder = ref(false);
const isCreatingTemplate = ref(false);
const viewFolderCount = computed(() => props.folderCount ?? 0);
const viewTableCount = computed(() => props.tableCount ?? 0);
const viewGuideDescription = computed(() =>
  viewFolderCount.value === 0
    ? "左側の目次からフォルダーを作成・選択すると、カードを自由配置して資料を整理できます。"
    : "フォルダー、レコード、テンプレートを組み合わせて、資料を見返しやすい自由配置ビューとして整理できます。"
);

function closeDialog() {
  emit("update:modelValue", false);
}

watch(
  () => props.modelValue,
  (isOpen) => {
    if (!isOpen) {
      isFolderDialogOpen.value = false;
      isTemplateDialogOpen.value = false;
    }
  }
);

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

async function createRootFolder() {
  const trimmedName = folderFormName.value.trim();
  if (!trimmedName || !props.onCreateFolder) {
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
  const trimmedName = templateFormName.value.trim();
  if (!trimmedName || !props.onCreateFolderLayoutTemplate) {
    return;
  }

  isCreatingTemplate.value = true;
  try {
    await props.onCreateFolderLayoutTemplate(null, trimmedName);
    isTemplateDialogOpen.value = false;
  } finally {
    isCreatingTemplate.value = false;
  }
}
</script>

<template>
  <v-dialog
    :model-value="modelValue"
    :max-width="mode === 'view' ? 860 : 680"
    @update:model-value="emit('update:modelValue', $event)"
  >
    <v-card rounded="xl">
      <v-card-title class="mode-help-title">
        <v-avatar color="primary" variant="tonal" size="40">
          <v-icon :icon="help.icon" />
        </v-avatar>
        <div>
          <span>{{ help.title }}</span>
          <p>{{ help.lead }}</p>
        </div>
        <v-spacer />
        <v-tooltip text="ヘルプを閉じる" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              icon="mdi-close"
              variant="text"
              aria-label="ヘルプを閉じる"
              @click="closeDialog"
            />
          </template>
        </v-tooltip>
      </v-card-title>

      <v-divider />

      <v-card-text v-if="mode === 'view'" class="mode-help-content">
        <div class="mode-help-view-guide">
          <ViewModeOnboardingCard
            :folder-action-variant="viewFolderCount === 0 ? 'flat' : 'tonal'"
            :folder-count="viewFolderCount"
            :is-creating-folder="isCreatingFolder"
            :is-creating-template="isCreatingTemplate"
            :table-count="viewTableCount"
            title="閲覧モードのスタートガイド"
            :description="viewGuideDescription"
            @create-folder="isFolderDialogOpen = true"
            @create-template="isTemplateDialogOpen = true"
          />
        </div>
      </v-card-text>

      <v-card-text v-else class="mode-help-content">
        <section
          v-for="section in help.sections"
          :key="section.title"
          class="mode-help-section"
        >
          <h3>{{ section.title }}</h3>
          <div class="mode-help-list">
            <div
              v-for="item in section.items"
              :key="item"
              class="mode-help-item"
            >
              <v-icon
                icon="mdi-check-circle-outline"
                size="18"
                color="primary"
              />
              <span>{{ item }}</span>
            </div>
          </div>
        </section>
      </v-card-text>

      <v-card-actions>
        <v-spacer />
        <v-btn color="primary" variant="flat" @click="closeDialog">
          閉じる
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>

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
          :disabled="templateFormName.trim().length === 0 || isCreatingTemplate"
          :loading="isCreatingTemplate"
          @click="createFolderLayoutTemplate"
        >
          作成
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.mode-help-title {
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
  padding: 1rem 1.25rem;
}

.mode-help-title p {
  margin: 0.2rem 0 0;
  font-size: 0.9rem;
  font-weight: 400;
  line-height: 1.5;
}

.mode-help-content {
  display: grid;
  gap: 1rem;
}

.mode-help-section {
  display: grid;
  gap: 0.35rem;
}

.mode-help-section h3 {
  margin: 0;
  font-size: 0.95rem;
}

.mode-help-list {
  display: grid;
  gap: 0.45rem;
}

.mode-help-item {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 0.5rem;
  align-items: start;
  line-height: 1.55;
}

.mode-help-view-guide {
  display: grid;
  justify-items: center;
  padding-block: 0.35rem 0.6rem;
}
</style>
