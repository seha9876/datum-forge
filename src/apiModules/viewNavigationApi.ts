import { invoke } from "@tauri-apps/api/core";

import type {
  AddViewNavFolderRecordsPayload,
  CreateViewNavFolderPayload,
  DeleteViewNavFolderPayload,
  RemoveViewNavFolderRecordPayload,
  ReorderViewNavFolderRecordsPayload,
  ViewNavFolderRecord,
  ViewNavNode,
  ViewTableSection
} from "../types";

export const viewNavigationApi = {
  listViewNavNodes: () => invoke<ViewNavNode[]>("list_view_nav_nodes"),
  createViewNavFolder: (payload: CreateViewNavFolderPayload) =>
    invoke<ViewNavNode>("create_view_nav_folder", { payload }),
  deleteViewNavFolder: (payload: DeleteViewNavFolderPayload) =>
    invoke<void>("delete_view_nav_folder", { payload }),
  listViewNavFolderRecords: () =>
    invoke<ViewNavFolderRecord[]>("list_view_nav_folder_records"),
  addViewNavFolderRecords: (payload: AddViewNavFolderRecordsPayload) =>
    invoke<ViewNavFolderRecord[]>("add_view_nav_folder_records", { payload }),
  removeViewNavFolderRecord: (payload: RemoveViewNavFolderRecordPayload) =>
    invoke<void>("remove_view_nav_folder_record", { payload }),
  reorderViewNavFolderRecords: (payload: ReorderViewNavFolderRecordsPayload) =>
    invoke<ViewNavFolderRecord[]>("reorder_view_nav_folder_records", {
      payload
    }),
  getViewTableSections: () =>
    invoke<ViewTableSection[]>("get_view_table_sections")
};
