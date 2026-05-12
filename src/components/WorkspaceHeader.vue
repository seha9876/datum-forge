<script setup lang="ts">
import WorkspaceModeTabs from "./workspace/WorkspaceModeTabs.vue";

import type { WorkspaceMode } from "../composables/useWorkspaceMode";

defineProps<{
  modelValue: WorkspaceMode;
  tableTitle: string;
  tableSubtitle: string;
  onOpenModeHelp: () => void;
  onOpenSettings: () => void;
}>();

const emit = defineEmits<{
  "update:modelValue": [WorkspaceMode];
}>();
</script>

<template>
  <v-card
    class="topbar-shell"
    color="surface"
    variant="elevated"
    rounded="xl"
    elevation="2"
    border
  >
    <div class="topbar-row">
      <div class="topbar-left">
        <div class="topbar-title-wrap">
          <strong>{{ tableTitle }}</strong>
          <small>{{ tableSubtitle }}</small>
        </div>
        <WorkspaceModeTabs
          class="topbar-mode-tabs"
          :model-value="modelValue"
          @update:model-value="emit('update:modelValue', $event)"
        />
      </div>

      <div class="topbar-actions">
        <v-tooltip
          v-if="modelValue !== 'settings'"
          text="このモードのヘルプ"
          location="bottom"
        >
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              icon="mdi-help-circle-outline"
              variant="text"
              aria-label="このモードのヘルプ"
              @click="onOpenModeHelp"
            />
          </template>
        </v-tooltip>

        <v-tooltip text="設定" location="bottom">
          <template #activator="{ props: tooltipProps }">
            <v-btn
              v-bind="tooltipProps"
              icon="mdi-cog"
              variant="text"
              :color="modelValue === 'settings' ? 'primary' : undefined"
              aria-label="設定"
              @click="onOpenSettings"
            />
          </template>
        </v-tooltip>
      </div>
    </div>
  </v-card>
</template>
