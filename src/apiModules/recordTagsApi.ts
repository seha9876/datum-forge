import { invoke } from "@tauri-apps/api/core";

import type {
  AttachRecordTagPayload,
  CreateAndAttachRecordTagPayload,
  DeleteRecordTagPayload,
  DetachRecordTagPayload,
  RecordTag,
  RecordTagBundle,
  RecordTagGroup,
  RecordTagGroupLinkPayload,
  SaveRecordTagGroupPayload,
  SaveRecordTagPayload
} from "../types";

export const recordTagsApi = {
  listRecordTags: () => invoke<RecordTagBundle>("list_record_tags"),
  listRecordTagsForRecord: (tableId: number, recordId: number) =>
    invoke<RecordTag[]>("list_record_tags_for_record", { tableId, recordId }),
  saveRecordTagGroup: (payload: SaveRecordTagGroupPayload) =>
    invoke<RecordTagGroup>("save_record_tag_group", { payload }),
  saveRecordTag: (payload: SaveRecordTagPayload) =>
    invoke<RecordTag>("save_record_tag", { payload }),
  deleteRecordTag: (payload: DeleteRecordTagPayload) =>
    invoke<void>("delete_record_tag", { payload }),
  attachRecordTagGroup: (payload: RecordTagGroupLinkPayload) =>
    invoke<RecordTag>("attach_record_tag_group", { payload }),
  detachRecordTagGroup: (payload: RecordTagGroupLinkPayload) =>
    invoke<RecordTag>("detach_record_tag_group", { payload }),
  attachRecordTag: (payload: AttachRecordTagPayload) =>
    invoke<RecordTag>("attach_record_tag", { payload }),
  createAndAttachRecordTag: (payload: CreateAndAttachRecordTagPayload) =>
    invoke<RecordTag>("create_and_attach_record_tag", { payload }),
  detachRecordTag: (payload: DetachRecordTagPayload) =>
    invoke<void>("detach_record_tag", { payload })
};
