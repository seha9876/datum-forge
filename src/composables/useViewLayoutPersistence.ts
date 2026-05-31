import { api } from "../api";

import { toLayoutPayloadItems } from "./useViewNavigation.helpers";

import type {
  ViewLayoutCardColumnBinding,
  ViewLayoutCardItem,
  ViewLayoutTemplateCard
} from "../types";
import type { ViewNavigationState } from "./useViewNavigationState";

export function useViewLayoutPersistence(state: ViewNavigationState) {
  let layoutSaveTimer: ReturnType<typeof globalThis.setTimeout> | null = null;

  function saveRecordLayoutOverrides(items: ViewLayoutCardItem[]) {
    const selected = state.selectedItem.value;
    const templateId = state.activeLayoutTemplateId.value;
    if (selected?.type !== "tableRecord" || !templateId) {
      return;
    }

    state.layoutCardItems.value = items;
    state.layoutSaving.value = true;

    if (layoutSaveTimer) {
      globalThis.clearTimeout(layoutSaveTimer);
    }

    layoutSaveTimer = globalThis.setTimeout(() => {
      void persistRecordLayoutOverrides(
        templateId,
        selected.tableId,
        selected.recordId,
        items
      );
    }, 240);
  }

  async function persistRecordLayoutOverrides(
    templateId: number,
    tableId: number,
    recordId: number,
    items: ViewLayoutCardItem[]
  ) {
    try {
      await api.saveViewLayoutCardOverrides({
        templateId,
        tableId,
        recordId,
        items: toLayoutPayloadItems(items)
      });
      await reloadSelectedLayout();
    } catch (saveError) {
      state.error.value =
        saveError instanceof Error ? saveError.message : String(saveError);
    } finally {
      state.layoutSaving.value = false;
    }
  }

  function saveLayoutTemplateCards(cards: ViewLayoutTemplateCard[]) {
    const template = state.selectedLayoutTemplate.value;
    if (!template) {
      return;
    }

    state.templateLayoutCards.value = cards;
    state.layoutSaving.value = true;

    if (layoutSaveTimer) {
      globalThis.clearTimeout(layoutSaveTimer);
    }

    layoutSaveTimer = globalThis.setTimeout(() => {
      void persistLayoutTemplateCards(template.id, cards);
    }, 240);
  }

  async function persistLayoutTemplateCards(
    templateId: number,
    cards: ViewLayoutTemplateCard[]
  ) {
    try {
      await api.saveViewLayoutTemplateCards({
        templateId,
        cards
      });
      state.templateLayoutCards.value = await api.getViewLayoutTemplateCards({
        templateId
      });
    } catch (saveError) {
      state.error.value =
        saveError instanceof Error ? saveError.message : String(saveError);
    } finally {
      state.layoutSaving.value = false;
    }
  }

  async function resetCardLayoutOverride(cardId: number) {
    const selected = state.selectedItem.value;
    const templateId = state.activeLayoutTemplateId.value;
    if (selected?.type !== "tableRecord" || !templateId) {
      return;
    }

    state.error.value = "";
    await api.resetViewLayoutCardOverride({
      templateId,
      tableId: selected.tableId,
      recordId: selected.recordId,
      cardId
    });
    await reloadSelectedLayout();
  }

  async function resetRecordLayoutOverrides() {
    const selected = state.selectedItem.value;
    const templateId = state.activeLayoutTemplateId.value;
    if (selected?.type !== "tableRecord" || !templateId) {
      return;
    }

    state.error.value = "";
    await api.resetViewLayoutCardOverrides({
      templateId,
      tableId: selected.tableId,
      recordId: selected.recordId
    });
    await reloadSelectedLayout();
  }

  async function saveLayoutCardColumnBindings(
    bindings: ViewLayoutCardColumnBinding[]
  ) {
    const selected = state.selectedItem.value;
    const templateId = state.activeLayoutTemplateId.value;
    if (selected?.type !== "tableRecord" || !templateId) {
      return;
    }

    state.error.value = "";
    state.layoutSaving.value = true;
    try {
      await api.saveViewLayoutCardColumnBindings({
        templateId,
        tableId: selected.tableId,
        bindings
      });
      await reloadSelectedLayout();
    } catch (saveError) {
      state.error.value =
        saveError instanceof Error ? saveError.message : String(saveError);
    } finally {
      state.layoutSaving.value = false;
    }
  }

  async function reloadSelectedLayout() {
    const selected = state.selectedItem.value;
    if (selected?.type !== "tableRecord") {
      return;
    }

    const resolvedLayout = await api.getResolvedViewFieldLayout({
      tableId: selected.tableId,
      recordId: selected.recordId,
      folderId: selected.folderId ?? null,
      folderRecordId: selected.folderRecordId ?? null
    });
    state.layoutCardItems.value = resolvedLayout.items;
    state.activeLayoutTemplateId.value = resolvedLayout.activeTemplateId;
    state.activeLayoutTemplateName.value = resolvedLayout.activeTemplateName;
  }

  return {
    reloadSelectedLayout,
    resetCardLayoutOverride,
    resetRecordLayoutOverrides,
    saveLayoutCardColumnBindings,
    saveLayoutTemplateCards,
    saveRecordLayoutOverrides
  };
}

export type ViewLayoutPersistenceActions = ReturnType<
  typeof useViewLayoutPersistence
>;
