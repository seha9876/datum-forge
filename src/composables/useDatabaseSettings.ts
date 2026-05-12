import { open } from "@tauri-apps/plugin-dialog";
import { computed, ref, watch } from "vue";

import { useAppStore } from "../stores/app";

import { useConfirmDialog } from "./useConfirmDialog";

const DB_FILE_NAME = "datum-forge.sqlite";
const DB_FILE_STEM = "datum-forge";
const FIXED_CREATE_EXTENSION = ".sqlite";
const SWITCH_CONFIRM_MESSAGE =
  "DBを切り替えると現在表示中のデータが再読み込みされます。続行しますか？";
const RENAME_CONFIRM_MESSAGE =
  "DBファイル名を変更すると現在表示中のデータが再読み込みされます。続行しますか？";
const CREATE_CONFIRM_MESSAGE =
  "新しいDBを作成して現在の接続先に切り替えます。現在表示中のデータは再読み込みされます。続行しますか？";

/** DB ファイルパスから、親フォルダー部分だけを取り出します。 */
function getDirectoryPath(filePath: string) {
  const normalized = filePath.split("\\").join("/");
  const index = normalized.lastIndexOf("/");
  return index >= 0 ? filePath.slice(0, index) : "";
}

/** DB ファイルパスから、ファイル名だけを取り出します。 */
function getFileName(filePath: string) {
  const normalized = filePath.split("\\").join("/");
  const parts = normalized.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? DB_FILE_NAME;
}

/** 拡張子を除いたファイル名を、入力欄の初期値に使います。 */
function getFileStem(fileName: string) {
  const index = fileName.lastIndexOf(".");
  return index > 0 ? fileName.slice(0, index) : fileName;
}

/** 現在の DB 拡張子を取得し、リネーム後も同じ拡張子を保つために使います。 */
function getExtension(fileName: string) {
  const index = fileName.lastIndexOf(".");
  return index > 0 ? fileName.slice(index + 1) : "sqlite";
}

/** フォルダー入力の末尾区切りをそろえ、比較しやすくします。 */
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

/** フォルダーとファイル名から、プレビュー用の DB パスを組み立てます。 */
function joinDbPath(directoryPath: string, fileName = DB_FILE_NAME) {
  const trimmed = normalizeDirectoryInput(directoryPath);
  const normalizedFileName = fileName.trim();
  if (!trimmed || !normalizedFileName) {
    return "";
  }

  const separator = trimmed.includes("\\") ? "\\" : "/";
  return `${trimmed.replace(/[\\/]+$/, "")}${separator}${normalizedFileName}`;
}

function createDbFileNameFromStem(stem: string) {
  return `${stem.trim()}${FIXED_CREATE_EXTENSION}`;
}

function joinCreateDbPath(directoryPath: string, stem: string) {
  const trimmedStem = stem.trim();
  if (!trimmedStem) {
    return "";
  }
  return joinDbPath(directoryPath, createDbFileNameFromStem(trimmedStem));
}

/** 長いパスを画面上で読みやすい短い表記にします。 */
function abbreviatePath(path: string) {
  if (path.length <= 44) {
    return path;
  }

  const normalized = path.split("\\").join("/");
  const parts = normalized.split("/").filter(Boolean);
  const fileName = parts[parts.length - 1] ?? DB_FILE_NAME;
  const root = path.match(/^[A-Za-z]:/)?.[0] ?? parts[0] ?? "";
  return `${root}...${fileName ? `\\${fileName}` : ""}`;
}

/** 選択したファイルが SQLite DB として扱える拡張子か確認します。 */
function hasSupportedDbExtension(path: string) {
  return /\.(sqlite|db)$/i.test(path.trim());
}

/** ファイル名入力にパス区切りが混ざっていないか確認します。 */
function hasPathSeparator(value: string) {
  return /[\\/]/.test(value);
}

function isValidCreateDbFileStem(value: string) {
  const trimmed = value.trim();
  return (
    trimmed !== "" &&
    trimmed !== "." &&
    trimmed !== ".." &&
    !trimmed.includes(".") &&
    !hasPathSeparator(trimmed)
  );
}

export function useDatabaseSettings() {
  const store = useAppStore();
  const confirmDialog = useConfirmDialog();
  const createDbDirectoryPath = ref("");
  const createDbFileName = ref(DB_FILE_STEM);
  const newDirectoryPath = ref("");
  const newDbFileName = ref("");
  const existingDbFilePath = ref("");
  const errorMessage = ref("");
  const toastMessage = ref("");
  const isToastVisible = ref(false);

  const currentDbPath = computed(() => store.settings?.dbPath ?? "");
  const currentDbDirectory = computed(() =>
    getDirectoryPath(currentDbPath.value)
  );
  const currentDbFileName = computed(() => getFileName(currentDbPath.value));
  const currentDbExtension = computed(() =>
    getExtension(currentDbFileName.value)
  );
  const currentDbFileStem = computed(() =>
    getFileStem(currentDbFileName.value)
  );
  const abbreviatedCurrentDbPath = computed(() =>
    abbreviatePath(currentDbPath.value)
  );
  const createPreviewDbPath = computed(() =>
    joinCreateDbPath(createDbDirectoryPath.value, createDbFileName.value)
  );
  const previewDbPath = computed(() =>
    joinDbPath(newDirectoryPath.value, currentDbFileName.value)
  );
  const isDirectoryChanged = computed(
    () =>
      normalizeDirectoryInput(newDirectoryPath.value) !== "" &&
      normalizeDirectoryInput(newDirectoryPath.value) !==
        normalizeDirectoryInput(currentDbDirectory.value)
  );
  const isFileChanged = computed(
    () =>
      existingDbFilePath.value.trim() !== "" &&
      existingDbFilePath.value.trim() !== currentDbPath.value
  );
  const renamePreviewDbPath = computed(() =>
    joinDbPath(currentDbDirectory.value, newDbFileName.value.trim())
  );
  const isDbFileNameChanged = computed(
    () =>
      newDbFileName.value.trim() !== "" &&
      newDbFileName.value.trim() !== currentDbFileStem.value
  );
  const canCreateNewDatabase = computed(
    () =>
      normalizeDirectoryInput(createDbDirectoryPath.value) !== "" &&
      createDbFileName.value.trim() !== ""
  );

  watch(
    currentDbDirectory,
    (directory) => {
      newDirectoryPath.value = directory;
      createDbDirectoryPath.value = directory;
    },
    { immediate: true }
  );

  watch(
    currentDbFileStem,
    (fileStem) => {
      newDbFileName.value = fileStem;
    },
    { immediate: true }
  );

  function showToast(message: string) {
    toastMessage.value = message;
    isToastVisible.value = true;
  }

  async function confirmSwitch() {
    return confirmDialog.open({
      title: "DBの切り替え",
      message: SWITCH_CONFIRM_MESSAGE,
      confirmText: "続行",
      color: "primary"
    });
  }

  async function confirmCreate() {
    return confirmDialog.open({
      title: "新規DBの作成",
      message: CREATE_CONFIRM_MESSAGE,
      confirmText: "作成して切り替え",
      color: "primary"
    });
  }

  async function confirmRename() {
    return confirmDialog.open({
      title: "DBファイル名の変更",
      message: RENAME_CONFIRM_MESSAGE,
      confirmText: "変更",
      color: "primary"
    });
  }

  async function selectCreateDirectory() {
    errorMessage.value = "";
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: createDbDirectoryPath.value || currentDbDirectory.value
    });

    if (typeof selected === "string") {
      createDbDirectoryPath.value = selected;
    }
  }

  async function selectDirectory() {
    errorMessage.value = "";
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: newDirectoryPath.value || currentDbDirectory.value
    });

    if (typeof selected === "string") {
      newDirectoryPath.value = selected;
    }
  }

  async function selectDbFile() {
    errorMessage.value = "";
    const selected = await open({
      multiple: false,
      defaultPath: existingDbFilePath.value || currentDbDirectory.value,
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

  async function createNewDatabase() {
    errorMessage.value = "";
    const dbDirectory = normalizeDirectoryInput(createDbDirectoryPath.value);
    const dbFileStem = createDbFileName.value.trim();

    if (!dbDirectory) {
      errorMessage.value = "DBの保存先フォルダーを入力してください。";
      return;
    }

    if (!dbFileStem) {
      errorMessage.value = "DBファイル名を入力してください。";
      return;
    }

    if (!isValidCreateDbFileStem(dbFileStem)) {
      errorMessage.value =
        "DBファイル名は拡張子を付けず、フォルダー区切りや . を含めずに入力してください。";
      return;
    }

    if (!(await confirmCreate())) {
      return;
    }

    try {
      await store.createDatabaseFile({
        dbDirectory,
        dbFileName: createDbFileNameFromStem(dbFileStem)
      });
      showToast("新しいDBを作成して切り替えました");
    } catch (error) {
      errorMessage.value =
        error instanceof Error ? error.message : String(error);
    }
  }

  async function saveDirectory() {
    errorMessage.value = "";
    const trimmed = normalizeDirectoryInput(newDirectoryPath.value);

    if (!trimmed) {
      errorMessage.value = "DBの保存先フォルダーを入力してください。";
      return;
    }

    if (!(await confirmSwitch())) {
      return;
    }

    try {
      await store.updateDatabaseDirectory(trimmed);
      showToast("保存先を変更しました");
    } catch (error) {
      errorMessage.value =
        error instanceof Error ? error.message : String(error);
    }
  }

  async function renameCurrentDbFile() {
    errorMessage.value = "";
    const trimmed = newDbFileName.value.trim();

    if (!trimmed) {
      errorMessage.value = "DBファイル名を入力してください。";
      return;
    }

    if (trimmed === "." || trimmed === ".." || hasPathSeparator(trimmed)) {
      errorMessage.value =
        "DBファイル名にはフォルダー区切りを含めないでください。";
      return;
    }

    if (!(await confirmRename())) {
      return;
    }

    try {
      await store.renameDatabaseFile(trimmed);
      showToast("DBファイル名を変更しました");
    } catch (error) {
      errorMessage.value =
        error instanceof Error ? error.message : String(error);
    }
  }

  async function openExistingDbFile() {
    errorMessage.value = "";
    const trimmed = existingDbFilePath.value.trim();

    if (!trimmed) {
      errorMessage.value = "読み込むDBファイルを入力してください。";
      return;
    }

    if (!hasSupportedDbExtension(trimmed)) {
      errorMessage.value =
        "DBファイルは .sqlite または .db を指定してください。";
      return;
    }

    if (!(await confirmSwitch())) {
      return;
    }

    try {
      await store.openDatabaseFile(trimmed);
      showToast("DBを読み込みました");
    } catch (error) {
      errorMessage.value =
        error instanceof Error ? error.message : String(error);
    }
  }

  return {
    abbreviatedCurrentDbPath,
    canCreateNewDatabase,
    createDbDirectoryPath,
    createDbFileName,
    createNewDatabase,
    createPreviewDbPath,
    currentDbPath,
    errorMessage,
    existingDbFilePath,
    fixedCreateExtension: FIXED_CREATE_EXTENSION,
    isDirectoryChanged,
    isDbFileNameChanged,
    isFileChanged,
    isToastVisible,
    loading: computed(() => store.loading),
    currentDbExtension,
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
  };
}
