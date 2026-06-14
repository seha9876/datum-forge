import { computed } from "vue";

import { getCardPresetBackgroundColorMode } from "./cardPresets/registry";
import {
  DEFAULT_CARD_STYLE,
  type InputLikeEvent,
  type LayoutStyleKey,
  type LayoutStyleValue
} from "./ViewFreeLayoutCanvas.helpers";

import type { ViewLayoutCardItem } from "../../../types";
import type { ComputedRef, CSSProperties } from "vue";

type UpdateLayouts = (items: ViewLayoutCardItem[], save: boolean) => void;

function isPaddingSideKey(
  key: LayoutStyleKey
): key is "paddingTop" | "paddingRight" | "paddingBottom" | "paddingLeft" {
  return (
    key === "paddingTop" ||
    key === "paddingRight" ||
    key === "paddingBottom" ||
    key === "paddingLeft"
  );
}

export function useFreeLayoutStyleEditing(
  selectedLayouts: ComputedRef<ViewLayoutCardItem[]>,
  updateLayouts: UpdateLayouts
) {
  function canOverrideBackgroundColor(layout: ViewLayoutCardItem) {
    return (
      getCardPresetBackgroundColorMode(layout.presetId) === "replace-preset"
    );
  }

  function layoutStyleValue(
    layout: ViewLayoutCardItem,
    key: LayoutStyleKey
  ): LayoutStyleValue {
    if (isPaddingSideKey(key)) {
      return layout[key] ?? layout.padding ?? DEFAULT_CARD_STYLE[key];
    }

    return layout[key] ?? DEFAULT_CARD_STYLE[key];
  }

  function sharedStyleValue(key: LayoutStyleKey): LayoutStyleValue | "" {
    const [firstLayout, ...restLayouts] = selectedLayouts.value;
    if (!firstLayout) {
      return "";
    }

    const firstValue = layoutStyleValue(firstLayout, key);
    const hasMixedValues = restLayouts.some(
      (layout) => layoutStyleValue(layout, key) !== firstValue
    );
    return hasMixedValues ? "" : firstValue;
  }

  const styleInspectorValues = computed(() => ({
    backgroundColor: sharedStyleValue("backgroundColor"),
    borderRadius: sharedStyleValue("borderRadius"),
    fontSize: sharedStyleValue("fontSize"),
    fontWeight: sharedStyleValue("fontWeight"),
    padding: sharedStyleValue("padding"),
    paddingTop: sharedStyleValue("paddingTop"),
    paddingRight: sharedStyleValue("paddingRight"),
    paddingBottom: sharedStyleValue("paddingBottom"),
    paddingLeft: sharedStyleValue("paddingLeft"),
    textAlign: sharedStyleValue("textAlign"),
    textColor: sharedStyleValue("textColor"),
    textDirection: sharedStyleValue("textDirection"),
    showLabel: sharedStyleValue("showLabel")
  }));

  function cardStyle(layout: ViewLayoutCardItem): CSSProperties {
    const backgroundColor = layoutStyleValue(layout, "backgroundColor");
    const textColor = layoutStyleValue(layout, "textColor");
    const canApplyBackgroundColor = canOverrideBackgroundColor(layout);
    return {
      "--card-background":
        canApplyBackgroundColor &&
        backgroundColor &&
        backgroundColor !== "transparent"
          ? String(backgroundColor)
          : undefined,
      "--card-border-radius": `${layoutStyleValue(layout, "borderRadius")}px`,
      "--card-font-size": `${layoutStyleValue(layout, "fontSize")}px`,
      "--card-font-weight": String(layoutStyleValue(layout, "fontWeight")),
      "--card-padding-bottom": `${layoutStyleValue(layout, "paddingBottom")}px`,
      "--card-padding-left": `${layoutStyleValue(layout, "paddingLeft")}px`,
      "--card-padding-right": `${layoutStyleValue(layout, "paddingRight")}px`,
      "--card-padding-top": `${layoutStyleValue(layout, "paddingTop")}px`,
      "--card-text-align": String(layoutStyleValue(layout, "textAlign")),
      "--card-text-color": textColor ? String(textColor) : undefined,
      backgroundColor:
        canApplyBackgroundColor && backgroundColor === "transparent"
          ? "transparent"
          : undefined,
      borderRadius: "var(--card-border-radius)",
      color: "var(--card-text-color, inherit)",
      fontSize: "var(--card-font-size)",
      fontWeight: "var(--card-font-weight)",
      height: `${layout.height}px`,
      paddingBottom: "var(--card-padding-bottom)",
      paddingLeft: "var(--card-padding-left)",
      paddingRight: "var(--card-padding-right)",
      paddingTop: "var(--card-padding-top)",
      textAlign: "var(--card-text-align)" as CSSProperties["textAlign"],
      transform: `translate(${layout.x}px, ${layout.y}px)`,
      width: `${layout.width}px`
    };
  }

  function cardContentStyle(layout: ViewLayoutCardItem): CSSProperties {
    return {
      writingMode:
        layoutStyleValue(layout, "textDirection") === "vertical"
          ? "vertical-rl"
          : "horizontal-tb"
    };
  }

  function styleInputValue(key: LayoutStyleKey) {
    const value = styleInspectorValues.value[key];
    return value == null || value === "" ? "" : String(value);
  }

  function themeColorInputValue(tokenName: string) {
    const documentElement = globalThis.document?.documentElement;
    if (!documentElement) {
      return "";
    }

    const tokenValue = globalThis
      .getComputedStyle(documentElement)
      .getPropertyValue(tokenName)
      .trim();
    const colorChannels = tokenValue
      .split(/[,\s]+/)
      .map((part) => Number(part))
      .filter((part) => Number.isFinite(part))
      .slice(0, 3);

    if (colorChannels.length !== 3) {
      return "";
    }

    return colorChannels
      .map((part) =>
        Math.max(0, Math.min(255, part)).toString(16).padStart(2, "0")
      )
      .join("")
      .replace(/^/, "#");
  }

  function backgroundColorInputValue() {
    const value = styleInputValue("backgroundColor");
    return value && value !== "transparent"
      ? value
      : themeColorInputValue("--v-theme-surface");
  }

  function isTransparentBackgroundSelected() {
    return styleInspectorValues.value.backgroundColor === "transparent";
  }

  function hasTransparentBackground(layout: ViewLayoutCardItem) {
    return (
      canOverrideBackgroundColor(layout) &&
      layoutStyleValue(layout, "backgroundColor") === "transparent"
    );
  }

  function applySelectedStyle(key: LayoutStyleKey, value: LayoutStyleValue) {
    if (selectedLayouts.value.length === 0) {
      return;
    }

    updateLayouts(
      selectedLayouts.value.map((layout) => ({
        ...layout,
        [key]: value
      })),
      true
    );
  }

  function applyBackgroundColorMode(mode: "color" | "transparent") {
    applySelectedStyle(
      "backgroundColor",
      mode === "transparent"
        ? "transparent"
        : backgroundColorInputValue() || null
    );
  }

  function styleNumberInputValue(key: LayoutStyleKey) {
    const value = styleInspectorValues.value[key];
    return typeof value === "number" ? value : "";
  }

  function styleBooleanInputValue(key: LayoutStyleKey) {
    const value = styleInspectorValues.value[key];
    if (key === "showLabel") {
      return value === "" ? true : value !== false;
    }
    return value === "bold";
  }

  function inputTarget(event: unknown) {
    return (event as InputLikeEvent).target;
  }

  function applyStyleFromInput(key: LayoutStyleKey, event: unknown) {
    const value = inputTarget(event)?.value ?? "";
    if (!value) {
      return;
    }

    applySelectedStyle(key, value);
  }

  function applyNumberStyleFromInput(key: LayoutStyleKey, event: unknown) {
    const value = Number(inputTarget(event)?.value);
    if (!Number.isFinite(value)) {
      return;
    }

    applySelectedStyle(key, value);
  }

  function applyFontWeightFromCheckbox(value: boolean | null) {
    applySelectedStyle("fontWeight", value ? "bold" : "normal");
  }

  function applyShowLabelFromCheckbox(value: boolean | null) {
    applySelectedStyle("showLabel", value ?? true);
  }

  function resetSelectedStyle() {
    if (selectedLayouts.value.length === 0) {
      return;
    }

    updateLayouts(
      selectedLayouts.value.map((layout) => ({
        ...layout,
        backgroundColor: null,
        borderRadius: null,
        fontSize: null,
        fontWeight: null,
        padding: null,
        paddingTop: null,
        paddingRight: null,
        paddingBottom: null,
        paddingLeft: null,
        textAlign: null,
        textColor: null,
        textDirection: null,
        showLabel: null
      })),
      true
    );
  }

  return {
    applyBackgroundColorMode,
    applyFontWeightFromCheckbox,
    applyNumberStyleFromInput,
    applySelectedStyle,
    applyShowLabelFromCheckbox,
    applyStyleFromInput,
    backgroundColorInputValue,
    cardContentStyle,
    cardStyle,
    hasTransparentBackground,
    isTransparentBackgroundSelected,
    layoutStyleValue,
    canOverrideBackgroundColor,
    resetSelectedStyle,
    styleBooleanInputValue,
    styleInputValue,
    styleInspectorValues,
    styleNumberInputValue,
    themeColorInputValue
  };
}
