import { invoke } from "@tauri-apps/api/core";

import type {
  AddColumnPayload,
  CreateTablePayload,
  DeleteColumnPayload,
  DeleteRecordPayload,
  DeleteTablePayload,
  ReferenceChoice,
  ReorderColumnsPayload,
  SaveOptionGroupPayload,
  SaveRecordPayload,
  TableDetail,
  UpdateColumnPayload,
  UpdateLabelColumnPayload
} from "../types";

export const tableApi = {
  getTableDetail: (tableId: number) =>
    invoke<TableDetail>("get_table_detail", { tableId }),
  createTable: (payload: CreateTablePayload) =>
    invoke<number>("create_table", { payload }),
  deleteTable: (payload: DeleteTablePayload) =>
    invoke<void>("delete_table", { payload }),
  addColumn: (payload: AddColumnPayload) =>
    invoke<void>("add_column", { payload }),
  deleteColumn: (payload: DeleteColumnPayload) =>
    invoke<void>("delete_column", { payload }),
  updateColumn: (payload: UpdateColumnPayload) =>
    invoke<void>("update_column", { payload }),
  updateLabelColumn: (payload: UpdateLabelColumnPayload) =>
    invoke<void>("update_label_column", { payload }),
  reorderColumns: (payload: ReorderColumnsPayload) =>
    invoke<void>("reorder_columns", { payload }),
  saveOptionGroup: (payload: SaveOptionGroupPayload) =>
    invoke<number>("save_option_group", { payload }),
  saveRecord: (payload: SaveRecordPayload) =>
    invoke<void>("save_record", { payload }),
  deleteRecord: (payload: DeleteRecordPayload) =>
    invoke<void>("delete_record", { payload }),
  getReferenceChoices: (tableId: number) =>
    invoke<ReferenceChoice[]>("get_reference_choices", { tableId })
};
