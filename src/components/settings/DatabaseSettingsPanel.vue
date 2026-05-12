<script setup lang="ts">
import { useDatabaseSettings } from "../../composables/useDatabaseSettings";

const {
  canCreateNewDatabase,
  createDbDirectoryPath,
  createDbFileName,
  createNewDatabase,
  createPreviewDbPath,
  currentDbExtension,
  currentDbPath,
  errorMessage,
  existingDbFilePath,
  fixedCreateExtension,
  isDirectoryChanged,
  isDbFileNameChanged,
  isFileChanged,
  isToastVisible,
  loading,
  newDbFileName,
  newDirectoryPath,
  openExistingDbFile,
  previewDbPath,
  renameCurrentDbFile,
  renamePreviewDbPath,
  saveDirectory,
  selectCreateDirectory,
  selectDbFile,
  selectDirectory,
  toastMessage
} = useDatabaseSettings();
</script>

<template>
  <section class="settings-section database-settings-section">
    <div class="settings-section-heading">
      <div>
        <h2>データベース</h2>
        <p>
          DBファイルを分けて管理できるように、新規作成、保存先の変更、ファイル名の変更、
          既存DBファイルの読み込みを行います。
        </p>
      </div>
    </div>

    <v-alert
      v-if="errorMessage"
      type="error"
      variant="tonal"
      density="comfortable"
    >
      {{ errorMessage }}
    </v-alert>

    <div class="settings-form-grid">
      <div class="settings-field">
        <span class="settings-label">現在のDB</span>
        <v-tooltip :text="currentDbPath" location="top">
          <template #activator="{ props: tooltipProps }">
            <v-sheet
              v-bind="tooltipProps"
              class="readonly-path"
              color="surface"
              rounded
              border
            >
              {{ currentDbPath }}
            </v-sheet>
          </template>
        </v-tooltip>
      </div>

      <v-card
        color="surface"
        variant="elevated"
        elevation="1"
        rounded="lg"
        border
        class="settings-operation pa-4"
      >
        <div class="settings-operation-heading">
          <h3>新規DBを作成</h3>
          <p>
            保存先とファイル名を指定して空のDBを作成し、そのDBへ切り替えます。
            新規作成時の拡張子は .sqlite 固定です。
          </p>
        </div>

        <div class="settings-field">
          <span class="settings-label">保存先フォルダー</span>
          <div class="settings-path-row">
            <v-text-field
              v-model="createDbDirectoryPath"
              hide-details="auto"
              variant="outlined"
              density="comfortable"
              placeholder="D:\data"
              :disabled="loading"
            />
            <v-btn
              prepend-icon="mdi-folder-open-outline"
              variant="tonal"
              :disabled="loading"
              @click="selectCreateDirectory"
            >
              フォルダー参照
            </v-btn>
          </div>
        </div>

        <div class="settings-field">
          <span class="settings-label">DBファイル名</span>
          <v-text-field
            v-model="createDbFileName"
            hide-details="auto"
            variant="outlined"
            density="comfortable"
            placeholder="project"
            :suffix="fixedCreateExtension"
            :disabled="loading"
          />
          <p class="settings-preview-path">
            {{
              createPreviewDbPath ||
              "保存先フォルダーとDBファイル名を入力してください。"
            }}
          </p>
        </div>

        <div class="settings-actions">
          <v-btn
            color="primary"
            prepend-icon="mdi-database-plus-outline"
            :disabled="!canCreateNewDatabase || loading"
            :loading="loading"
            @click="createNewDatabase"
          >
            作成して切り替え
          </v-btn>
        </div>
      </v-card>

      <v-card
        color="surface"
        variant="elevated"
        elevation="1"
        rounded="lg"
        border
        class="settings-operation pa-4"
      >
        <div class="settings-operation-heading">
          <h3>保存先フォルダーを変更</h3>
          <p>
            現在のDBファイルを、ファイル名を保ったまま別フォルダーへ移動します。
          </p>
        </div>

        <div class="settings-field">
          <span class="settings-label">保存先フォルダー</span>
          <div class="settings-path-row">
            <v-text-field
              v-model="newDirectoryPath"
              hide-details="auto"
              variant="outlined"
              density="comfortable"
              placeholder="D:\data"
              :disabled="loading"
            />
            <v-btn
              prepend-icon="mdi-folder-open-outline"
              variant="tonal"
              :disabled="loading"
              @click="selectDirectory"
            >
              フォルダー参照
            </v-btn>
          </div>
          <p class="settings-preview-path">
            {{ previewDbPath || "現在のDBファイル名を保ったまま移動します。" }}
          </p>
        </div>

        <div class="settings-actions">
          <v-btn
            color="primary"
            prepend-icon="mdi-content-save-outline"
            :disabled="!isDirectoryChanged || loading"
            :loading="loading"
            @click="saveDirectory"
          >
            保存
          </v-btn>
        </div>
      </v-card>

      <v-card
        color="surface"
        variant="elevated"
        elevation="1"
        rounded="lg"
        border
        class="settings-operation pa-4"
      >
        <div class="settings-operation-heading">
          <h3>DBファイル名を変更</h3>
          <p>
            現在の保存フォルダー内でDBファイル名だけを変更します。
            拡張子は現在のDBと同じまま維持します。
          </p>
        </div>

        <div class="settings-field">
          <span class="settings-label">DBファイル名</span>
          <v-text-field
            v-model="newDbFileName"
            hide-details="auto"
            variant="outlined"
            density="comfortable"
            placeholder="project"
            :suffix="`.${currentDbExtension}`"
            :disabled="loading"
          />
          <p class="settings-preview-path">
            {{
              renamePreviewDbPath || "拡張子は現在のDBと同じまま維持します。"
            }}
          </p>
        </div>

        <div class="settings-actions">
          <v-btn
            color="primary"
            prepend-icon="mdi-file-document-edit-outline"
            :disabled="!isDbFileNameChanged || loading"
            :loading="loading"
            @click="renameCurrentDbFile"
          >
            名前を変更
          </v-btn>
        </div>
      </v-card>

      <v-card
        color="surface"
        variant="elevated"
        elevation="1"
        rounded="lg"
        border
        class="settings-operation pa-4"
      >
        <div class="settings-operation-heading">
          <h3>既存のDBファイルを開く</h3>
          <p>.sqlite または .db ファイルを現在のDBとして読み込みます。</p>
        </div>

        <div class="settings-field">
          <span class="settings-label">DBファイル</span>
          <div class="settings-path-row">
            <v-text-field
              v-model="existingDbFilePath"
              hide-details="auto"
              variant="outlined"
              density="comfortable"
              placeholder="D:\data\other.sqlite"
              :disabled="loading"
            />
            <v-btn
              prepend-icon="mdi-database-search-outline"
              variant="tonal"
              :disabled="loading"
              @click="selectDbFile"
            >
              ファイル参照
            </v-btn>
          </div>
        </div>

        <div class="settings-actions">
          <v-btn
            color="primary"
            prepend-icon="mdi-database-import-outline"
            :disabled="!isFileChanged || loading"
            :loading="loading"
            @click="openExistingDbFile"
          >
            このDBを開く
          </v-btn>
        </div>
      </v-card>
    </div>

    <v-snackbar v-model="isToastVisible" color="success" timeout="2400">
      {{ toastMessage }}
    </v-snackbar>
  </section>
</template>
