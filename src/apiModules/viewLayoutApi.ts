import { invoke } from "@tauri-apps/api/core";

import type {
  AssignViewLayoutFolderTemplatePayload,
  AssignViewLayoutRecordTemplatePayload,
  ClearViewLayoutRecordTemplatePayload,
  CreateViewLayoutTemplatePayload,
  DeleteViewLayoutTemplatePayload,
  DuplicateViewLayoutTemplatePayload,
  FolderViewLayoutTemplates,
  GetResolvedViewFieldLayoutPayload,
  GetViewLayoutTemplateCardsPayload,
  ListViewLayoutCardColumnBindingsPayload,
  ListViewLayoutTemplatesForFolderPayload,
  RenameViewLayoutTemplatePayload,
  ResetViewLayoutCardOverridePayload,
  ResetViewLayoutCardOverridesPayload,
  ResolvedViewFieldLayout,
  SaveViewLayoutCardColumnBindingsPayload,
  SaveViewLayoutCardOverridesPayload,
  SaveViewLayoutTemplateCardsPayload,
  ViewLayoutCardColumnBinding,
  ViewLayoutTemplate,
  ViewLayoutTemplateCard,
  ViewNavFolderRecord
} from "../types";

export const viewLayoutApi = {
  listAllFolderLayoutTemplates: () =>
    invoke<ViewLayoutTemplate[]>("list_all_folder_layout_templates"),
  listViewLayoutTemplatesForFolder: (
    payload: ListViewLayoutTemplatesForFolderPayload
  ) =>
    invoke<FolderViewLayoutTemplates>("list_view_layout_templates_for_folder", {
      payload
    }),
  createViewLayoutTemplate: (payload: CreateViewLayoutTemplatePayload) =>
    invoke<ViewLayoutTemplate>("create_view_layout_template", { payload }),
  renameViewLayoutTemplate: (payload: RenameViewLayoutTemplatePayload) =>
    invoke<ViewLayoutTemplate>("rename_view_layout_template", { payload }),
  duplicateViewLayoutTemplate: (payload: DuplicateViewLayoutTemplatePayload) =>
    invoke<ViewLayoutTemplate>("duplicate_view_layout_template", { payload }),
  deleteViewLayoutTemplate: (payload: DeleteViewLayoutTemplatePayload) =>
    invoke<void>("delete_view_layout_template", { payload }),
  assignViewLayoutFolderTemplate: (
    payload: AssignViewLayoutFolderTemplatePayload
  ) =>
    invoke<ViewLayoutTemplate>("assign_view_layout_folder_template", {
      payload
    }),
  assignViewLayoutRecordTemplate: (
    payload: AssignViewLayoutRecordTemplatePayload
  ) =>
    invoke<ViewNavFolderRecord>("assign_view_layout_record_template", {
      payload
    }),
  clearViewLayoutRecordTemplate: (
    payload: ClearViewLayoutRecordTemplatePayload
  ) =>
    invoke<ViewNavFolderRecord>("clear_view_layout_record_template", {
      payload
    }),
  getResolvedViewFieldLayout: (payload: GetResolvedViewFieldLayoutPayload) =>
    invoke<ResolvedViewFieldLayout>("get_resolved_view_field_layout", {
      payload
    }),
  getViewLayoutTemplateCards: (payload: GetViewLayoutTemplateCardsPayload) =>
    invoke<ViewLayoutTemplateCard[]>("get_view_layout_template_cards", {
      payload
    }),
  listViewLayoutCardColumnBindings: (
    payload: ListViewLayoutCardColumnBindingsPayload
  ) =>
    invoke<ViewLayoutCardColumnBinding[]>(
      "list_view_layout_card_column_bindings",
      { payload }
    ),
  saveViewLayoutTemplateCards: (payload: SaveViewLayoutTemplateCardsPayload) =>
    invoke<void>("save_view_layout_template_cards", { payload }),
  saveViewLayoutCardColumnBindings: (
    payload: SaveViewLayoutCardColumnBindingsPayload
  ) => invoke<void>("save_view_layout_card_column_bindings", { payload }),
  saveViewLayoutCardOverrides: (payload: SaveViewLayoutCardOverridesPayload) =>
    invoke<void>("save_view_layout_card_overrides", { payload }),
  resetViewLayoutCardOverride: (payload: ResetViewLayoutCardOverridePayload) =>
    invoke<void>("reset_view_layout_card_override", { payload }),
  resetViewLayoutCardOverrides: (
    payload: ResetViewLayoutCardOverridesPayload
  ) => invoke<void>("reset_view_layout_card_overrides", { payload })
};
