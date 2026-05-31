<script setup lang="ts">
import { computed, ref, watch } from "vue";

import type {
  AppNotificationController,
  AppNotificationKind
} from "../composables/useAppNotifications";

const props = defineProps<{
  controller: AppNotificationController;
}>();

const isDetailsOpen = ref(false);

const current = computed(() => props.controller.state.current);
const hasDetails = computed(() => (current.value?.details.length ?? 0) > 0);
const metrics = computed(() => ({
  insertedCount: current.value?.metrics.insertedCount ?? 0,
  updatedCount: current.value?.metrics.updatedCount ?? 0,
  skippedCount: current.value?.metrics.skippedCount ?? 0,
  errorCount: current.value?.metrics.errorCount ?? 0
}));

const colorByKind: Record<AppNotificationKind, string> = {
  success: "success",
  warning: "warning",
  error: "error",
  info: "primary"
};

/** Snackbarを閉じるときは、詳細ダイアログも一緒に閉じて次の通知へ進めます。 */
function closeNotification() {
  isDetailsOpen.value = false;
  props.controller.close();
}

watch(
  () => current.value?.id,
  () => {
    isDetailsOpen.value = false;
  }
);
</script>

<template>
  <v-snackbar
    v-if="current"
    :model-value="controller.isOpen.value"
    class="app-notification-snackbar"
    :color="colorByKind[current.kind]"
    location="bottom right"
    :timeout="current.timeout"
    @update:model-value="!$event && closeNotification()"
  >
    <div class="app-notification-content">
      <div class="app-notification-title">{{ current.title }}</div>
      <div v-if="current.message" class="app-notification-message">
        {{ current.message }}
      </div>
      <dl class="app-notification-metrics">
        <div>
          <dt>追加</dt>
          <dd>{{ metrics.insertedCount }}</dd>
        </div>
        <div>
          <dt>更新</dt>
          <dd>{{ metrics.updatedCount }}</dd>
        </div>
        <div>
          <dt>スキップ</dt>
          <dd>{{ metrics.skippedCount }}</dd>
        </div>
        <div>
          <dt>エラー</dt>
          <dd>{{ metrics.errorCount }}</dd>
        </div>
      </dl>
      <v-btn
        v-if="hasDetails"
        class="app-notification-details-btn"
        size="small"
        variant="text"
        @click.stop="isDetailsOpen = true"
      >
        詳細を見る
      </v-btn>
    </div>

    <template #actions>
      <v-btn
        icon="mdi-close"
        variant="text"
        :aria-label="`${current.title}を閉じる`"
        @click="closeNotification"
      />
    </template>
  </v-snackbar>

  <v-dialog v-model="isDetailsOpen" max-width="520">
    <v-card rounded="lg">
      <v-card-title>{{ current?.title }}の詳細</v-card-title>
      <v-card-text>
        <ul class="app-notification-detail-list">
          <li v-for="detail in current?.details ?? []" :key="detail">
            {{ detail }}
          </li>
        </ul>
      </v-card-text>
      <v-card-actions>
        <v-spacer />
        <v-btn color="primary" variant="text" @click="isDetailsOpen = false">
          閉じる
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<style scoped>
.app-notification-snackbar {
  min-width: min(26rem, calc(100vw - 2rem));
}

.app-notification-content {
  display: grid;
  gap: 0.35rem;
}

.app-notification-title {
  font-weight: 700;
}

.app-notification-message {
  font-size: 0.86rem;
  opacity: 0.92;
}

.app-notification-metrics {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 0.4rem;
  margin: 0.25rem 0 0;
}

.app-notification-metrics div {
  display: grid;
  gap: 0.1rem;
}

.app-notification-metrics dt {
  font-size: 0.72rem;
  opacity: 0.82;
}

.app-notification-metrics dd {
  margin: 0;
  font-weight: 700;
}

.app-notification-details-btn {
  justify-self: start;
  padding-inline: 0;
}

.app-notification-detail-list {
  margin: 0;
  padding-left: 1.25rem;
  white-space: pre-line;
}
</style>
