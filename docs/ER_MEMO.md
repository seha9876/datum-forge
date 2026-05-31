# ERメモ

## 基本方針

Datum Forge は、ユーザーが作成する実データテーブルと、アプリが管理するメタテーブルを SQLite に保存する。

メタテーブルは以下を管理する。

- テーブル定義
- カラム定義
- single_select 選択肢
- レコードタグ
- 閲覧モードのカスタム目次
- 閲覧レイアウトのカード枠テンプレート

## メタテーブル一覧

- `app_tables`
- `app_table_columns`
- `select_option_groups`
- `select_options`
- `record_tag_groups`
- `record_tags`
- `record_tag_group_links`
- `record_tag_links`
- `view_nav_nodes`
- `view_nav_folder_records`
- `view_layout_templates`
- `view_layout_folder_template_assignments`
- `view_layout_record_template_assignments`
- `view_layout_template_cards`
- `view_layout_card_column_bindings`
- `view_layout_card_overrides`

新規DBでは廃止済みレイアウトテーブルは作成しない。

## 関係

`app_tables` 1 - N `app_table_columns`

`select_option_groups` 1 - N `select_options`

`record_tags` N - N `record_tag_groups` via `record_tag_group_links`

`record_tags` N - N ユーザー定義レコード via `record_tag_links`

`view_nav_nodes` 1 - N `view_nav_nodes`

`view_nav_nodes` 1 - N `view_nav_folder_records`

`app_tables` 1 - N `view_nav_folder_records`

`view_nav_nodes` 0..1 - 1 `view_layout_folder_template_assignments`

`view_nav_folder_records` 0..1 - 1 `view_layout_record_template_assignments`

`view_layout_templates` 1 - N `view_layout_record_template_assignments`

`view_layout_templates` 1 - N `view_layout_template_cards`

`view_layout_template_cards` 1 - N `view_layout_card_column_bindings`

`view_layout_template_cards` 1 - N `view_layout_card_overrides`

## 閲覧レイアウト

閲覧レイアウトはフォルダー用カード枠テンプレートを正とする。

- テンプレート本体は `view_layout_templates` に保存する。
- テンプレートはテーブル非依存で、`table_id` を持たない。
- 共有テンプレートは `folder_id IS NULL` とする。
- フォルダー専用テンプレートは `folder_id` を持つ。
- フォルダーへの有効テンプレート割当は `view_layout_folder_template_assignments` に保存する。
- データ個別テンプレート割当は `view_layout_record_template_assignments` に保存する。
- カード枠は `view_layout_template_cards` に保存する。
- カード枠と表示カラムの対応は `view_layout_card_column_bindings` に保存する。
- レコードごとの移動、リサイズ、表示状態、スタイル差分は `view_layout_card_overrides` に保存する。

テンプレート作成時に生成元テーブルは選ばない。作成直後はカード0件で、テンプレート編集画面の `カード追加` からカード枠を増やす。

テンプレート編集では任意のテーブル/レコードを一時プレビューできる。プレビュー時の一時紐付けは保存せず、永続的なカード枠と表示カラムの対応は `view_layout_card_column_bindings` のみを正とする。

フォルダー内レコードを開いたときの有効テンプレートは、データ個別テンプレート、フォルダーテンプレート、未設定の順で解決する。有効テンプレートが未割当なら自動作成せず未設定として扱う。

## 作成しない廃止済みレイアウトテーブル

- `view_field_layouts`
- `view_layout_template_items`
- `view_field_layout_overrides`
- `view_layout_template_assignments`

これらは新規DB前提の仕様には含めない。

## レコードタグ

タグ本体は `record_tags`、タググループは `record_tag_groups`、所属関係は `record_tag_group_links` で管理する。レコードへのタグ付与は `record_tag_links` に保存する。

## 参照カラム

`reference` 型の参照先テーブルは `app_table_columns.ref_table_id` で固定する。実データには参照先レコードIDを保存し、表示時は参照先テーブルの代表表示カラムを使って `id:label` 形式で表示する。

## CSV入出力

CSV入出力はメタテーブルを増やさず、ユーザー定義テーブルと `app_table_columns` の定義を使って処理する。

- エクスポートは `id` を含む全カラムを対象にし、ヘッダーは `display_name`、レコード順は `id ASC` とする。
- インポート時のCSVヘッダーは `column_name` または `display_name` と完全一致する必要がある。
- CSVヘッダー数は対象テーブルの全カラム数と一致し、`id` も含む必要がある。
- インポートは1トランザクションで処理し、エラー時は全行ロールバックする。
- インポート方式は、既存IDスキップ、自動採番で全件追加、ID重複時更新の3種類とする。
- `single_select` はラベルまたは `option_no`、`reference` はIDまたは `id:label` 形式を受け付ける。
