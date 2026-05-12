<script setup lang="ts">
import type { ConfirmDialogController } from "../composables/useConfirmDialog";

defineProps<{
  controller: ConfirmDialogController;
}>();
</script>

<template>
  <v-dialog
    :model-value="controller.isOpen.value"
    max-width="420"
    @update:model-value="!$event && controller.cancel()"
  >
    <v-card rounded="lg">
      <v-card-title>{{ controller.state.title }}</v-card-title>
      <v-card-text class="confirm-dialog-message">
        {{ controller.state.message }}
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn variant="text" @click="controller.cancel">
          {{ controller.state.cancelText }}
        </v-btn>
        <v-btn
          :color="controller.state.color"
          variant="flat"
          @click="controller.confirm"
        >
          {{ controller.state.confirmText }}
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.confirm-dialog-message {
  white-space: pre-line;
}
</style>
