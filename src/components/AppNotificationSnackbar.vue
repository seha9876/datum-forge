<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";

import type {
  AppNotificationController,
  AppNotificationKind
} from "../composables/useAppNotifications";
import type { NotificationSettings } from "../types";

const props = defineProps<{
  controller: AppNotificationController;
  notificationSettings?: NotificationSettings;
}>();

const isDetailsOpen = ref(false);
const isHovering = ref(false);
const remainingMs = ref(0);
const totalMs = ref(0);
let timerId: number | null = null;

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

const defaultNotificationSettings: NotificationSettings = {
  usePerKindDurations: false,
  commonDurationSeconds: 4,
  successDurationSeconds: 4,
  warningDurationSeconds: 4,
  errorDurationSeconds: 4
};

const progressValue = computed(() =>
  totalMs.value > 0 ? (remainingMs.value / totalMs.value) * 100 : 0
);
const shouldShowProgress = computed(() => totalMs.value > 0);

function durationSecondsForKind(kind: AppNotificationKind) {
  const settings = props.notificationSettings ?? defaultNotificationSettings;
  if (!settings.usePerKindDurations) {
    return settings.commonDurationSeconds;
  }
  switch (kind) {
    case "success":
      return settings.successDurationSeconds;
    case "warning":
      return settings.warningDurationSeconds;
    case "error":
      return settings.errorDurationSeconds;
    case "info":
      return settings.commonDurationSeconds;
  }
}

function clearTimer() {
  if (timerId === null) {
    return;
  }
  window.clearInterval(timerId);
  timerId = null;
}

function startTimer() {
  clearTimer();
  const notification = current.value;
  if (!notification) {
    remainingMs.value = 0;
    totalMs.value = 0;
    return;
  }

  const seconds = Math.min(
    60,
    Math.max(0, durationSecondsForKind(notification.kind))
  );
  remainingMs.value = seconds * 1000;
  totalMs.value = seconds * 1000;
  if (seconds === 0) {
    return;
  }

  timerId = window.setInterval(() => {
    if (isHovering.value) {
      return;
    }
    remainingMs.value = Math.max(0, remainingMs.value - 100);
    if (remainingMs.value === 0) {
      closeNotification();
    }
  }, 100);
}

/** Snackbarを閉じるときは、詳細ダイアログも一緒に閉じて次の通知へ進めます。 */
function closeNotification() {
  clearTimer();
  isDetailsOpen.value = false;
  props.controller.close();
}

watch(
  () => current.value?.id,
  () => {
    isDetailsOpen.value = false;
    isHovering.value = false;
    startTimer();
  }
);

watch(
  () => props.notificationSettings,
  () => {
    startTimer();
  },
  { deep: true }
);

onBeforeUnmount(() => {
  clearTimer();
});
</script>

<template>
  <v-snackbar
    v-if="current"
    :model-value="controller.isOpen.value"
    class="app-notification-snackbar"
    :color="colorByKind[current.kind]"
    location="bottom right"
    :timeout="-1"
    @mouseenter="isHovering = true"
    @mouseleave="isHovering = false"
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
      <v-progress-linear
        v-if="shouldShowProgress"
        class="app-notification-progress"
        bg-opacity="0.22"
        color="white"
        height="3"
        :model-value="progressValue"
      />
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

.app-notification-progress {
  margin-top: 0.15rem;
}

.app-notification-detail-list {
  margin: 0;
  padding-left: 1.25rem;
  white-space: pre-line;
}
</style>
