import { defineStore } from "pinia";

import { api } from "../api";

import type {
  AddColumnPayload,
  AppBootstrap,
  AppSettings,
  CreateDatabasePayload,
  CreateTablePayload,
  DeleteColumnPayload,
  DeleteRecordPayload,
  DeleteTablePayload,
  ExportTableCsvPayload,
  ImportExcelTablePayload,
  ImportTableCsvPayload,
  InspectCsvImportPayload,
  InspectExcelTablesPayload,
  PreviewCsvImportPayload,
  PreviewExcelTableImportPayload,
  ReorderColumnsPayload,
  ReferenceChoice,
  SaveOptionGroupPayload,
  SaveRecordPayload,
  StartupDbStatus,
  TableDetail,
  UpdateLabelColumnPayload,
  UpdateNotificationSettingsPayload,
  UpdateColumnPayload
} from "../types";

export const useAppStore = defineStore("app", {
  state: () => ({
    bootstrap: null as AppBootstrap | null,
    settings: null as AppSettings | null,
    startupDbStatus: null as StartupDbStatus | null,
    selectedTableId: null as number | null,
    currentTable: null as TableDetail | null,
    references: {} as Record<number, ReferenceChoice[]>,
    loading: false,
    error: ""
  }),
  actions: {
    resetWorkspaceState() {
      this.bootstrap = null;
      this.settings = null;
      this.selectedTableId = null;
      this.currentTable = null;
      this.references = {};
    },
    /** アプリ起動時に必要な初期データを読み込みます。 */
    async initialize() {
      this.loading = true;
      this.error = "";
      try {
        this.startupDbStatus = await api.getStartupDatabaseStatus();
        if (this.startupDbStatus.state !== "ready") {
          this.resetWorkspaceState();
          return;
        }

        this.bootstrap = await api.bootstrap();
        this.settings = await api.getAppSettings();
        // 初回起動時は先頭テーブルを自動選択し、すぐ操作できる状態にします。
        if (!this.selectedTableId && this.bootstrap.tables.length > 0) {
          this.selectedTableId = this.bootstrap.tables[0].id;
        }
        if (this.selectedTableId) {
          await this.loadTable(this.selectedTableId);
        }
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
      } finally {
        this.loading = false;
      }
    },
    /** 指定テーブルの詳細を読み込み、参照型カラムの候補もキャッシュします。 */
    async loadTable(tableId: number) {
      this.selectedTableId = tableId;
      this.currentTable = await api.getTableDetail(tableId);
      const refColumns = this.currentTable.columns.filter(
        (column) => column.refTableId
      );
      for (const column of refColumns) {
        if (column.refTableId && !this.references[column.refTableId]) {
          this.references[column.refTableId] = await api.getReferenceChoices(
            column.refTableId
          );
        }
      }
    },
    /** テーブル作成後に一覧と現在テーブルを最新状態へ更新します。 */
    async createTable(payload: CreateTablePayload) {
      const tableId = await api.createTable(payload);
      await this.initialize();
      await this.loadTable(tableId);
    },
    async deleteTable(payload: DeleteTablePayload) {
      this.error = "";
      try {
        const wasSelected = this.selectedTableId === payload.tableId;
        await api.deleteTable(payload);
        if (wasSelected) {
          this.selectedTableId = null;
          this.currentTable = null;
        }
        this.references = {};
        await this.initialize();
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      }
    },
    async exportTableCsv(payload: ExportTableCsvPayload) {
      this.error = "";
      try {
        await api.exportTableCsv(payload);
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      }
    },
    /** CSV取り込み後、サイドバーと一覧の表示を最新状態へ戻します。 */
    async importTableCsv(payload: ImportTableCsvPayload) {
      this.error = "";
      try {
        const result = await api.importTableCsv(payload);
        await this.initialize();
        await this.loadTable(payload.tableId);
        return result;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      }
    },
    async inspectCsvImport(payload: InspectCsvImportPayload) {
      this.error = "";
      try {
        return await api.inspectCsvImport(payload);
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      }
    },
    async previewCsvImport(payload: PreviewCsvImportPayload) {
      this.error = "";
      try {
        return await api.previewCsvImport(payload);
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      }
    },
    async inspectExcelTables(payload: InspectExcelTablesPayload) {
      this.error = "";
      try {
        return await api.inspectExcelTables(payload);
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      }
    },
    async previewExcelTableImport(payload: PreviewExcelTableImportPayload) {
      this.error = "";
      try {
        return await api.previewExcelTableImport(payload);
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      }
    },
    async importExcelTable(payload: ImportExcelTablePayload) {
      this.error = "";
      try {
        const result = await api.importExcelTable(payload);
        this.settings = await api.getAppSettings();
        await this.initialize();
        await this.loadTable(payload.tableId);
        return result;
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      }
    },
    /** カラム追加後にテーブル定義とレコード表示を再読み込みします。 */
    async addColumn(payload: AddColumnPayload) {
      this.error = "";
      try {
        await api.addColumn(payload);
        await this.initialize();
        await this.loadTable(payload.tableId);
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      }
    },
    /** カラム削除後にテーブル定義とレコード表示を再読み込みします。 */
    async deleteColumn(payload: DeleteColumnPayload) {
      await api.deleteColumn(payload);
      await this.initialize();
      await this.loadTable(payload.tableId);
    },
    /** カラム更新後にテーブル定義とレコード表示を再読み込みします。 */
    async updateColumn(payload: UpdateColumnPayload) {
      await api.updateColumn(payload);
      await this.initialize();
      await this.loadTable(payload.tableId);
    },
    /** 主表示カラム更新後に関連する表示を再読み込みします。 */
    async updateLabelColumn(payload: UpdateLabelColumnPayload) {
      await api.updateLabelColumn(payload);
      await this.initialize();
      await this.loadTable(payload.tableId);
    },
    /** カラム並び替え後にテーブル定義を再読み込みします。 */
    async reorderColumns(payload: ReorderColumnsPayload) {
      await api.reorderColumns(payload);
      await this.initialize();
      await this.loadTable(payload.tableId);
    },
    /** 選択肢グループ保存後に初期データを更新します。 */
    async saveOptionGroup(payload: SaveOptionGroupPayload) {
      await api.saveOptionGroup(payload);
      await this.initialize();
    },
    /** レコード保存後に現在テーブルを再読み込みします。 */
    async saveRecord(payload: SaveRecordPayload) {
      this.error = "";
      try {
        await api.saveRecord(payload);
        if (this.selectedTableId) {
          await this.loadTable(this.selectedTableId);
        }
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      }
    },
    /** レコード削除後に現在テーブルを再読み込みします。 */
    async deleteRecord(payload: DeleteRecordPayload) {
      await api.deleteRecord(payload);
      if (this.selectedTableId) {
        await this.loadTable(this.selectedTableId);
      }
    },
    async updateRecordIdVisibility(show: boolean) {
      this.loading = true;
      this.error = "";
      try {
        this.settings = await api.updateRecordIdVisibility(show);
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async updateNotificationSettings(
      payload: UpdateNotificationSettingsPayload
    ) {
      this.loading = true;
      this.error = "";
      try {
        this.settings = await api.updateNotificationSettings(payload);
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async createDatabaseFile(payload: CreateDatabasePayload) {
      this.loading = true;
      this.error = "";
      try {
        this.settings = await api.createDatabaseFile(payload);
        this.resetWorkspaceState();
        await this.initialize();
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async setupOpenDatabaseFile(dbFile: string) {
      this.loading = true;
      this.error = "";
      try {
        this.settings = await api.setupOpenDatabaseFile(dbFile);
        this.resetWorkspaceState();
        await this.initialize();
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async updateDatabaseDirectory(dbDirectory: string) {
      this.loading = true;
      this.error = "";
      try {
        this.settings = await api.updateDatabaseDirectory(dbDirectory);
        this.selectedTableId = null;
        this.currentTable = null;
        this.references = {};
        await this.initialize();
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async renameDatabaseFile(dbFileName: string) {
      this.loading = true;
      this.error = "";
      try {
        this.settings = await api.renameDatabaseFile(dbFileName);
        this.selectedTableId = null;
        this.currentTable = null;
        this.references = {};
        await this.initialize();
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    },
    async openDatabaseFile(dbFile: string) {
      this.loading = true;
      this.error = "";
      try {
        this.settings = await api.openDatabaseFile(dbFile);
        this.selectedTableId = null;
        this.currentTable = null;
        this.references = {};
        await this.initialize();
      } catch (error) {
        this.error = error instanceof Error ? error.message : String(error);
        throw error;
      } finally {
        this.loading = false;
      }
    }
  }
});
