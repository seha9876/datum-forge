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
  ImportTableCsvPayload,
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
    /**
     * アプリ全体で必要な初期データを読み込みます。
     */
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
    /**
     * 指定テーブルの詳細を読み込み、必要な参照候補もキャッシュします。
     *
     * @param tableId 読み込むテーブルID
     */
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
    /**
     * テーブル作成後に一覧と現在テーブルを更新します。
     *
     * @param payload 作成するテーブル情報
     */
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
    /**
     * CSVを取り込んだ後、サイドバーと一覧の表示を最新状態へ戻します。
     */
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
    /**
     * カラム追加後に最新状態を再読み込みします。
     *
     * @param payload 追加するカラム情報
     */
    async addColumn(payload: AddColumnPayload) {
      await api.addColumn(payload);
      await this.initialize();
      await this.loadTable(payload.tableId);
    },
    /**
     * カラム削除後に最新状態を再読み込みします。
     *
     * @param payload 削除対象
     */
    async deleteColumn(payload: DeleteColumnPayload) {
      await api.deleteColumn(payload);
      await this.initialize();
      await this.loadTable(payload.tableId);
    },
    /**
     * カラム更新後に最新状態を再読み込みします。
     *
     * @param payload 更新内容
     */
    async updateColumn(payload: UpdateColumnPayload) {
      await api.updateColumn(payload);
      await this.initialize();
      await this.loadTable(payload.tableId);
    },
    /**
     * 主表示カラム更新後に最新状態を再読み込みします。
     *
     * @param payload 主表示カラム設定
     */
    async updateLabelColumn(payload: UpdateLabelColumnPayload) {
      await api.updateLabelColumn(payload);
      await this.initialize();
      await this.loadTable(payload.tableId);
    },
    /**
     * カラム並び替え後に最新状態を再読み込みします。
     *
     * @param payload 並び替え結果
     */
    async reorderColumns(payload: ReorderColumnsPayload) {
      await api.reorderColumns(payload);
      await this.initialize();
      await this.loadTable(payload.tableId);
    },
    /**
     * 単一選択グループ保存後に初期データを更新します。
     *
     * @param payload 保存するグループ情報
     */
    async saveOptionGroup(payload: SaveOptionGroupPayload) {
      await api.saveOptionGroup(payload);
      await this.initialize();
    },
    /**
     * レコード保存後に現在テーブルを再読み込みします。
     *
     * @param payload 保存するレコード情報
     */
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
    /**
     * レコード削除後に現在テーブルを再読み込みします。
     *
     * @param payload 削除対象
     */
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
