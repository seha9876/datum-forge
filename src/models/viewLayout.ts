export type ViewLayoutAutoHeightBehavior = "scaleToFit" | "scroll" | "truncate";

export interface ViewLayoutCardItem {
  tableId: number;
  cardId: number;
  columns: ViewLayoutCardColumnBinding[];
  slots: ViewLayoutTemplateCardSlot[];
  presetId?: string | null;
  label?: string | null;
  x: number;
  y: number;
  width: number;
  height: number;
  visible: boolean;
  backgroundColor?: string | null;
  textColor?: string | null;
  fontSize?: number | null;
  textDirection?: "horizontal" | "vertical" | null;
  fontWeight?: "normal" | "bold" | null;
  textAlign?: "left" | "center" | "right" | null;
  padding?: number | null;
  paddingTop?: number | null;
  paddingRight?: number | null;
  paddingBottom?: number | null;
  paddingLeft?: number | null;
  borderRadius?: number | null;
  showLabel?: boolean | null;
  autoHeightEnabled: boolean;
  pushDownSiblings: boolean;
  maxAutoHeight?: number | null;
  maxAutoHeightBehavior: ViewLayoutAutoHeightBehavior;
  hasOverride: boolean;
}

export interface ViewLayoutCardColumnBinding {
  cardId: number;
  columnId: number;
  sortOrder: number;
}

export interface ViewLayoutTemplateCardSlot {
  slotId: number;
  sortOrder: number;
}

export interface ViewLayoutTemplate {
  id: number;
  name: string;
  scopeType: "folder";
  folderId: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface ViewLayoutTemplateCard {
  cardId: number;
  slots: ViewLayoutTemplateCardSlot[];
  presetId?: string | null;
  x: number;
  y: number;
  width: number;
  height: number;
  visible: boolean;
  label: string | null;
  backgroundColor?: string | null;
  textColor?: string | null;
  fontSize?: number | null;
  textDirection?: "horizontal" | "vertical" | null;
  fontWeight?: "normal" | "bold" | null;
  textAlign?: "left" | "center" | "right" | null;
  padding?: number | null;
  paddingTop?: number | null;
  paddingRight?: number | null;
  paddingBottom?: number | null;
  paddingLeft?: number | null;
  borderRadius?: number | null;
  showLabel?: boolean | null;
  autoHeightEnabled: boolean;
  pushDownSiblings: boolean;
  maxAutoHeight?: number | null;
  maxAutoHeightBehavior: ViewLayoutAutoHeightBehavior;
}

export interface ResolvedViewFieldLayout {
  templates: ViewLayoutTemplate[];
  activeTemplateId: number | null;
  activeTemplateName: string | null;
  items: ViewLayoutCardItem[];
}

export interface FolderViewLayoutTemplates {
  templates: ViewLayoutTemplate[];
  activeTemplateId: number | null;
}

export interface SaveViewLayoutCardOverridesPayload {
  templateId: number;
  tableId: number;
  recordId: number;
  items: Array<{
    cardId: number;
    presetId?: string | null;
    x: number;
    y: number;
    width: number;
    height: number;
    visible: boolean;
    backgroundColor?: string | null;
    textColor?: string | null;
    fontSize?: number | null;
    textDirection?: "horizontal" | "vertical" | null;
    fontWeight?: "normal" | "bold" | null;
    textAlign?: "left" | "center" | "right" | null;
    padding?: number | null;
    paddingTop?: number | null;
    paddingRight?: number | null;
    paddingBottom?: number | null;
    paddingLeft?: number | null;
    borderRadius?: number | null;
    showLabel?: boolean | null;
  }>;
}

export interface CreateViewLayoutTemplatePayload {
  name: string;
  scopeType?: "folder";
  folderId?: number | null;
}

export interface RenameViewLayoutTemplatePayload {
  templateId: number;
  name: string;
}

export interface DuplicateViewLayoutTemplatePayload {
  templateId: number;
  name: string;
}

export interface DeleteViewLayoutTemplatePayload {
  templateId: number;
}

export interface ListViewLayoutTemplatesForFolderPayload {
  folderId: number;
}

export interface AssignViewLayoutFolderTemplatePayload {
  folderId: number;
  templateId: number;
}

export interface GetResolvedViewFieldLayoutPayload {
  tableId: number;
  recordId: number;
  folderId?: number | null;
  folderRecordId?: number | null;
}

export interface AssignViewLayoutRecordTemplatePayload {
  folderRecordId: number;
  templateId: number;
}

export interface ClearViewLayoutRecordTemplatePayload {
  folderRecordId: number;
}

export interface GetViewLayoutTemplateCardsPayload {
  templateId: number;
}

export interface ListViewLayoutCardColumnBindingsPayload {
  templateId: number;
  tableId: number;
}

export interface SaveViewLayoutTemplateCardsPayload {
  templateId: number;
  cards: ViewLayoutTemplateCard[];
}

export interface SaveViewLayoutCardColumnBindingsPayload {
  templateId: number;
  tableId: number;
  bindings: ViewLayoutCardColumnBinding[];
}

export interface ResetViewLayoutCardOverridePayload {
  templateId: number;
  tableId: number;
  recordId: number;
  cardId: number;
}

export interface ResetViewLayoutCardOverridesPayload {
  templateId: number;
  tableId: number;
  recordId: number;
}
