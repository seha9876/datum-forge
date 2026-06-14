export type CardPresetBackgroundColorMode =
  | "disabled"
  | "replace-preset"
  | "augment-preset";

export interface CardPresetDefinition {
  backgroundColorMode: CardPresetBackgroundColorMode;
  id: string;
  kind: "builtin";
  label: string;
}
