export interface FormattedImportMessage {
  id: string;
  message: string;
  action: string;
  rowSummary: string;
  details: string[];
}

interface ParsedImportMessage {
  key: string;
  message: string;
  action: string;
  row: number | null;
  rawMessage: string;
}

const MAX_VISIBLE_ROWS = 5;

/** バックエンドから複数行文字列で返る詳細も、1件ずつ扱える形にそろえます。 */
function normalizeRawMessages(messages: string[]) {
  return messages
    .flatMap((message) => message.split(/\r?\n/))
    .map((message) => message.trim())
    .filter((message) => message.length > 0);
}

/** Rust側で付けている内部向けプレフィックスは、表示用の解析前に外します。 */
function stripInternalPrefix(message: string) {
  return message.replace(/^invalid input:\s*/i, "").trim();
}

/** 同じ原因のエラーが多いときに、画面へ出す行番号を短くまとめます。 */
function formatRows(rows: number[]) {
  if (rows.length === 0) {
    return "";
  }

  const sortedRows = [...new Set(rows)].sort((a, b) => a - b);
  const visibleRows = sortedRows.slice(0, MAX_VISIBLE_ROWS).join(", ");
  const hiddenCount = sortedRows.length - MAX_VISIBLE_ROWS;
  return hiddenCount > 0 ? `${visibleRows} ほか${hiddenCount}件` : visibleRows;
}

function rowMessage(
  row: number,
  column: string,
  reason: string,
  rawMessage: string
): ParsedImportMessage {
  const base = {
    row,
    rawMessage
  };

  switch (reason) {
    case "is required":
      return {
        ...base,
        key: `required:${column}`,
        message: `「${column}」が空の行があります。`,
        action: "必須項目のため値を入力してください。"
      };
    case "does not match an option":
      return {
        ...base,
        key: `option:${column}`,
        message: `「${column}」に登録されていない値があります。`,
        action: "選択肢に存在する値を入力してください。"
      };
    case "must be an integer":
      return {
        ...base,
        key: `integer:${column}`,
        message: `「${column}」は整数で入力してください。`,
        action: "小数や文字を含めず、整数だけを入力してください。"
      };
    case "must be a number":
      return {
        ...base,
        key: `number:${column}`,
        message: `「${column}」は数値で入力してください。`,
        action: "文字や記号を含めず、数値として読める値を入力してください。"
      };
    case "must be a valid date":
      return {
        ...base,
        key: `date:${column}`,
        message: `「${column}」は日付として読み取れません。`,
        action: "日付形式を確認してください。"
      };
    case "must be true, false, 1, or 0":
      return {
        ...base,
        key: `boolean:${column}`,
        message: `「${column}」は真偽値として読み取れません。`,
        action: "true / false / 1 / 0 のいずれかを入力してください。"
      };
    case "must be a reference id":
      return {
        ...base,
        key: `reference:${column}`,
        message: `「${column}」の参照先IDを読み取れません。`,
        action:
          "参照先のID、またはエクスポート形式の「ID:表示名」を入力してください。"
      };
    case "has no Excel column mapping":
    case "has no CSV column mapping":
      return {
        ...base,
        key: `mapping:${column}`,
        message: `「${column}」に対応する取り込み元の列がありません。`,
        action: "列の対応付けを確認してください。"
      };
    default:
      return {
        ...base,
        key: `row:${column}:${reason}`,
        message: `「${column}」に取り込めない値があります。`,
        action: "入力内容を確認してください。"
      };
  }
}

/** `row 2: 性別 does not match an option` のような行単位エラーを解析します。 */
function parseRowMessage(message: string, rawMessage: string) {
  const rowMatch = message.match(/^row\s+(\d+):\s+(.+)$/i);
  if (!rowMatch) {
    return null;
  }

  const row = Number(rowMatch[1]);
  const body = rowMatch[2].replace(/\s+\(raw:\s.*\)$/, "");
  const knownReasons = [
    "must be true, false, 1, or 0",
    "has no Excel column mapping",
    "has no CSV column mapping",
    "does not match an option",
    "must be a reference id",
    "must be a valid date",
    "must be an integer",
    "must be a number",
    "is required"
  ];
  const reason = knownReasons.find((item) => body.endsWith(` ${item}`));
  if (!reason) {
    return rowMessage(row, body, "unknown", rawMessage);
  }

  return rowMessage(
    row,
    body.slice(0, -reason.length).trim(),
    reason,
    rawMessage
  );
}

/** ヘッダー不一致やファイル形式不正など、行番号を持たないエラーを解析します。 */
function parseNonRowMessage(
  message: string,
  rawMessage: string
): ParsedImportMessage {
  const headerCount = message.match(
    /^CSV header count must match table columns: expected (\d+), got (\d+)$/i
  );
  if (headerCount) {
    return {
      key: "csv-header-count",
      message: "CSVの列数が取り込み先テーブルと一致しません。",
      action: `列数を確認してください。必要な列数は${headerCount[1]}列、ファイル内は${headerCount[2]}列です。`,
      row: null,
      rawMessage
    };
  }

  const headerMatch = message.match(
    /^CSV header `(.+)` does not uniquely match a table column$/i
  );
  if (headerMatch) {
    return {
      key: `csv-header:${headerMatch[1]}`,
      message: `CSV列「${headerMatch[1]}」の取り込み先を特定できません。`,
      action: "物理名または表示名と完全一致する列名にしてください。",
      row: null,
      rawMessage
    };
  }

  const duplicateCsvColumn = message.match(
    /^CSV column `(.+)` is mapped more than once$/i
  );
  if (duplicateCsvColumn) {
    return {
      key: `csv-duplicate:${duplicateCsvColumn[1]}`,
      message: `「${duplicateCsvColumn[1]}」に複数の列が対応しています。`,
      action:
        "同じ取り込み先に複数列が対応しないよう、CSVヘッダーを確認してください。",
      row: null,
      rawMessage
    };
  }

  const duplicateExcelColumn = message.match(
    /^Excel column `(.+)` is mapped more than once$/i
  );
  if (duplicateExcelColumn) {
    return {
      key: `excel-duplicate:${duplicateExcelColumn[1]}`,
      message: `Excel列「${duplicateExcelColumn[1]}」に複数の列が対応しています。`,
      action:
        "同じ取り込み元列を複数の取り込み先に割り当てないよう、列の対応付けを確認してください。",
      row: null,
      rawMessage
    };
  }

  const missingCsvColumn = message.match(/^CSV column `(.+)` was not found$/i);
  if (missingCsvColumn) {
    return {
      key: `csv-missing:${missingCsvColumn[1]}`,
      message: `CSV列「${missingCsvColumn[1]}」が見つかりません。`,
      action: "CSVヘッダー、または列の対応付けを確認してください。",
      row: null,
      rawMessage
    };
  }

  if (/^CSV header must include id$/i.test(message)) {
    return {
      key: "csv-id-missing",
      message: "CSVにID列がありません。",
      action: "インポートするCSVに id 列を追加してください。",
      row: null,
      rawMessage
    };
  }

  const missingExcelColumn = message.match(
    /^Excel列 `(.+)` が見つかりません。?$/
  );
  if (missingExcelColumn) {
    return {
      key: `excel-missing:${missingExcelColumn[1]}`,
      message: `Excel列「${missingExcelColumn[1]}」が見つかりません。`,
      action: "Excelテーブルの列名、または列の対応付けを確認してください。",
      row: null,
      rawMessage
    };
  }

  const missingExcelColumnEnglish = message.match(
    /^Excel column `(.+)` was not found$/i
  );
  if (missingExcelColumnEnglish) {
    return {
      key: `excel-missing:${missingExcelColumnEnglish[1]}`,
      message: `Excel列「${missingExcelColumnEnglish[1]}」が見つかりません。`,
      action: "Excelテーブルの列名、または列の対応付けを確認してください。",
      row: null,
      rawMessage
    };
  }

  if (message.includes("id列の対応付けが必要")) {
    return {
      key: "excel-id-mapping",
      message: "ID列の対応付けが必要です。",
      action:
        "列の対応付けで、Datum Forgeの id に対応するExcel列を選択してください。",
      row: null,
      rawMessage
    };
  }

  if (/^id column mapping is required$/i.test(message)) {
    return {
      key: "id-mapping",
      message: "ID列の対応付けが必要です。",
      action:
        "列の対応付けで、Datum Forgeの id に対応する列を選択してください。",
      row: null,
      rawMessage
    };
  }

  if (
    message.includes("unsupported file type") ||
    message.includes("未対応の拡張子")
  ) {
    return {
      key: "unsupported-file",
      message: "このファイル形式はインポートできません。",
      action: "CSV、xlsx、xlsm のいずれかのファイルを選択してください。",
      row: null,
      rawMessage
    };
  }

  if (message.includes("duplicate record")) {
    return {
      key: "duplicate-record",
      message: "同じデータが既に存在します。",
      action: "追加方法または更新方法を確認してください。",
      row: null,
      rawMessage
    };
  }

  if (message.includes("reference not found")) {
    return {
      key: "reference-not-found",
      message: "参照先のデータが見つかりません。",
      action: "先に参照先データを登録してください。",
      row: null,
      rawMessage
    };
  }

  return {
    key: `unknown:${message}`,
    message: "取り込み内容を確認してください。",
    action:
      "ファイル形式、列名、セルの値が取り込み先テーブルに合っているか確認してください。",
    row: null,
    rawMessage
  };
}

function parseImportMessage(rawMessage: string) {
  const message = stripInternalPrefix(rawMessage);
  return (
    parseRowMessage(message, rawMessage) ??
    parseNonRowMessage(message, rawMessage)
  );
}

/** 画面表示用に、同じ列・同じ原因のエラーを1つのメッセージへ集約します。 */
export function formatImportMessages(
  messages: string[]
): FormattedImportMessage[] {
  const groups = new Map<string, ParsedImportMessage[]>();

  for (const rawMessage of normalizeRawMessages(messages)) {
    const parsed = parseImportMessage(rawMessage);
    const group = groups.get(parsed.key) ?? [];
    group.push(parsed);
    groups.set(parsed.key, group);
  }

  return Array.from(groups.entries()).map(([key, group]) => {
    const first = group[0];
    return {
      id: key,
      message: first.message,
      action: first.action,
      rowSummary: formatRows(
        group.flatMap((item) => (item.row === null ? [] : [item.row]))
      ),
      details: group.map((item) => item.rawMessage)
    };
  });
}

/** Snackbarの詳細欄へ渡せるよう、集約済みメッセージと内部詳細を文字列へ戻します。 */
export function formatImportNotificationDetails(messages: string[]) {
  return formatImportMessages(messages).flatMap((message) => [
    [
      message.message,
      message.rowSummary ? `対象行: ${message.rowSummary}` : "",
      message.action
    ]
      .filter(Boolean)
      .join(" "),
    ...message.details.map((detail) => `詳細: ${detail}`)
  ]);
}
