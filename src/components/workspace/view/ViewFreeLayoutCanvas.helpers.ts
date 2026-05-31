import type { ViewLayoutCardItem } from "../../../types";

export type RectLike = {
  bottom: number;
  left: number;
  right: number;
  top: number;
};

export type CanvasElement = {
  clientHeight: number;
  clientWidth: number;
  getBoundingClientRect: () => RectLike;
};

export type PointerTarget = {
  setPointerCapture?: (pointerId: number) => void;
};

export type PointerLikeEvent = {
  button?: number;
  clientX: number;
  clientY: number;
  ctrlKey?: boolean;
  currentTarget: object | null;
  pointerId: number;
  preventDefault: () => void;
  shiftKey: boolean;
  stopPropagation: () => void;
};

export type KeyboardLikeEvent = {
  key: string;
  preventDefault?: () => void;
  repeat?: boolean;
};

export type WheelLikeEvent = {
  clientX: number;
  clientY: number;
  ctrlKey: boolean;
  deltaY: number;
  preventDefault: () => void;
};

export type InputLikeEvent = {
  target: {
    checked?: boolean;
    value: string;
  } | null;
};

export type MoveInteraction = {
  cardId: number;
  cardIds: number[];
  moved: boolean;
  originLayouts: ViewLayoutCardItem[];
  originalSelection: number[];
  shiftKey: boolean;
  startX: number;
  startY: number;
  type: "move";
};

export type ResizeDirection = "n" | "s" | "e" | "w" | "ne" | "nw" | "se" | "sw";

export type ResizeInteraction = {
  cardId: number;
  direction: ResizeDirection;
  moved: boolean;
  origin: ViewLayoutCardItem;
  startX: number;
  startY: number;
  type: "resize";
};

export type InteractionState = MoveInteraction | ResizeInteraction;

export type SelectionBox = {
  height: number;
  width: number;
  x: number;
  y: number;
};

export type SelectionDrag = {
  additive: boolean;
  initialSelection: number[];
  moved: boolean;
  startX: number;
  startY: number;
};

export type PanDrag = {
  startOffsetX: number;
  startOffsetY: number;
  startX: number;
  startY: number;
};

export type LayoutStyleKey =
  | "backgroundColor"
  | "textColor"
  | "fontSize"
  | "textDirection"
  | "fontWeight"
  | "textAlign"
  | "padding"
  | "paddingTop"
  | "paddingRight"
  | "paddingBottom"
  | "paddingLeft"
  | "borderRadius"
  | "showLabel";

export type LayoutStyleValue = boolean | string | number | null;

export const MIN_CARD_WIDTH = 120;
export const MIN_CARD_HEIGHT = 64;
export const DRAG_THRESHOLD = 4;
export const WORLD_WIDTH = 2400;
export const WORLD_HEIGHT = 1600;
export const MIN_VIEWPORT_SCALE = 0.25;
export const MAX_VIEWPORT_SCALE = 3;
export const VIEWPORT_ZOOM_STEP = 0.15;

export const RESIZE_HANDLES: Array<{
  direction: ResizeDirection;
  label: string;
}> = [
  { direction: "n", label: "上辺でサイズ変更" },
  { direction: "e", label: "右辺でサイズ変更" },
  { direction: "s", label: "下辺でサイズ変更" },
  { direction: "w", label: "左辺でサイズ変更" },
  { direction: "ne", label: "右上でサイズ変更" },
  { direction: "se", label: "右下でサイズ変更" },
  { direction: "sw", label: "左下でサイズ変更" },
  { direction: "nw", label: "左上でサイズ変更" }
];

export const DEFAULT_CARD_STYLE = {
  backgroundColor: null,
  borderRadius: 14,
  fontSize: 16,
  fontWeight: "bold",
  padding: 12,
  paddingTop: 12,
  paddingRight: 12,
  paddingBottom: 12,
  paddingLeft: 12,
  showLabel: true,
  textAlign: "left",
  textColor: null,
  textDirection: "horizontal"
} as const;

export function clampViewportScale(scale: number) {
  return Math.max(MIN_VIEWPORT_SCALE, Math.min(MAX_VIEWPORT_SCALE, scale));
}

export function boxFromPoints(
  startX: number,
  startY: number,
  endX: number,
  endY: number
): SelectionBox {
  const x = Math.min(startX, endX);
  const y = Math.min(startY, endY);
  return {
    height: Math.abs(endY - startY),
    width: Math.abs(endX - startX),
    x,
    y
  };
}

export function intersects(a: SelectionBox, b: SelectionBox) {
  return (
    a.x < b.x + b.width &&
    a.x + a.width > b.x &&
    a.y < b.y + b.height &&
    a.y + a.height > b.y
  );
}
