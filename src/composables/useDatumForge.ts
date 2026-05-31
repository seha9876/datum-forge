import { computed, onMounted, reactive, ref, watch } from "vue";

import { useAppStore } from "../stores/app";

import {
  createDefaultOptionRows,
  createOptionClientKey,
  fieldTypeLabel,
  fieldTypes,
  inputType,
  isRequiredValueEmpty,
  normalizeRecordValues as normalizeRecordValuesForForm
} from "./datumForgeForms";

import type {
  AddColumnPayload,
  AppColumn,
  ImportTableCsvMode,
  ImportColumnMappingPayload,
  ReferenceChoice,
  SaveOptionGroupPayload,
  SelectOptionGroup,
  SaveRecordPayload,
  UpdateLabelColumnPayload
} from "../types";

/**
 * Datum Forge 全体で使うフォーム状態と操作関数をまとめて提供します。
 *
 * @returns テーブル・カラム・レコード・選択肢グループ操作に必要な状態と関数
 */
export function useDatumForge() {
  /** API 呼び出しと状態更新をまとめるアプリストアです。 */
  const store = useAppStore();

  /** テーブル作成ダイアログの入力状態です。 */
  const tableForm = reactive({
    tableName: "",
    displayName: ""
  });

  /** カラム追加フォームの入力状態です。 */
  const columnForm = reactive<AddColumnPayload>({
    tableId: 0,
    columnName: "",
    displayName: "",
    fieldType: "text",
    isRequired: false,
    selectOptionGroupId: null,
    refTableId: null
  });

  /** 単一選択グループ管理フォームの入力状態です。 */
  const optionGroupForm = reactive<SaveOptionGroupPayload>({
    name: "",
    description: "",
    options: createDefaultOptionRows()
  });

  /** 編集対象として選択中の単一選択グループIDです。新規作成時は null です。 */
  const selectedOptionGroupId = ref<number | null>(null);

  /** 編集中レコードのIDです。新規作成時は null を保ちます。 */
  const editingRecordId = ref<number | null>(null);
  /** レコード編集フォームにバインドする値の集合です。 */
  const recordValues = ref<Record<string, unknown>>({});

  /** サイドバーで選択中のテーブル詳細です。 */
  const selectedTable = computed(() => store.currentTable);

  watch(
    () => store.selectedTableId,
    (tableId) => {
      if (!tableId) {
        return;
      }

      // テーブル切り替え時に、カラム追加フォームとレコード編集状態を同期します。
      columnForm.tableId = tableId;
      resetRecordForm();
    },
    { immediate: true }
  );

  /**
   * レコード入力フォームを新規作成状態へ戻します。
   */
  function resetRecordForm() {
    recordValues.value = {};
    editingRecordId.value = null;
  }

  /**
   * 指定レコードをフォームへ読み込み、編集モードへ切り替えます。
   *
   * @param recordId 編集対象レコードID
   */
  function startEditRecord(recordId: number) {
    const record = store.currentTable?.records.find(
      (item) => item.id === recordId
    );
    if (!record) {
      return;
    }

    recordValues.value = normalizeRecordValuesForForm(
      record,
      store.currentTable?.columns
        .filter((column) => column.fieldType === "boolean")
        .map((column) => column.columnName) ?? []
    );
    editingRecordId.value = recordId;
  }

  /**
   * カラムの追加情報を表示用テキストへ整形します。
   *
   * @param column 対象カラム
   * @returns 追加情報の表示文言
   */
  function fieldTypeMeta(column: AppColumn) {
    if (column.fieldType === "single_select") {
      const group = store.bootstrap?.optionGroups.find(
        (item) => item.id === column.selectOptionGroupId
      );
      return group ? `グループ: ${group.name}` : "グループ未設定";
    }

    if (column.fieldType === "reference") {
      const table = store.bootstrap?.tables.find(
        (item) => item.id === column.refTableId
      );
      return table ? `参照先: ${table.displayName}` : "参照先未設定";
    }

    return "";
  }

  /**
   * 参照型カラムに対応する候補一覧を返します。
   *
   * @param column 対象カラム
   * @returns 参照候補一覧
   */
  function referenceChoices(column: AppColumn) {
    if (!column.refTableId) {
      return [] as ReferenceChoice[];
    }

    return store.references[column.refTableId] ?? [];
  }

  /**
   * テーブル作成フォームを送信します。
   */
  async function submitTable() {
    if (!tableForm.tableName || !tableForm.displayName) {
      return;
    }

    await store.createTable({ ...tableForm });
    tableForm.tableName = "";
    tableForm.displayName = "";
  }

  /**
   * カラム作成フォームを送信し、完了後に初期値へ戻します。
   */
  async function submitColumn() {
    if (!store.selectedTableId) {
      return;
    }

    columnForm.tableId = store.selectedTableId;
    await store.addColumn({ ...columnForm });

    Object.assign(columnForm, {
      tableId: store.selectedTableId,
      columnName: "",
      displayName: "",
      fieldType: "text",
      isRequired: false,
      selectOptionGroupId: null,
      refTableId: null
    } satisfies AddColumnPayload);
  }

  /**
   * 指定カラムを削除し、レコード編集中状態もクリアします。
   *
   * @param columnId 削除対象カラムID
   */
  async function deleteColumn(columnId: number) {
    if (!store.selectedTableId) {
      return;
    }

    await store.deleteColumn({
      tableId: store.selectedTableId,
      columnId
    });
    resetRecordForm();
  }

  /**
   * カラム名と表示名を更新します。
   *
   * @param columnId 更新対象カラムID
   * @param columnName 保存する物理名
   * @param displayName 保存する表示名
   */
  async function updateColumn(
    columnId: number,
    columnName: string,
    displayName: string,
    isRequired: boolean
  ) {
    if (!store.selectedTableId) {
      return;
    }

    await store.updateColumn({
      tableId: store.selectedTableId,
      columnId,
      columnName,
      displayName,
      isRequired
    });
    resetRecordForm();
  }

  /**
   * テーブルの主表示カラムを更新します。
   *
   * @param labelColumnId 主表示に使うカラムID
   */
  async function updateLabelColumn(labelColumnId: number | null) {
    if (!store.selectedTableId) {
      return;
    }

    const payload: UpdateLabelColumnPayload = {
      tableId: store.selectedTableId,
      labelColumnId
    };

    await store.updateLabelColumn(payload);
  }

  /**
   * ドラッグ後のカラム順を保存します。
   *
   * @param orderedColumns 並び替え後のカラム一覧
   */
  async function reorderColumns(orderedColumns: AppColumn[]) {
    if (!store.selectedTableId) {
      return;
    }

    await store.reorderColumns({
      tableId: store.selectedTableId,
      orderedColumnIds: orderedColumns.map((column) => column.id)
    });
  }

  /**
   * 単一選択グループに空の選択肢行を追加します。
   */
  function addOptionRow() {
    optionGroupForm.options.push({
      clientKey: createOptionClientKey(),
      optionNo: optionGroupForm.options.length + 1,
      sortOrder: optionGroupForm.options.length + 1,
      label: ""
    });
    syncOptionOrdering();
  }

  /**
   * 指定位置の選択肢行を削除します。
   *
   * @param index 削除する選択肢行の位置
   */
  function removeOptionRow(index: number) {
    if (optionGroupForm.options.length <= 1) {
      return;
    }

    optionGroupForm.options.splice(index, 1);
    syncOptionOrdering();
  }

  /**
   * 選択肢の番号と表示順を配列順にそろえます。
   */
  function syncOptionOrdering() {
    optionGroupForm.options.forEach((option, index) => {
      option.optionNo = index + 1;
      option.sortOrder = index + 1;
    });
  }

  /**
   * 単一選択グループフォームを新規作成状態へ戻します。
   */
  function resetOptionGroupForm() {
    selectedOptionGroupId.value = null;
    Object.assign(optionGroupForm, {
      id: undefined,
      name: "",
      description: "",
      options: createDefaultOptionRows()
    } satisfies SaveOptionGroupPayload);
  }

  /**
   * 既存の単一選択グループをフォームへ読み込み、編集状態へ切り替えます。
   *
   * @param group 編集対象の単一選択グループ
   */
  function startEditOptionGroup(group: SelectOptionGroup) {
    selectedOptionGroupId.value = group.id;
    Object.assign(optionGroupForm, {
      id: group.id,
      name: group.name,
      description: group.description ?? "",
      options:
        group.options.length > 0
          ? group.options.map((option) => ({
              clientKey: createOptionClientKey(),
              id: option.id,
              optionNo: option.optionNo,
              sortOrder: option.sortOrder,
              label: option.label
            }))
          : createDefaultOptionRows()
    } satisfies SaveOptionGroupPayload);
    syncOptionOrdering();
  }

  /**
   * 選択肢グループを保存し、空フォームへ戻します。
   */
  async function submitOptionGroup() {
    await store.saveOptionGroup({
      ...optionGroupForm,
      // 空ラベルは保存対象から除外し、バックエンドへ不要な項目を送らないようにします。
      options: optionGroupForm.options
        .filter((option) => option.label.trim())
        .map(({ clientKey: _clientKey, ...option }) => option)
    });

    resetOptionGroupForm();
  }

  /**
   * レコードフォームを保存します。
   */
  async function submitRecord() {
    if (!store.selectedTableId) {
      return false;
    }

    const missingColumns =
      store.currentTable?.columns.filter(
        (column) =>
          column.columnName !== "id" &&
          column.isRequired &&
          isRequiredValueEmpty(recordValues.value[column.columnName])
      ) ?? [];
    if (missingColumns.length > 0) {
      return false;
    }

    const payload: SaveRecordPayload = {
      tableId: store.selectedTableId,
      recordId: editingRecordId.value,
      values: recordValues.value
    };

    await store.saveRecord(payload);
    resetRecordForm();
    return true;
  }

  /**
   * 指定レコードを削除し、編集中状態も解除します。
   *
   * @param recordId 削除対象レコードID
   */
  async function deleteRecord(recordId: number) {
    if (!store.selectedTableId) {
      return;
    }

    await store.deleteRecord({
      tableId: store.selectedTableId,
      recordId
    });
    resetRecordForm();
  }

  async function deleteTable(tableId: number) {
    await store.deleteTable({ tableId });
    resetRecordForm();
  }

  /**
   * 選択された保存先へ、指定テーブルのCSVを書き出します。
   */
  async function exportTableCsv(tableId: number, outputPath: string) {
    await store.exportTableCsv({ tableId, outputPath });
  }

  /**
   * 選択されたCSVを取り込み、成功後はレコード編集フォームを閉じます。
   */
  async function importTableCsv(
    tableId: number,
    inputPath: string,
    mode: ImportTableCsvMode,
    columnMapping: ImportColumnMappingPayload[]
  ) {
    const result = await store.importTableCsv({
      tableId,
      inputPath,
      mode,
      columnMapping
    });
    resetRecordForm();
    return result;
  }

  async function inspectCsvImport(tableId: number, inputPath: string) {
    return await store.inspectCsvImport({ tableId, inputPath });
  }

  async function previewCsvImport(
    tableId: number,
    inputPath: string,
    mode: ImportTableCsvMode,
    columnMapping: ImportColumnMappingPayload[]
  ) {
    return await store.previewCsvImport({
      tableId,
      inputPath,
      mode,
      columnMapping
    });
  }

  async function inspectExcelTables(tableId: number, inputPath: string) {
    return await store.inspectExcelTables({ tableId, inputPath });
  }

  async function previewExcelTableImport(
    tableId: number,
    inputPath: string,
    excelTableName: string,
    mode: ImportTableCsvMode,
    columnMapping: ImportColumnMappingPayload[]
  ) {
    return await store.previewExcelTableImport({
      tableId,
      inputPath,
      excelTableName,
      mode,
      columnMapping
    });
  }

  async function importExcelTable(
    tableId: number,
    inputPath: string,
    excelTableName: string,
    mode: ImportTableCsvMode,
    columnMapping: ImportColumnMappingPayload[]
  ) {
    const result = await store.importExcelTable({
      tableId,
      inputPath,
      excelTableName,
      mode,
      columnMapping
    });
    resetRecordForm();
    return result;
  }

  onMounted(() => {
    // 初回表示時に必要な初期データを読み込みます。
    void store.initialize();
  });

  return {
    addOptionRow,
    columnForm,
    deleteColumn,
    deleteRecord,
    deleteTable,
    exportTableCsv,
    inspectCsvImport,
    inspectExcelTables,
    previewCsvImport,
    previewExcelTableImport,
    importExcelTable,
    importTableCsv,
    editingRecordId,
    fieldTypes,
    fieldTypeLabel,
    fieldTypeMeta,
    inputType,
    optionGroupForm,
    recordValues,
    referenceChoices,
    reorderColumns,
    removeOptionRow,
    resetOptionGroupForm,
    resetRecordForm,
    selectedTable,
    selectedOptionGroupId,
    startEditOptionGroup,
    startEditRecord,
    store,
    submitColumn,
    submitOptionGroup,
    submitRecord,
    submitTable,
    syncOptionOrdering,
    tableForm,
    updateColumn,
    updateLabelColumn
  };
}
