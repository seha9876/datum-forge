<script setup lang="ts">
import { computed } from "vue";
import { VueDraggable } from "vue-draggable-plus";

import type {
  AppBootstrap,
  SaveOptionGroupPayload,
  SelectOptionGroup
} from "../../../types";

const props = defineProps<{
  bootstrap: AppBootstrap | null;
  optionGroupForm: SaveOptionGroupPayload;
  selectedOptionGroupId: number | null;
  onAddOptionRow: () => void;
  onRemoveOptionRow: (index: number) => void;
  onResetOptionGroupForm: () => void;
  onStartEditOptionGroup: (group: SelectOptionGroup) => void;
  onSubmitOptionGroup: () => Promise<void>;
  onSyncOptionOrdering: () => void;
}>();

/** 既存グループをフォームへ読み込んでいるかを表します。 */
const isEditingOptionGroup = computed(
  () => props.selectedOptionGroupId !== null
);

/** フォームの状態に合わせて見出し文言を切り替えます。 */
const formTitle = computed(() =>
  isEditingOptionGroup.value ? "単一選択グループ編集" : "単一選択グループ管理"
);

/** ドラッグ操作しやすいよう選択肢行を computed 経由で束ねています。 */
const optionRows = computed({
  get: () => props.optionGroupForm.options,
  set: (value) => {
    // ドラッグ後の配列順をフォーム本体へそのまま反映します。
    props.optionGroupForm.options = value;
  }
});

/** 編集状態に合わせて保存ボタンの文言を切り替えます。 */
const saveButtonLabel = computed(() =>
  isEditingOptionGroup.value ? "更新" : "保存"
);

/**
 * ドラッグ後の見た目順に optionNo / sortOrder を再採番します。
 */
function handleDragEnd() {
  props.onSyncOptionOrdering();
}

/**
 * 登録済みグループカードをキーボード操作でも選択できるようにします。
 *
 * @param event キーボードイベント
 * @param group 選択する単一選択グループ
 */
function handleGroupCardKeydown(
  event: globalThis.KeyboardEvent,
  group: SelectOptionGroup
) {
  if (event.key !== "Enter" && event.key !== " ") {
    return;
  }

  event.preventDefault();
  props.onStartEditOptionGroup(group);
}
</script>

<template>
  <div class="mode-grid">
    <!-- 単一選択グループを新規作成・編集するフォーム領域です。 -->
    <v-card
      tag="section"
      color="surface"
      variant="elevated"
      rounded="xl"
      elevation="2"
      border
      class="pa-4"
    >
      <div class="section-heading">
        <div>
          <h2>{{ formTitle }}</h2>
          <p class="help-text">
            `single_select` 型で使う選択肢グループをまとめて管理します。
          </p>
        </div>
      </div>

      <!-- グループ自体の基本情報を入力します。 -->
      <v-text-field
        v-model="optionGroupForm.name"
        density="comfortable"
        hide-details
        label="グループ名"
        placeholder="status"
        variant="outlined"
      />
      <v-text-field
        v-model="optionGroupForm.description"
        density="comfortable"
        hide-details
        label="説明"
        placeholder="状態管理用"
        variant="outlined"
      />

      <!-- グループに属する選択肢一覧の編集ヘッダーです。 -->
      <div class="option-list-header">
        <strong>選択肢</strong>
        <span class="help-inline">ドラッグして並び順を調整できます</span>
      </div>

      <!-- 各選択肢をドラッグで並び替えながら編集します。 -->
      <VueDraggable
        v-model="optionRows"
        class="option-grid"
        handle=".drag-handle"
        item-key="clientKey"
        ghost-class="option-card-ghost"
        chosen-class="option-card-chosen"
        drag-class="option-card-drag"
        :animation="0"
        :force-fallback="true"
        :fallback-on-body="true"
        @end="handleDragEnd"
      >
        <div
          v-for="(option, index) in optionRows"
          :key="option.clientKey ?? `option-${index}`"
          class="option-card"
        >
          <div class="option-card-head">
            <div class="option-card-header">
              <v-tooltip text="並び順を変更" location="bottom">
                <template #activator="{ props: tooltipProps }">
                  <button
                    v-bind="tooltipProps"
                    class="drag-handle"
                    type="button"
                    aria-label="並び順を変更"
                  >
                    ::
                  </button>
                </template>
              </v-tooltip>
              <span>選択肢 {{ index + 1 }}</span>
            </div>
            <v-btn
              color="error"
              density="comfortable"
              :disabled="optionRows.length <= 1"
              size="small"
              variant="tonal"
              @click="onRemoveOptionRow(index)"
            >
              削除
            </v-btn>
          </div>
          <v-text-field
            v-model="option.label"
            density="compact"
            hide-details
            label="ラベル"
            placeholder="未対応 / 対応中 / 完了"
            variant="outlined"
          />
        </div>
      </VueDraggable>

      <!-- 選択肢追加とグループ保存の操作群です。 -->
      <div class="toolbar">
        <v-btn prepend-icon="mdi-plus" variant="tonal" @click="onAddOptionRow"
          >選択肢を追加</v-btn
        >
        <div class="toolbar-actions">
          <v-btn
            v-if="isEditingOptionGroup"
            variant="tonal"
            @click="onResetOptionGroupForm"
          >
            キャンセル
          </v-btn>
          <v-btn color="primary" variant="flat" @click="onSubmitOptionGroup">{{
            saveButtonLabel
          }}</v-btn>
        </div>
      </div>
    </v-card>

    <!-- 保存済みの単一選択グループを参照する一覧領域です。 -->
    <v-card
      tag="section"
      color="surface"
      variant="elevated"
      rounded="xl"
      elevation="2"
      border
      class="pa-4"
    >
      <div class="section-heading">
        <div>
          <h2>登録済みグループ</h2>
          <p class="help-text">現在利用できる単一選択グループの一覧です。</p>
        </div>
      </div>

      <!-- 各グループの名前、説明、選択肢数をカード形式で表示します。 -->
      <div class="group-cards">
        <v-card
          v-for="group in bootstrap?.optionGroups ?? []"
          :key="group.id"
          :aria-pressed="group.id === selectedOptionGroupId"
          :class="[
            'group-card',
            { selected: group.id === selectedOptionGroupId }
          ]"
          color="surface"
          elevation="0"
          role="button"
          rounded="lg"
          tabindex="0"
          variant="elevated"
          border
          @click="onStartEditOptionGroup(group)"
          @keydown="handleGroupCardKeydown($event, group)"
        >
          <div class="section-header">
            <strong>{{ group.name }}</strong>
            <span>{{ group.options.length }}件</span>
          </div>
          <p v-if="group.description" class="help-text">
            {{ group.description }}
          </p>
          <div class="chips">
            <v-chip
              v-for="option in group.options"
              :key="option.id"
              color="primary"
              size="small"
              variant="tonal"
            >
              {{ option.label }}
            </v-chip>
          </div>
        </v-card>
      </div>
    </v-card>
  </div>
</template>
