import { open } from "@tauri-apps/plugin-dialog";
import { computed, ref, watch } from "vue";

import { api } from "../api";
import { useAppStore } from "../stores/app";

type SetupMode = "create" | "open";
type SetupErrorAction = "selectDirectory" | "selectDbFile" | "focusFileName";

interface SetupError {
  cause: string;
  action: string;
  actionType?: SetupErrorAction;
  secondaryActionType?: SetupErrorAction;
}

const DEFAULT_FILE_STEM = "datum-forge";
const FIXED_CREATE_EXTENSION = ".sqlite";

function normalizeDirectoryInput(directoryPath: string) {
  const trimmed = directoryPath.trim();
  if (/^[A-Za-z]:[\\/]*$/.test(trimmed)) {
    return `${trimmed.slice(0, 2)}\\`;
  }
  if (/^[\\/]+$/.test(trimmed)) {
    return trimmed[0];
  }
  return trimmed.replace(/[\\/]+$/, "");
}

function createDbFileNameFromStem(fileStem: string) {
  return `${fileStem.trim()}${FIXED_CREATE_EXTENSION}`;
}

function createStemFromFileName(fileName: string) {
  return fileName.trim().replace(/\.(sqlite|db)$/i, "") || DEFAULT_FILE_STEM;
}

function joinDbPath(directoryPath: string, fileStem: string) {
  const trimmed = normalizeDirectoryInput(directoryPath);
  const stem = fileStem.trim();
  if (!trimmed || !stem) {
    return "";
  }

  const separator = trimmed.includes("\\") ? "\\" : "/";
  return `${trimmed.replace(/[\\/]+$/, "")}${separator}${createDbFileNameFromStem(stem)}`;
}

function hasSupportedDbExtension(path: string) {
  return /\.(sqlite|db)$/i.test(path.trim());
}

function hasPathSeparator(value: string) {
  return /[\\/]/.test(value);
}

function isValidCreateDbFileStem(value: string) {
  const trimmed = value.trim();
  return (
    Boolean(trimmed) &&
    trimmed !== "." &&
    trimmed !== ".." &&
    !trimmed.includes(".") &&
    !hasPathSeparator(trimmed)
  );
}

function abbreviatePath(path: string) {
  if (path.length <= 58) {
    return path;
  }

  const normalized = path.split("\\").join("/");
  const parts = normalized.split("/").filter(Boolean);
  const fileName = parts[parts.length - 1] ?? "";
  const root = path.match(/^[A-Za-z]:/)?.[0] ?? parts[0] ?? "";
  return `${root}\\...\\${fileName}`;
}

function actionLabel(actionType?: SetupErrorAction) {
  switch (actionType) {
    case "selectDirectory":
      return "フォルダーを選ぶ";
    case "selectDbFile":
      return "ファイルを選ぶ";
    case "focusFileName":
      return "ファイル名を変更";
    default:
      return "";
  }
}

export function useDatabaseSetup() {
  const store = useAppStore();
  const selectedSetupMode = ref<SetupMode>("create");
  const dbDirectory = ref("");
  const dbFileName = ref(DEFAULT_FILE_STEM);
  const existingDbFilePath = ref("");
  const setupError = ref<SetupError | null>(null);
  const toastMessage = ref("");
  const isToastVisible = ref(false);

  const status = computed(() => store.startupDbStatus);
  const setupTitle = computed(() => {
    if (status.value?.state === "missingDb") {
      return "DBファイルが見つかりません";
    }
    if (status.value?.state === "error") {
      return "DB設定を確認してください";
    }
    return "DBをセットアップ";
  });
  const setupDescription = computed(() => {
    if (status.value?.state === "missingDb") {
      return "新しく作成するか、移動済みのDBを開いて作業を再開できます。";
    }
    if (status.value?.state === "error") {
      return "設定を読み込めませんでした。DBを作成または選択して復旧できます。";
    }
    return "最初に使用するDBを作成するか、既存のDBを開いてください。";
  });
  const previewDbPath = computed(() =>
    joinDbPath(dbDirectory.value, dbFileName.value)
  );
  const abbreviatedPreviewDbPath = computed(() =>
    abbreviatePath(previewDbPath.value)
  );
  const missingDbPath = computed(() => status.value?.missingDbPath ?? "");
  const abbreviatedMissingDbPath = computed(() =>
    abbreviatePath(missingDbPath.value)
  );
  const abbreviatedExistingDbFilePath = computed(() =>
    abbreviatePath(existingDbFilePath.value.trim())
  );
  const primaryErrorActionLabel = computed(() =>
    actionLabel(setupError.value?.actionType)
  );
  const secondaryErrorActionLabel = computed(() =>
    actionLabel(setupError.value?.secondaryActionType)
  );

  watch(
    status,
    (nextStatus) => {
      if (!nextStatus || nextStatus.state === "ready") {
        return;
      }

      selectedSetupMode.value = "create";
      dbDirectory.value = nextStatus.defaultDbDirectory;
      dbFileName.value = createStemFromFileName(
        nextStatus.defaultDbFileName || DEFAULT_FILE_STEM
      );

      if (nextStatus.state === "missingDb") {
        setupError.value = {
          cause: "設定されているDBファイルが見つかりません。",
          action:
            "DBを移動した場合は下部の「既存を開く」を選んでください。新しく始める場合はこのまま作成できます。",
          actionType: "selectDbFile",
          secondaryActionType: "selectDirectory"
        };
        return;
      }

      if (nextStatus.state === "error" && nextStatus.message) {
        setupError.value = {
          cause: nextStatus.message,
          action:
            "設定ファイルを読み込めません。新しいDBを作成するか、下部の「既存を開く」からDBを選択してください。",
          actionType: "selectDbFile",
          secondaryActionType: "selectDirectory"
        };
      }
    },
    { immediate: true }
  );

  function clearError() {
    setupError.value = null;
  }

  function setError(error: SetupError) {
    setupError.value = error;
  }

  function showToast(message: string) {
    toastMessage.value = message;
    isToastVisible.value = true;
  }

  async function selectDirectory() {
    clearError();
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: dbDirectory.value || status.value?.defaultDbDirectory
    });

    if (typeof selected === "string") {
      dbDirectory.value = selected;
    }
  }

  async function selectDbFile() {
    clearError();
    selectedSetupMode.value = "open";
    const selected = await open({
      multiple: false,
      defaultPath: existingDbFilePath.value || dbDirectory.value,
      filters: [
        {
          name: "SQLite Database",
          extensions: ["sqlite", "db"]
        }
      ]
    });

    if (typeof selected === "string") {
      existingDbFilePath.value = selected;
    }
  }

  async function copyPath(path: string) {
    clearError();
    const trimmed = path.trim();
    if (!trimmed) {
      setError({
        cause: "コピーするDBパスがありません。",
        action: "DBパスが表示されてからもう一度実行してください。"
      });
      return;
    }

    try {
      await globalThis.navigator.clipboard.writeText(trimmed);
      showToast("DBパスをコピーしました");
    } catch {
      setError({
        cause: "DBパスをクリップボードへコピーできませんでした。",
        action: "パス欄の内容を選択して手動でコピーしてください。"
      });
    }
  }

  async function openPathFolder(path: string) {
    clearError();
    const trimmed = path.trim();
    if (!trimmed) {
      setError({
        cause: "開くDBパスがありません。",
        action: "DBパスが表示されてからもう一度実行してください。"
      });
      return;
    }

    try {
      await api.openPathFolder(trimmed);
    } catch (error) {
      setError({
        cause: error instanceof Error ? error.message : String(error),
        action:
          "保存先フォルダーを選び直すか、既存DBファイルを参照してください。",
        actionType:
          selectedSetupMode.value === "create"
            ? "selectDirectory"
            : "selectDbFile"
      });
    }
  }

  async function submitCreate(directory: string, fileStem: string) {
    try {
      await store.createDatabaseFile({
        dbDirectory: directory,
        dbFileName: createDbFileNameFromStem(fileStem)
      });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      const likelyNameConflict =
        message.includes("exists") || message.includes("既に存在");
      setError({
        cause: message,
        action: likelyNameConflict
          ? "同じ場所に同名のDBがあります。ファイル名を変更してください。"
          : "保存先フォルダーとファイル名を確認して、もう一度作成してください。",
        actionType: likelyNameConflict ? "focusFileName" : "selectDirectory"
      });
    }
  }

  async function createDatabase() {
    clearError();
    selectedSetupMode.value = "create";
    const directory = normalizeDirectoryInput(dbDirectory.value);
    const fileStem = dbFileName.value.trim();

    if (!directory) {
      setError({
        cause: "DBの保存先フォルダーが入力されていません。",
        action: "保存先フォルダーを選んでから作成してください。",
        actionType: "selectDirectory"
      });
      return;
    }
    if (!fileStem) {
      setError({
        cause: "DBファイル名が入力されていません。",
        action: "作成するDBファイル名を入力してください。",
        actionType: "focusFileName"
      });
      return;
    }
    if (!isValidCreateDbFileStem(fileStem)) {
      setError({
        cause:
          "DBファイル名は拡張子を付けず、フォルダー区切りや . を含めずに入力してください。",
        action: "例: project と入力すると project.sqlite を作成します。",
        actionType: "focusFileName"
      });
      return;
    }

    await submitCreate(directory, fileStem);
  }

  async function quickCreateDatabase() {
    clearError();
    selectedSetupMode.value = "create";
    const directory = normalizeDirectoryInput(
      status.value?.defaultDbDirectory || dbDirectory.value
    );
    const fileStem = createStemFromFileName(
      status.value?.defaultDbFileName || DEFAULT_FILE_STEM
    );

    dbDirectory.value = directory;
    dbFileName.value = fileStem;

    if (!directory) {
      setError({
        cause: "デフォルト保存先を取得できませんでした。",
        action: "フォルダーを選んでから作成してください。",
        actionType: "selectDirectory"
      });
      return;
    }

    await submitCreate(directory, fileStem);
  }

  async function openExistingDatabase() {
    clearError();
    selectedSetupMode.value = "open";
    const trimmed = existingDbFilePath.value.trim();

    if (!trimmed) {
      setError({
        cause: "開くDBファイルが選択されていません。",
        action: "既存の .sqlite または .db ファイルを選択してください。",
        actionType: "selectDbFile"
      });
      return;
    }
    if (!hasSupportedDbExtension(trimmed)) {
      setError({
        cause: "DBファイルの拡張子が対応していません。",
        action: ".sqlite または .db ファイルを選択してください。",
        actionType: "selectDbFile"
      });
      return;
    }

    try {
      await store.setupOpenDatabaseFile(trimmed);
    } catch (error) {
      setError({
        cause: error instanceof Error ? error.message : String(error),
        action:
          "ファイルが移動または削除されていないか確認し、もう一度選択してください。",
        actionType: "selectDbFile"
      });
    }
  }

  async function runErrorAction(actionType?: SetupErrorAction) {
    switch (actionType) {
      case "selectDirectory":
        await selectDirectory();
        break;
      case "selectDbFile":
        await selectDbFile();
        break;
      case "focusFileName":
        selectedSetupMode.value = "create";
        break;
      default:
        break;
    }
  }

  return {
    abbreviatedExistingDbFilePath,
    abbreviatedMissingDbPath,
    abbreviatedPreviewDbPath,
    copyPath,
    createDatabase,
    dbDirectory,
    dbFileName,
    existingDbFilePath,
    fixedCreateExtension: FIXED_CREATE_EXTENSION,
    isToastVisible,
    loading: computed(() => store.loading),
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
  };
}
