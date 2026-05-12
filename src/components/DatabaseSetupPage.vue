<script setup lang="ts">
import { useDatabaseSetup } from "../composables/useDatabaseSetup";

const {
  abbreviatedExistingDbFilePath,
  abbreviatedMissingDbPath,
  abbreviatedPreviewDbPath,
  copyPath,
  createDatabase,
  dbDirectory,
  dbFileName,
  existingDbFilePath,
  fixedCreateExtension,
  isToastVisible,
  loading,
  missingDbPath,
  openExistingDatabase,
  openPathFolder,
  previewDbPath,
  primaryErrorActionLabel,
  quickCreateDatabase,
  runErrorAction,
  secondaryErrorActionLabel,
  selectDbFile,
  selectDirectory,
  selectedSetupMode,
  setupDescription,
  setupError,
  setupTitle,
  toastMessage
} = useDatabaseSetup();
</script>

<template>
  <main class="database-setup-page">
    <section class="database-setup-shell">
      <div class="database-setup-heading">
        <div class="database-setup-logo" aria-hidden="true">DF</div>
        <div>
          <h1>{{ setupTitle }}</h1>
          <p>{{ setupDescription }}</p>
        </div>
      </div>

      <v-btn-toggle
        v-model="selectedSetupMode"
        mandatory
        divided
        color="primary"
        variant="tonal"
        rounded="lg"
        class="database-setup-toggle d-flex ga-1 pa-1"
      >
        <v-btn value="create" prepend-icon="mdi-database-plus-outline">
          新規作成
        </v-btn>
        <v-btn value="open" prepend-icon="mdi-database-import-outline">
          既存を開く
        </v-btn>
      </v-btn-toggle>

      <v-alert
        v-if="setupError"
        type="warning"
        variant="tonal"
        density="comfortable"
        class="database-setup-alert"
      >
        <div class="database-setup-alert-content">
          <div>
            <strong>原因: {{ setupError.cause }}</strong>
            <p>次の操作: {{ setupError.action }}</p>
          </div>
          <div
            v-if="primaryErrorActionLabel || secondaryErrorActionLabel"
            class="database-setup-alert-actions"
          >
            <v-btn
              v-if="primaryErrorActionLabel"
              size="small"
              color="primary"
              variant="tonal"
              @click="runErrorAction(setupError.actionType)"
            >
              {{ primaryErrorActionLabel }}
            </v-btn>
            <v-btn
              v-if="secondaryErrorActionLabel"
              size="small"
              variant="text"
              @click="runErrorAction(setupError.secondaryActionType)"
            >
              {{ secondaryErrorActionLabel }}
            </v-btn>
          </div>
        </div>
      </v-alert>

      <v-sheet
        v-if="missingDbPath"
        class="database-path-box pa-3"
        color="surface"
        rounded="lg"
        border
      >
        <div class="database-path-copy">
          <span>見つからなかったDB</span>
          <v-tooltip :text="missingDbPath" location="top">
            <template #activator="{ props: tooltipProps }">
              <strong v-bind="tooltipProps">
                {{ abbreviatedMissingDbPath }}
              </strong>
            </template>
          </v-tooltip>
        </div>
        <div class="database-path-actions">
          <v-tooltip text="見つからなかったDBパスをコピー" location="bottom">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-content-copy"
                variant="text"
                density="comfortable"
                :disabled="!missingDbPath"
                aria-label="見つからなかったDBパスをコピー"
                @click="copyPath(missingDbPath)"
              />
            </template>
          </v-tooltip>
          <v-tooltip
            text="見つからなかったDBのフォルダーを開く"
            location="bottom"
          >
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                icon="mdi-folder-open-outline"
                variant="text"
                density="comfortable"
                :disabled="!missingDbPath"
                aria-label="見つからなかったDBのフォルダーを開く"
                @click="openPathFolder(missingDbPath)"
              />
            </template>
          </v-tooltip>
        </div>
      </v-sheet>

      <v-card
        v-if="selectedSetupMode === 'create'"
        tag="section"
        color="surface"
        variant="elevated"
        elevation="1"
        rounded="lg"
        border
        class="database-setup-panel pa-4"
      >
        <div class="database-setup-panel-heading">
          <h2>新規DBを作成</h2>
          <p>
            保存先とファイル名を指定して、空のDBを作成します。
            新規作成時の拡張子は .sqlite 固定です。
          </p>
        </div>

        <div class="settings-field">
          <span class="settings-label">保存先フォルダー</span>
          <div class="settings-path-row">
            <v-text-field
              v-model="dbDirectory"
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
        </div>

        <div class="settings-field">
          <span class="settings-label">DBファイル名</span>
          <v-text-field
            v-model="dbFileName"
            hide-details="auto"
            variant="outlined"
            density="comfortable"
            placeholder="datum-forge"
            :suffix="fixedCreateExtension"
            :disabled="loading"
          />
        </div>

        <v-sheet
          class="database-path-box pa-3"
          color="surface"
          rounded="lg"
          border
        >
          <div class="database-path-copy">
            <span>作成予定パス</span>
            <v-tooltip :text="previewDbPath" location="top">
              <template #activator="{ props: tooltipProps }">
                <strong v-bind="tooltipProps">
                  {{
                    abbreviatedPreviewDbPath ||
                    "保存先とファイル名を入力してください"
                  }}
                </strong>
              </template>
            </v-tooltip>
          </div>
          <div class="database-path-actions">
            <v-tooltip text="作成予定パスをコピー" location="bottom">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-content-copy"
                  variant="text"
                  density="comfortable"
                  :disabled="!previewDbPath"
                  aria-label="作成予定パスをコピー"
                  @click="copyPath(previewDbPath)"
                />
              </template>
            </v-tooltip>
            <v-tooltip text="作成予定フォルダーを開く" location="bottom">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-folder-open-outline"
                  variant="text"
                  density="comfortable"
                  :disabled="!previewDbPath"
                  aria-label="作成予定フォルダーを開く"
                  @click="openPathFolder(previewDbPath)"
                />
              </template>
            </v-tooltip>
          </div>
        </v-sheet>

        <div class="database-setup-actions">
          <v-tooltip
            location="top"
            text="入力欄の内容に関係なく、標準の保存先とファイル名で作成します。"
          >
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                variant="tonal"
                prepend-icon="mdi-lightning-bolt-outline"
                :loading="loading"
                :disabled="loading"
                @click="quickCreateDatabase"
              >
                標準設定で作成
              </v-btn>
            </template>
          </v-tooltip>
          <v-btn
            color="primary"
            size="large"
            prepend-icon="mdi-database-plus-outline"
            :loading="loading"
            :disabled="loading"
            @click="createDatabase"
          >
            作成する
          </v-btn>
        </div>
      </v-card>

      <v-card
        v-else
        tag="section"
        color="surface"
        variant="elevated"
        elevation="1"
        rounded="lg"
        border
        class="database-setup-panel pa-4"
      >
        <div class="database-setup-panel-heading">
          <h2>既存DBを開く</h2>
          <p>.sqlite または .db ファイルを選択して作業を再開します。</p>
        </div>

        <div class="settings-field">
          <span class="settings-label">DBファイル</span>
          <div class="settings-path-row">
            <v-text-field
              v-model="existingDbFilePath"
              hide-details="auto"
              variant="outlined"
              density="comfortable"
              placeholder="D:\data\datum-forge.sqlite"
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

        <v-sheet
          class="database-path-box pa-3"
          color="surface"
          rounded="lg"
          border
        >
          <div class="database-path-copy">
            <span>選択中のDB</span>
            <v-tooltip :text="existingDbFilePath" location="top">
              <template #activator="{ props: tooltipProps }">
                <strong v-bind="tooltipProps">
                  {{
                    abbreviatedExistingDbFilePath ||
                    "DBファイルを選択してください"
                  }}
                </strong>
              </template>
            </v-tooltip>
          </div>
          <div class="database-path-actions">
            <v-tooltip text="選択中のDBパスをコピー" location="bottom">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-content-copy"
                  variant="text"
                  density="comfortable"
                  :disabled="!existingDbFilePath"
                  aria-label="選択中のDBパスをコピー"
                  @click="copyPath(existingDbFilePath)"
                />
              </template>
            </v-tooltip>
            <v-tooltip text="選択中DBのフォルダーを開く" location="bottom">
              <template #activator="{ props: tooltipProps }">
                <v-btn
                  v-bind="tooltipProps"
                  icon="mdi-folder-open-outline"
                  variant="text"
                  density="comfortable"
                  :disabled="!existingDbFilePath"
                  aria-label="選択中DBのフォルダーを開く"
                  @click="openPathFolder(existingDbFilePath)"
                />
              </template>
            </v-tooltip>
          </div>
        </v-sheet>

        <div class="database-setup-actions">
          <v-btn
            color="primary"
            size="large"
            prepend-icon="mdi-database-import-outline"
            :loading="loading"
            :disabled="loading"
            @click="openExistingDatabase"
          >
            DBを開く
          </v-btn>
        </div>
      </v-card>

      <v-snackbar v-model="isToastVisible" color="success" timeout="2200">
        {{ toastMessage }}
      </v-snackbar>
    </section>
  </main>
</template>
