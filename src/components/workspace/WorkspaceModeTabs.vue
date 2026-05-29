<script setup lang="ts">
import type { WorkspaceMode } from "../../composables/useWorkspaceMode";

type WorkspaceTabMode = Exclude<WorkspaceMode, "settings">;

defineProps<{
  modelValue: WorkspaceMode;
}>();

const emit = defineEmits<{
  "update:modelValue": [WorkspaceTabMode];
}>();

/** カスタムタイトルバーに表示するワークスペースモード定義です。 */
const modes: Array<{
  id: WorkspaceTabMode;
  label: string;
  icon: string;
}> = [
  {
    id: "view",
    label: "閲覧モード",
    icon: "mdi-book-open-page-variant-outline"
  },
  { id: "design", label: "設計モード", icon: "mdi-ruler-square-compass" },
  { id: "data", label: "データモード", icon: "mdi-table-large" },
  { id: "master", label: "マスタ管理", icon: "mdi-format-list-bulleted-square" }
];

type WheelLikeEvent = {
  currentTarget: unknown;
  deltaMode: number;
  deltaX: number;
  deltaY: number;
  preventDefault: () => void;
};

type ScrollContainer = {
  clientWidth: number;
  scrollLeft: number;
  scrollWidth: number;
  scrollTo: (options: { left: number; behavior: "smooth" }) => void;
};

type QueryableTarget = {
  querySelector: (selector: string) => ScrollContainer | null;
};

const WHEEL_DELTA_LINE = 1;
const WHEEL_DELTA_PAGE = 2;
const WHEEL_SCROLL_INTERVAL_MS = 220;

let lastWheelScrollAt = 0;

function normalizeWheelDelta(event: WheelLikeEvent) {
  const delta =
    Math.abs(event.deltaX) > Math.abs(event.deltaY)
      ? event.deltaX
      : event.deltaY;

  if (event.deltaMode === WHEEL_DELTA_LINE) {
    return delta;
  }

  if (event.deltaMode === WHEEL_DELTA_PAGE) {
    return delta;
  }

  return delta;
}

function isQueryableTarget(target: unknown): target is QueryableTarget {
  return (
    target !== null &&
    typeof target === "object" &&
    typeof (target as Partial<QueryableTarget>).querySelector === "function"
  );
}

function handleTabsWheel(event: WheelLikeEvent) {
  const root = event.currentTarget;

  if (!isQueryableTarget(root)) {
    return;
  }

  const scrollContainer = root.querySelector(".v-slide-group__container");

  if (!scrollContainer) {
    return;
  }

  const maxScrollLeft =
    scrollContainer.scrollWidth - scrollContainer.clientWidth;

  if (maxScrollLeft <= 0) {
    return;
  }

  const delta = normalizeWheelDelta(event);

  if (delta === 0) {
    return;
  }

  const now = Date.now();
  if (now - lastWheelScrollAt < WHEEL_SCROLL_INTERVAL_MS) {
    event.preventDefault();
    return;
  }

  const scrollDirection = delta > 0 ? 1 : -1;
  const nextScrollLeft = Math.min(
    Math.max(
      scrollContainer.scrollLeft +
        scrollContainer.clientWidth * scrollDirection,
      0
    ),
    maxScrollLeft
  );

  if (nextScrollLeft === scrollContainer.scrollLeft) {
    return;
  }

  event.preventDefault();
  lastWheelScrollAt = now;
  scrollContainer.scrollTo({ left: nextScrollLeft, behavior: "smooth" });
}
</script>

<template>
  <!-- カスタムタイトルバー内でワークスペース全体の表示モードを切り替えます。 -->
  <div class="mode-tabs-titlebar" @wheel="handleTabsWheel">
    <v-tabs
      :model-value="modelValue"
      color="primary"
      align-tabs="start"
      density="compact"
      @update:model-value="
        emit('update:modelValue', $event as WorkspaceTabMode)
      "
    >
      <v-tab
        v-for="mode in modes"
        :key="mode.id"
        :value="mode.id"
        :prepend-icon="mode.icon"
      >
        {{ mode.label }}
      </v-tab>
    </v-tabs>
  </div>
</template>
