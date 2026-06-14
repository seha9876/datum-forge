<script setup lang="ts">
import type { ViewLayoutCardItem, ViewSelection } from "../../../types";

defineProps<{
  bindableTemplateLayouts: ViewLayoutCardItem[];
  bindingColumnItems: Array<{ title: string; value: number }>;
  bindingDraft: Record<number, Array<number | null>>;
  canSaveBindingDraft: boolean;
  cardBindingLabel: (layout: ViewLayoutCardItem, index: number) => string;
  columnDisplayName: (columnId: number | null) => string;
  hasUnboundTemplateForTable: boolean;
  isBindingCardSelected: (cardId: number) => boolean;
  isBindingSlotCountLocked: (layout: ViewLayoutCardItem) => boolean;
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
  "set-binding-draft": [number, number, number | null];
  "add-binding-slot": [number];
  "remove-binding-slot": [number, number];
  "move-binding-slot-up": [number, number];
  "move-binding-slot-down": [number, number];
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
          label="レコードテンプレート"
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
      <strong>カードごとの表示カラム</strong>
      <p>
        テンプレートで用意した表示スロットに、どのカラムを表示するかを順番どおりに設定します。
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
        <div class="view-binding-card-header">
          <strong>{{ cardBindingLabel(layout, index) }}</strong>
          <v-btn
            size="x-small"
            variant="text"
            prepend-icon="mdi-plus"
            :disabled="isBindingSlotCountLocked(layout)"
            @click.stop="emit('add-binding-slot', layout.cardId)"
          >
            行追加
          </v-btn>
        </div>

        <div class="view-binding-slot-list">
          <div
            v-for="(columnId, bindingIndex) in bindingDraft[layout.cardId] ?? [
              null
            ]"
            :key="`${layout.cardId}:${bindingIndex}`"
            class="view-binding-slot"
          >
            <div class="view-binding-slot-meta">
              <span>{{ bindingIndex + 1 }} 行目</span>
              <small>{{ columnDisplayName(columnId ?? null) }}</small>
            </div>
            <div class="view-binding-slot-controls">
              <v-select
                :items="bindingColumnItems"
                :model-value="columnId ?? null"
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
                    bindingIndex,
                    typeof $event === 'number' ? $event : null
                  )
                "
              />
              <div class="view-binding-slot-actions">
                <v-btn
                  icon="mdi-chevron-up"
                  size="x-small"
                  variant="text"
                  :disabled="
                    isBindingSlotCountLocked(layout) || bindingIndex === 0
                  "
                  @click.stop="
                    emit('move-binding-slot-up', layout.cardId, bindingIndex)
                  "
                />
                <v-btn
                  icon="mdi-chevron-down"
                  size="x-small"
                  variant="text"
                  :disabled="
                    isBindingSlotCountLocked(layout) ||
                    bindingIndex >=
                      (bindingDraft[layout.cardId]?.length ?? 1) - 1
                  "
                  @click.stop="
                    emit('move-binding-slot-down', layout.cardId, bindingIndex)
                  "
                />
                <v-btn
                  icon="mdi-close"
                  size="x-small"
                  variant="text"
                  :disabled="
                    isBindingSlotCountLocked(layout) ||
                    (bindingDraft[layout.cardId]?.length ?? 1) <= 1
                  "
                  @click.stop="
                    emit('remove-binding-slot', layout.cardId, bindingIndex)
                  "
                />
              </div>
            </div>
          </div>
        </div>
      </v-card>
    </div>

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
        保存
      </v-btn>
    </div>
  </aside>
</template>
