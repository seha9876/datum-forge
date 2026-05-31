import type { FieldType, TableRecord } from "../types";

export const fieldTypes: FieldType[] = [
  "text",
  "integer",
  "real",
  "boolean",
  "date",
  "image",
  "single_select",
  "reference"
];

const fieldTypeLabels: Record<FieldType, string> = {
  text: "テキスト",
  integer: "整数",
  real: "小数",
  boolean: "真偽値",
  date: "日付",
  image: "画像",
  single_select: "単一選択",
  reference: "参照"
};

let optionClientKeySeed = 0;

export function createDefaultOptionRows() {
  return [
    {
      clientKey: createOptionClientKey(),
      optionNo: 1,
      sortOrder: 1,
      label: ""
    },
    {
      clientKey: createOptionClientKey(),
      optionNo: 2,
      sortOrder: 2,
      label: ""
    }
  ];
}

export function createOptionClientKey() {
  optionClientKeySeed += 1;
  return `option-${optionClientKeySeed}`;
}

export function isRequiredValueEmpty(value: unknown) {
  return (
    value === null ||
    value === undefined ||
    (typeof value === "string" && value.trim() === "")
  );
}

export function normalizeRecordValues(
  record: TableRecord,
  booleanColumnNames: string[]
) {
  const normalized = { ...record.values };

  for (const columnName of booleanColumnNames) {
    if (columnName in normalized) {
      normalized[columnName] = Boolean(normalized[columnName]);
    }
  }

  return normalized;
}

export function inputType(fieldType: FieldType) {
  if (fieldType === "integer" || fieldType === "real") {
    return "number";
  }

  if (fieldType === "date") {
    return "date";
  }

  return "text";
}

export function fieldTypeLabel(fieldType: FieldType) {
  return fieldTypeLabels[fieldType];
}
