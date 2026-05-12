<script setup lang="ts">
import { computed, ref } from "vue";

import type { RecordTag, ViewSelection } from "../../../types";

type TableRecordSelection = Extract<ViewSelection, { type: "tableRecord" }>;

const props = defineProps<{
  allTags: RecordTag[];
  selectedItem: TableRecordSelection;
  selectedTags: RecordTag[];
  onAttachExistingTag: (
    tableId: number,
    recordId: number,
    tagId: number
  ) => Promise<void>;
  onCreateAndAttachTag: (
    tableId: number,
    recordId: number,
    name: string
  ) => Promise<void>;
  onDetachTag: (
    tableId: number,
    recordId: number,
    tagId: number
  ) => Promise<void>;
}>();

const tagInput = ref("");
const isSuggestionOpen = ref(false);
const isSubmitting = ref(false);

const selectedTagIds = computed(
  () => new Set(props.selectedTags.map((tag) => tag.id))
);

const availableTags = computed(() =>
  props.allTags.filter((tag) => !selectedTagIds.value.has(tag.id))
);

const suggestions = computed(() => {
  const keyword = tagInput.value.trim().toLocaleLowerCase();
  const source = keyword
    ? availableTags.value.filter((tag) =>
        tag.name.toLocaleLowerCase().includes(keyword)
      )
    : availableTags.value;
  return source.slice(0, 8);
});

const canCreateTag = computed(() => {
  const name = tagInput.value.trim();
  if (!name) {
    return false;
  }
  return !props.allTags.some((tag) => tag.name === name);
});

function closeSuggestion() {
  isSuggestionOpen.value = false;
  tagInput.value = "";
}

async function attachTag(tag: RecordTag) {
  if (isSubmitting.value) {
    return;
  }
  isSubmitting.value = true;
  try {
    await props.onAttachExistingTag(
      props.selectedItem.tableId,
      props.selectedItem.recordId,
      tag.id
    );
    closeSuggestion();
  } finally {
    isSubmitting.value = false;
  }
}

async function submitTagInput() {
  const name = tagInput.value.trim();
  if (!name || isSubmitting.value) {
    return;
  }

  const existingTag = availableTags.value.find((tag) => tag.name === name);
  if (existingTag) {
    await attachTag(existingTag);
    return;
  }

  isSubmitting.value = true;
  try {
    await props.onCreateAndAttachTag(
      props.selectedItem.tableId,
      props.selectedItem.recordId,
      name
    );
    closeSuggestion();
  } finally {
    isSubmitting.value = false;
  }
}

async function detachTag(tag: RecordTag) {
  await props.onDetachTag(
    props.selectedItem.tableId,
    props.selectedItem.recordId,
    tag.id
  );
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
    class="record-tag-editor"
  >
    <div class="record-tag-heading">
      <h2>タグ</h2>
    </div>

    <div class="record-tag-content">
      <div
        v-if="selectedTags.length > 0"
        class="record-tag-chip-list"
        aria-label="付与済みタグ"
      >
        <v-chip
          v-for="tag in selectedTags"
          :key="tag.id"
          class="record-tag-chip"
          closable
          color="primary"
          density="comfortable"
          size="small"
          variant="tonal"
          @click:close="detachTag(tag)"
        >
          {{ tag.name }}
        </v-chip>

        <v-menu
          v-model="isSuggestionOpen"
          location="bottom end"
          :close-on-content-click="false"
          @update:model-value="(value) => !value && closeSuggestion()"
        >
          <template #activator="{ props: activatorProps }">
            <v-tooltip text="新しいタグ" location="bottom">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="{ ...activatorProps, ...tooltipProps }"
                  color="primary"
                  density="comfortable"
                  icon="mdi-plus"
                  size="small"
                  aria-label="新しいタグ"
                  variant="tonal"
                />
              </template>
            </v-tooltip>
          </template>
          <v-card
            class="record-tag-popover"
            color="surface"
            elevation="8"
            rounded="lg"
          >
            <v-card-text class="record-tag-popover-content">
              <v-text-field
                v-model="tagInput"
                autofocus
                density="compact"
                hide-details
                placeholder="タグを検索または作成"
                prepend-inner-icon="mdi-tag-plus-outline"
                variant="outlined"
                @keydown.enter.prevent="submitTagInput"
                @keydown.escape="closeSuggestion"
              >
                <template #append-inner>
                  <v-btn
                    color="primary"
                    :disabled="!tagInput.trim() || isSubmitting"
                    density="comfortable"
                    size="small"
                    variant="tonal"
                    @click.stop="submitTagInput"
                  >
                    追加
                  </v-btn>
                </template>
              </v-text-field>

              <v-list
                v-if="suggestions.length > 0 || canCreateTag"
                class="record-tag-suggestions"
                bg-color="transparent"
                density="compact"
                lines="one"
              >
                <v-list-item
                  v-for="tag in suggestions"
                  :key="tag.id"
                  prepend-icon="mdi-tag-outline"
                  :title="tag.name"
                  @click="attachTag(tag)"
                >
                  <template #append>
                    <span class="record-tag-usage">{{ tag.usageCount }}件</span>
                  </template>
                </v-list-item>
                <v-list-item
                  v-if="canCreateTag"
                  color="primary"
                  prepend-icon="mdi-plus"
                  :title="`「${tagInput.trim()}」を新規作成`"
                  @click="submitTagInput"
                />
              </v-list>
            </v-card-text>
          </v-card>
        </v-menu>
      </div>

      <v-menu
        v-else
        v-model="isSuggestionOpen"
        location="bottom end"
        :close-on-content-click="false"
        @update:model-value="(value) => !value && closeSuggestion()"
      >
        <template #activator="{ props: activatorProps }">
          <v-btn
            v-bind="activatorProps"
            block
            color="primary"
            prepend-icon="mdi-plus"
            variant="tonal"
          >
            新しいタグ
          </v-btn>
        </template>
        <v-card
          class="record-tag-popover"
          color="surface"
          elevation="8"
          rounded="lg"
        >
          <v-card-text class="record-tag-popover-content">
            <v-text-field
              v-model="tagInput"
              autofocus
              density="compact"
              hide-details
              placeholder="タグを検索または作成"
              prepend-inner-icon="mdi-tag-plus-outline"
              variant="outlined"
              @keydown.enter.prevent="submitTagInput"
              @keydown.escape="closeSuggestion"
            >
              <template #append-inner>
                <v-btn
                  color="primary"
                  :disabled="!tagInput.trim() || isSubmitting"
                  density="comfortable"
                  size="small"
                  variant="tonal"
                  @click.stop="submitTagInput"
                >
                  追加
                </v-btn>
              </template>
            </v-text-field>

            <v-list
              v-if="suggestions.length > 0 || canCreateTag"
              class="record-tag-suggestions"
              bg-color="transparent"
              density="compact"
              lines="one"
            >
              <v-list-item
                v-for="tag in suggestions"
                :key="tag.id"
                prepend-icon="mdi-tag-outline"
                :title="tag.name"
                @click="attachTag(tag)"
              >
                <template #append>
                  <span class="record-tag-usage">{{ tag.usageCount }}件</span>
                </template>
              </v-list-item>
              <v-list-item
                v-if="canCreateTag"
                color="primary"
                prepend-icon="mdi-plus"
                :title="`「${tagInput.trim()}」を新規作成`"
                @click="submitTagInput"
              />
            </v-list>
          </v-card-text>
        </v-card>
      </v-menu>
    </div>
  </v-card>
</template>
