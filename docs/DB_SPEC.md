# DB仕様書

Datum Forge は、ユーザー定義テーブル本体と、アプリが挙動を理解するためのメタテーブルを同じ SQLite DB に保存する。

## 採用する型

## CSV入出力に関するDB扱い

CSV入出力は新しいメタテーブルを追加せず、既存のユーザー定義テーブルと `app_table_columns` の定義を使って処理する。

CSVインポートの処理結果はDBへ保存せず、Tauri commandの戻り値として返す。戻り値には追加件数、更新件数、スキップ件数、エラー件数、詳細メッセージを含め、フロントエンドのグローバル通知表示に利用する。インポートは引き続き1トランザクションで実行し、形式不正やDBエラー時はロールバックする。

通知表示時間の設定はユーザー定義DBには保存せず、既存の `.local/settings.json` に保存する。設定値は共通秒数、個別設定の有効/無効、成功・警告・エラー別秒数を持つ。各秒数は `0〜60` に正規化し、0は自動非表示なしとして扱う。

### CSVエクスポート

- 対象テーブルのカラム定義は `app_table_columns.sort_order` 順に取得する。
- ヘッダーには `app_table_columns.display_name` を使う。
- レコードはユーザー定義テーブルの `id` 昇順で取得する。
- 値は画面表示用の値へ変換して出力する。
  - `single_select`: `select_options.label`
  - `reference`: `参照先ID:参照先代表表示値`
  - `boolean`: `true` / `false`
- CSVファイル自体はアプリDB外の任意パスへ保存する。

### CSVインポート

- CSVヘッダーは `app_table_columns.column_name` または `app_table_columns.display_name` と完全一致する必要がある。
- CSVヘッダー数は対象テーブルの全カラム数と一致する必要があり、`id` カラムも必須とする。
- ヘッダー検証、値変換、INSERT/UPDATEは1トランザクションで実行する。
- 値変換は既存のカラム型に従う。
  - 空文字: `NULL`
  - `integer` / `date`: 整数
  - `real`: 小数
  - `boolean`: `true` / `false` / `1` / `0`
  - `single_select`: `select_options.label` または `option_no`
  - `reference`: 参照先ID、または `参照先ID:表示値`
- `新しいIDの行だけ追加` では、既存IDと重複する行をスキップし、重複しない行はCSVのIDを維持して追加する。
- `すべて新しい行として追加` では、CSVのIDをINSERT対象から外し、SQLiteの自動採番に任せる。
- `同じIDの行は上書き` では、既存IDの非IDカラムを更新し、存在しないIDはCSVのIDを維持して追加する。

### Excelインポート

- Excelインポートはユーザー定義DBへ新しいメタテーブルを追加せず、CSVインポートと同じユーザー定義テーブル、`app_table_columns`、型変換ルールを使って処理する。
- 対象ファイルは `.xlsx` / `.xlsm` とし、OpenXML内のExcelテーブル定義を読み取る。Excelテーブルとして定義されていない通常範囲やシート全体は対象外とする。
- 文字列セルはOpenXML上の実値を使い、shared strings / inline string のふりがな情報は無視する。表示形式や装飾だけで付く文字列はインポート値に含めない。
- Excelテーブル一覧取得、プレビュー、実行はTauri commandで分ける。プレビューではDBを書き換えず、実行時のみ1トランザクションで追加/更新する。
- 列マッピングはDatum Forge側の全カラムを対象にし、`id` も必須とする。Excel列との初期対応は `column_name` または `display_name` の完全一致で作る。
- 差分確認は既存CSVインポート方式に合わせ、`id` で既存レコードと照合して追加、更新、変更なし、スキップ予定件数を算出する。
- 最後に使用したExcelテーブル名はユーザー定義DBではなく `.local/settings.json` の `lastExcelImportTables` に、Datum ForgeテーブルID単位で保存する。

- `text` -> `TEXT`
- `integer` -> `INTEGER`
- `real` -> `REAL`
- `boolean` -> `INTEGER` (`0` / `1`)
- `date` -> `INTEGER` (`yyyyMMdd`)
- `image` -> `TEXT` (ローカルファイルパス)
- `single_select` -> `INTEGER` (`select_options.option_no`)
- `reference` -> `INTEGER` (参照先レコードID)

## メタテーブル

### app_tables

ユーザー定義テーブルの一覧を管理する。

- `id`
- `table_name`
- `display_name`
- `label_column_id`
- `sort_order`
- `created_at`
- `updated_at`

### app_table_columns

ユーザー定義テーブルのカラム定義を管理する。

- `id`
- `table_id`
- `column_name`
- `display_name`
- `field_type`
- `sort_order`
- `select_option_group_id`
- `ref_table_id`
- `is_required`
- `created_at`
- `updated_at`

テーブル削除時は、削除対象の物理テーブルと `app_tables` / `app_table_columns` の対象行を削除する。あわせて `record_tag_links`, `view_nav_folder_records`, `view_layout_card_column_bindings`, `view_layout_card_overrides` の対象 `table_id` の行を削除する。`app_table_columns.ref_table_id` から参照されているテーブルは削除をブロックし、参照元カラムを先に削除または変更する必要がある。

### select_option_groups / select_options

`single_select` 用の選択肢グループと選択肢を管理する。実データには `option_no` を保存し、表示時に対応する `label` を参照する。

### record_tag_groups / record_tags / record_tag_group_links / record_tag_links

レコードタグを管理する。タグとタググループの所属は `record_tag_group_links` を正とする。`record_tags.group_id` は使わない。

### view_nav_nodes

閲覧モード左サイドバーのカスタム目次フォルダーを管理する。

- `id`
- `node_type`
- `parent_id`
- `name`
- `sort_order`
- `created_at`
- `updated_at`

### view_nav_folder_records

カスタム目次フォルダーに登録した既存レコードを管理する。

- `id`
- `folder_id`
- `table_id`
- `record_id`
- `record_label`
- `sort_order`
- `created_at`
- `updated_at`

`folder_id`, `table_id`, `record_id` の組み合わせは一意とする。

同じフォルダー内の表示順は `sort_order` で管理し、閲覧目次のレコード行ドラッグ並び替えで更新する。

## 閲覧レイアウト

新規DBでは、閲覧レイアウトはフォルダー用カード枠テンプレートに一本化する。廃止済みレイアウトテーブルは作成しない。

### view_layout_templates

カード枠テンプレート本体を管理する。テンプレートはテーブル非依存で、作成時は常にカード0件の空テンプレートとして開始する。

- `id`
- `name`
- `scope_type` (`folder`)
- `folder_id` (`NULL` の場合は共有テンプレート)
- `created_at`
- `updated_at`

`table_id` は持たない。

### view_layout_folder_template_assignments

フォルダーに現在有効なテンプレートを割り当てる。

- `folder_id`
- `template_id`
- `updated_at`

フォルダー内レコードを開くときに割当がなければ、自動作成せず未設定として扱う。

### view_layout_record_template_assignments

フォルダー内に配置されたレコード単位のテンプレート割当を管理する。同じ実レコードでも、別フォルダーまたは別配置なら異なる個別テンプレートを設定できる。

- `folder_record_id`
- `template_id`
- `updated_at`

`folder_record_id` を主キーとする。データ個別テンプレートがある場合はフォルダーテンプレートより優先し、ない場合はフォルダーテンプレートを継承する。フォルダーテンプレートと同じテンプレートを個別指定した場合は、個別割当を作らず解除扱いにする。

### view_layout_template_cards

テンプレート内のカード枠を管理する。カード枠の追加はテンプレート編集画面の `カード追加` が唯一の初期作成導線。

- `card_id`
- `template_id`
- `x`
- `y`
- `width`
- `height`
- `visible`
- `background_color`
- `text_color`
- `font_size`
- `text_direction`
- `font_weight`
- `text_align`
- `padding`
- `padding_top`
- `padding_right`
- `padding_bottom`
- `padding_left`
- `border_radius`
- `show_label`
- `sort_order`
- `updated_at`

### view_layout_card_column_bindings

カード枠と、テーブルごとの表示カラムの紐付けを管理する。

- `template_id`
- `table_id`
- `card_id`
- `column_id`
- `updated_at`

同じカード枠でも、フォルダー内に複数テーブルのレコードがある場合はテーブルごとに異なるカラムへ紐付けられる。

テンプレート編集画面のプレビューでは、このテーブル別紐付けを初期値として読み込む。ただし、テンプレート編集画面で行う一時紐付けはDBへ保存せず、永続的な変更はレコード表示画面のテンプレート設定から `save_view_layout_card_column_bindings` で保存する。

### view_layout_card_overrides

レコード個別のカード枠差分を管理する。テンプレート本体は変更せず、位置、サイズ、表示状態、スタイル差分だけを保存する。

- `template_id`
- `table_id`
- `record_id`
- `card_id`
- `offset_x`
- `offset_y`
- `offset_width`
- `offset_height`
- nullable style fields
- `updated_at`

カードをレコード個別に非表示にした場合は、テンプレート本体を変更せず、このテーブルの対象レコードだけの `visible` 差分として保存する。

## 作成しないテーブル

新規DBでは以下の廃止済みテーブルを作成しない。

- `view_field_layouts`
- `view_layout_template_items`
- `view_field_layout_overrides`
- `view_layout_template_assignments`

## DBファイル

- 既定パス: `.local/datum-forge.sqlite`
- 設定ファイル: `.local/settings.json`
- 設定キー: `dbPath`

起動時に `ready`, `firstLaunch`, `missingDb`, `error` を判定し、必要な場合はDBセットアップ画面を表示する。

新規DB作成は起動時DBセットアップ画面と設定画面から実行でき、どちらも `create_database_file` を使う。新規作成時のファイル名入力は拡張子なしの stem のみを受け付け、フロントエンドが必ず `.sqlite` を付けたファイル名を渡す。たとえば `project` と入力した場合の作成ファイル名は `project.sqlite` である。入力値に `.`、フォルダー区切り、`.db`、`.sqlite` などの拡張子を含めた場合は不正として扱う。

設定画面から新規DBを作成した場合は、作成後に新しいDBを現在の接続先として保存し、アプリ状態を再読み込みする。同名ファイルが存在する場合は作成せずエラーにする。既存DBを開く処理では、既存ファイル互換のため `.sqlite` と `.db` の両方を許可する。
