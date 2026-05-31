<script setup lang="ts">
import type { ViewLayoutCardItem, ViewSelection } from "../../../types";

defineProps<{
  bindableTemplateLayouts: ViewLayoutCardItem[];
  bindingColumnItems: Array<{ title: string; value: number }>;
  bindingDraft: Record<number, number | null>;
  canSaveBindingDraft: boolean;
  cardBindingLabel: (layout: ViewLayoutCardItem, index: number) => string;
  columnDisplayName: (columnId: number | null) => string;
  hasDuplicateBindingColumns: boolean;
  hasUnboundTemplateForTable: boolean;
  isBindingCardSelected: (cardId: number) => boolean;
  recordTemplateItems: Array<{ title: string; value: number | null }>;
  recordTemplateSourceChipLabel: string;
  recordTemplateSourceColor: string;
  recordTemplateTooltipText: string;
  saving: boolean;
  selectedItem: ViewSelection | null;
}>();

const emit = defineEmits<{
  "assign-record-template": [unknown];
  "clear-record-template": [];
  "close-binding-editor": [];
  "save-binding-draft": [];
  "select-binding-card": [number];
  "set-binding-draft": [number, number | null];
}>();
</script>

<template>
  <aside class="view-binding-panel" @pointerdown.stop @click.stop>
    <div class="view-record-template-settings">
      <strong>テンプレート設定</strong>
      <div class="view-record-template-heading">
        <div>
          <strong>適用中テンプレート</strong>
        </div>
        <v-tooltip
          v-if="recordTemplateTooltipText"
          :text="recordTemplateTooltipText"
          location="bottom"
        >
          <template #activator="{ props: tooltipProps }">
            <v-chip
              v-bind="tooltipProps"
              class="view-record-template-source-chip"
              size="small"
              :color="recordTemplateSourceColor"
              variant="tonal"
            >
              {{ recordTemplateSourceChipLabel }}
            </v-chip>
          </template>
        </v-tooltip>
        <v-chip
          v-else
          class="view-record-template-source-chip"
          size="small"
          :color="recordTemplateSourceColor"
          variant="tonal"
        >
          {{ recordTemplateSourceChipLabel }}
        </v-chip>
      </div>
      <div class="view-record-template-control-row">
        <v-select
          :items="recordTemplateItems"
          :model-value="
            selectedItem?.type === 'tableRecord'
              ? (selectedItem.recordTemplateId ?? null)
              : null
          "
          item-title="title"
          item-value="value"
          label="個別テンプレート"
          variant="outlined"
          density="compact"
          hide-details
          :disabled="saving || recordTemplateItems.length === 0"
          @update:model-value="emit('assign-record-template', $event)"
        />
        <v-btn
          color="primary"
          variant="tonal"
          size="small"
          :disabled="
            saving ||
            selectedItem?.type !== 'tableRecord' ||
            !selectedItem.recordTemplateId
          "
          @click="emit('clear-record-template')"
        >
          個別設定を解除
        </v-btn>
      </div>
    </div>
    <div class="view-binding-setup-copy">
      <strong>カードごとの表示項目</strong>
      <p>
        カードを選ぶと、キャンバス上の対応する位置を強調します。未使用にしたいカードは未選択のまま保存できます。
      </p>
    </div>
    <div class="view-binding-list">
      <v-card
        v-for="(layout, index) in bindableTemplateLayouts"
        :key="layout.cardId"
        :class="{ selected: isBindingCardSelected(layout.cardId) }"
        variant="outlined"
        rounded="lg"
        class="view-binding-card"
        role="button"
        tabindex="0"
        @click="emit('select-binding-card', layout.cardId)"
        @focusin="emit('select-binding-card', layout.cardId)"
      >
        <div>
          <strong>{{ cardBindingLabel(layout, index) }}</strong>
        </div>
        <v-select
          :items="bindingColumnItems"
          :model-value="bindingDraft[layout.cardId] ?? null"
          clearable
          density="compact"
          hide-details
          item-title="title"
          item-value="value"
          label="表示カラム"
          variant="outlined"
          @click.stop
          @focusin.stop
          @pointerdown.stop
          @update:model-value="
            emit(
              'set-binding-draft',
              layout.cardId,
              typeof $event === 'number' ? $event : null
            )
          "
        />
      </v-card>
    </div>
    <v-alert
      v-if="hasDuplicateBindingColumns"
      type="warning"
      variant="tonal"
      density="compact"
    >
      同じカラムを複数のカードへ紐付けることはできません。
    </v-alert>
    <div class="view-binding-actions">
      <v-btn
        v-if="!hasUnboundTemplateForTable"
        variant="text"
        :disabled="saving"
        @click="emit('close-binding-editor')"
      >
        閉じる
      </v-btn>
      <v-btn
        color="primary"
        variant="flat"
        :disabled="!canSaveBindingDraft"
        :loading="saving"
        @click="emit('save-binding-draft')"
      >
        紐付けを保存
      </v-btn>
    </div>
  </aside>
</template>
