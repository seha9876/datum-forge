import type {
  CardPresetBackgroundColorMode,
  CardPresetDefinition
} from "./types";

export const CHALKBOARD_CARD_PRESET_ID = "builtin/chalkboard";

const BUILTIN_CARD_PRESETS: CardPresetDefinition[] = [
  {
    backgroundColorMode: "disabled",
    id: CHALKBOARD_CARD_PRESET_ID,
    kind: "builtin",
    label: "黒板風"
  }
];

export function listCardPresets() {
  return [...BUILTIN_CARD_PRESETS];
}

export function isKnownCardPresetId(presetId: string | null | undefined) {
  if (!presetId) {
    return true;
  }

  return BUILTIN_CARD_PRESETS.some((preset) => preset.id === presetId);
}

export function getCardPresetDefinition(presetId: string | null | undefined) {
  if (!presetId) {
    return null;
  }

  return BUILTIN_CARD_PRESETS.find((preset) => preset.id === presetId) ?? null;
}

export function getCardPresetBackgroundColorMode(
  presetId: string | null | undefined
): CardPresetBackgroundColorMode {
  return (
    getCardPresetDefinition(presetId)?.backgroundColorMode ?? "replace-preset"
  );
}
