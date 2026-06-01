# PROJECT SPEC

## インポート結果通知

- CSV/Excelインポート完了後は、アプリ右下のグローバルSnackbarで必ず結果を通知する。
- 通知には結果種別、追加件数、更新件数、スキップ件数、エラー件数を表示する。
- Snackbarはユーザー設定の秒数に従って自動で閉じる。0秒の場合は自動で閉じず、閉じるボタンで閉じる。
- 自動非表示が有効な通知では `v-progress-linear` の残り時間バーを表示し、マウスホバー中はカウントを停止する。
- 既存IDをスキップした場合は警告として扱い、「詳細を見る」から補足内容を確認できる。
- 形式不正やDBエラーなどでインポートが失敗した場合は、取り込み処理をロールバックし、エラー件数1件として通知する。
- インポートの警告・エラーは、画面上ではユーザー向けの日本語メッセージへ整形して表示する。同じ列・同じ原因の行エラーは対象行をまとめ、内部エラー文は詳細として確認できるようにする。
- 将来的にはRust側から `{ row, column, value, kind, candidates, rawMessage }` のような構造化エラーを返し、利用可能な選択肢なども安定して表示できる形へ拡張する。
- 通知機構はCSVインポート専用ではなく、将来のエクスポート、PDF出力、バックアップにも再利用できるグローバル機構とする。

## 通知設定

- 通知設定は `.local/settings.json` に保存し、アプリ再起動後も維持する。
- 通常は共通の「通知の表示時間」を秒単位で設定する。
- `種類ごとに個別設定する` を有効にした場合だけ、成功・警告・エラーの表示時間を個別に設定できる。
- 各秒数は `0〜60` の整数として扱う。範囲外の値は保存時に範囲内へ丸める。
- 個別設定が無効な場合、成功・警告・エラー・情報通知は共通秒数を使う。

## CSV入出力

設計モードおよびデータモードの左サイドバーにあるテーブル操作メニューから、選択テーブルのCSVエクスポートとインポートを実行できる。インポートは1つの入口からファイルを選び、拡張子に応じてCSVまたはExcelとして処理する。

### CSVエクスポート

- 保存先は実行時に毎回選択する。
- 既定ファイル名は `テーブル表示名_YYYYMMDD.csv` とする。
- 出力対象は `id` を含む全カラムとする。
- ヘッダーはカラムの論理名である `app_table_columns.display_name` を使う。
- レコードはID昇順で出力する。
- セル値は画面表示用の値を使う。`single_select` はラベル、`reference` は `id:label` 形式で出力する。
- ファイルはUTF-8 BOM付き、CRLF改行で保存する。

### CSVインポート

- 読み込むファイルはCSV形式のみとする。
- CSVファイル選択後、Datum Forge列とCSV列のマッピングを表示する。初期マッピングは物理名 `column_name` または論理名 `display_name` の完全一致を優先する。
- `id` を含む全Datum Forge列が一意に対応していない場合、または型変換できない値がある場合は実行不可にする。
- CSV側に未使用列がある場合は警告として表示するが、取り込み先に割り当てられていないだけなら実行可能とする。
- インポート実行前に、先頭10件のプレビュー、総件数、追加予定件数、更新予定件数、変更なし件数、スキップ予定件数、エラー件数を必ず表示する。
- インポートは1トランザクションで処理し、途中エラー時は全行をロールバックする。
- インポート後は対象テーブルを再読み込みし、データモードの一覧へ反映する。

インポート方式は次の3種類とする。直近に選んだ方式を `localStorage` に保存し、通常の `インポート` クリックではその方式を使う。

- `新しいIDの行だけ追加`: CSVのIDが既存レコードと重複する行はスキップし、存在しないIDの行だけCSVのIDを維持して追加する。
- `すべて新しい行として追加`: CSVのIDは使わず、SQLiteの自動採番で全行を新規追加する。
- `同じIDの行は上書き`: CSVのIDが既存なら非IDカラムを更新し、存在しないIDならCSVのIDを維持して追加する。

CSV値の変換ルールは次の通りとする。

- 空文字は未入力として扱う。
- 必須カラムが空の場合はエラーにする。
- `boolean` は `true` / `false` / `1` / `0` を受け付ける。
- `single_select` は選択肢ラベル完全一致、または `option_no` を受け付ける。
- `reference` は参照先ID、またはエクスポート時の `id:label` 形式を受け付ける。

### Excelインポート

- 読み込むファイルは `.xlsx` / `.xlsm` とし、Excelの「テーブル」として定義された範囲だけを対象にする。通常のシート範囲やブック全体は取り込まない。
- 文字列セルはExcelの見た目ではなくOpenXML上の実値を読み取る。shared strings / inline string に含まれるふりがな情報は無視し、表示形式や装飾だけで付く接尾辞は取り込まない。
- Excelファイル選択後、ブック内のExcelテーブル一覧を表示し、取り込むExcelテーブルを選択する。
- 対象のDatum Forgeテーブルで前回使ったExcelテーブル名が `.local/settings.json` に残っている場合は初期候補にする。前回候補がない場合は、Datum Forge側の物理名/論理名に近いExcelテーブルを候補にする。
- Excelテーブル選択後、Datum Forge列とExcel列のマッピングを表示する。初期マッピングは物理名または論理名の完全一致を優先する。
- `id` を含む全Datum Forge列が一意に対応していない場合、または型変換できない値がある場合は実行不可にする。
- インポート実行前に、先頭10件のプレビュー、総件数、追加予定件数、更新予定件数、変更なし件数、スキップ予定件数、エラー件数を必ず表示する。
- インポート方式はCSVインポートと同じ3種類を使い、既存レコードとの照合キーも `id` のみとする。
- 実行後はCSVインポートと同じグローバルSnackbarで結果を通知する。

## 概要

Datum Forge は、ユーザーが自由にテーブルを定義し、設計、データ入力、閲覧、マスタ管理を行うデスクトップアプリである。DB は SQLite を使い、ユーザー定義テーブルとアプリ用メタテーブルを同じDBに保存する。

## 起動とDBセットアップ

起動時に現在のDB状態を判定する。

- `ready`: DBを利用できる
- `firstLaunch`: 設定ファイルがなく、初回セットアップが必要
- `missingDb`: 設定済みDBファイルが見つからない
- `error`: 設定ファイル破損やパス解決失敗など、復旧操作が必要

DBセットアップ未完了時は通常ワークスペースを表示せず、DBセットアップ画面を表示する。

起動時DBセットアップ画面と設定画面のデータベースカテゴリから新規DBを作成できる。新規作成時のDBファイル名入力は拡張子なしの名前のみを受け付け、実際のファイルは必ず `.sqlite` 拡張子で作成する。たとえば `project` と入力した場合は `project.sqlite` を作成する。入力値に `.`、フォルダー区切り、拡張子を含めた場合は作成しない。

設定画面で作成したDBは、作成完了後すぐ現在の接続先として開き、ワークスペース状態を初期化して再読み込みする。既存DBファイルは上書きしない。既存DBを開く操作では、既存資産を扱えるように `.sqlite` と `.db` の両方を許可する。

## UIシェル

- Tauri の標準タイトルバーは使わず、Vue 側のカスタムタイトルバーをアプリ最上段に表示する。
- カスタムタイトルバーには、アプリアイコン、サイドメニュー開閉ボタン、アプリケーションメニュー、モード切替タブ、ウィンドウ操作ボタンを配置する。
- モード切替はカスタムタイトルバー内の `v-tabs` で行い、本文側にはモード切替ヘッダーを置かない。
- 各モード本文の上部に、画面名や補助文言を表示する共通ヘッダーは置かない。本文はタイトルバー直下のワークスペース領域から開始する。
- アプリケーションメニューの `ヘルプ` はメニューとして開き、`このモードのヘルプ` と `設定` を表示する。
- `このモードのヘルプ` は通常モードでのみ利用できる。設定画面、DB確認中、DBセットアップ中は無効にする。
- `設定` は `ヘルプ` メニューから開閉する。設定画面を開く前の通常モードを保持し、設定を閉じると元の通常モードへ戻る。
- `ファイル`、`編集`、`表示`、`ウィンドウ` は将来拡張用の表示項目とし、現時点では機能を持たない。

## モード

### 設計モード

テーブルとカラムを管理する。

- テーブル作成時は SQLite 上に物理テーブルを作成する。
- 作成直後に `id INTEGER PRIMARY KEY AUTOINCREMENT` を追加する。
- テーブル削除は左サイドバーのテーブル一覧から行う。削除時は物理テーブル、カラム定義、レコードタグ紐付け、閲覧ナビ配置、レイアウト差分を削除する。
- 他テーブルの `reference` カラムから参照されているテーブルは削除できない。参照元カラムを削除または変更してから削除する。
- カラム追加、更新、削除、並び替え、代表表示カラム設定を行う。
- `single_select` は選択肢グループを必須とする。
- `reference` は参照先テーブルを必須とする。

### データモード

選択中テーブルのレコードを作成、更新、削除する。

- 入力UIはカラム型に応じて切り替える。
- `reference` は参照先テーブルの代表表示カラムで候補を表示する。
- 一覧表示では型ごとに整形した表示値を使う。

### 閲覧モード

左サイドバーのカスタム目次からフォルダーとレコードを開き、カード枠テンプレートに基づく自由配置ビューを表示する。

#### 左サイドバー

- 上部の表示切り替えは `v-btn-toggle + v-btn` を使う。
- 通常幅では `目次` / `テンプレート` をアイコン + ラベルで表示する。
- 狭幅ではラベルを非表示にしてアイコンのみ表示にし、tooltip で意味を補う。
- rail 表示時は既存の開閉操作と現在タブアイコンを維持する。

#### 目次

- フォルダー階層は `view_nav_nodes` で管理する。
- フォルダーに登録した既存レコードは `view_nav_folder_records` で管理する。
- フォルダー内レコードを開くと、データ個別テンプレート、フォルダーテンプレートの順で解決した有効テンプレートを使って表示する。
- フォルダー内レコードは左側の目次行全体をドラッグして同じフォルダー内で並び替えでき、順序は `view_nav_folder_records.sort_order` に保存する。

#### レイアウトテンプレート

新規DB前提では、閲覧レイアウトはフォルダー用カード枠テンプレートに一本化する。

- テンプレートは `view_layout_templates` に保存する。
- テンプレートはテーブル非依存で、`table_id` を持たない。
- 作成時に生成元テーブルは選ばない。
- 作成直後はカード0件の空テンプレートとする。
- カード枠はテンプレート編集画面の `カード追加` から作成する。
- 共有テンプレートは `folder_id = NULL` とする。
- フォルダー専用テンプレートは `folder_id` を持つ。
- フォルダーへの有効テンプレート割当は `view_layout_folder_template_assignments` に保存する。
- データ個別テンプレート割当は `view_layout_record_template_assignments` に保存する。
- 有効テンプレートの優先順位は、データ個別テンプレート、フォルダーテンプレート、未設定の順とする。
- フォルダー内レコードを開くときに有効テンプレートがなければ、自動作成せず未設定として扱う。
- テンプレート編集ではカード枠の追加、移動、リサイズ、削除、スタイル編集を行う。
- テンプレート編集では任意のテーブル/レコードを選んで一時プレビューできる。プレビューはテンプレート本体に保存しない。
- テンプレート編集のプレビューでは、既存の `view_layout_card_column_bindings` を初期値として読み込み、一時紐付けで表示カラムを仮に変更できる。一時紐付けは保存しない。
- 一時紐付けUIは折りたたみ可能とし、未紐付けカードがある場合だけ自動展開する。
- テンプレート編集の非編集時は、プレビュー対象がある場合に実データを完成イメージとして表示する。編集時のみカード操作と一時紐付け操作を行える。
- レコード個別キャンバスでは、テンプレート編集は行わず、データ個別テンプレート選択とカード枠への個別差分を保存する。

#### テンプレート設定

- レコード表示画面の設定ボタン名は `テンプレート設定` とする。
- テンプレート設定パネルでは、適用元を `このデータ専用`、`フォルダから継承中`、`未設定` のチップで表示する。
- `フォルダから継承中` のチップにはテンプレート名を括弧付きで省略表示し、長い名前は tooltip で全文を確認できる。
- 個別テンプレート select では、現在フォルダーで使える共有テンプレートとフォルダー専用テンプレートから、フォルダーテンプレートと同じものを除外して選択できる。
- 個別テンプレートを解除すると、フォルダーテンプレート継承または未設定へ戻る。
- フォルダーテンプレートと同じテンプレートを個別指定しようとした場合は、個別割当を作らず解除扱いにする。

#### カードとカラムの紐付け

- テンプレート本体はテーブル非依存のため、カード枠と表示カラムの対応は `view_layout_card_column_bindings` に `template_id + table_id + card_id -> column_id` として保存する。
- レコード表示時に対象テーブルの紐付けがない場合は、テンプレート設定パネル内で表示カラムを選択できる。
- テンプレート設定パネルではキャンバス上のカード位置を見ながら、カードごとに対象テーブルのカラムを1行UIで選択できる。
- 紐付け済み後もレコード表示画面の `テンプレート設定` から同じテーブル向けの紐付けを編集できる。
- テンプレート設定パネルは開閉をトグルでき、未保存の変更がある状態で閉じようとした場合は破棄確認ダイアログを表示する。
- テンプレート編集画面では永続的な紐付け変更は行わない。

#### レコード個別差分

- レコード個別の移動、リサイズ、表示状態、スタイル変更は `view_layout_card_overrides` に保存する。
- テンプレート本体は変更しない。
- カード単位、レコード単位で差分をリセットできる。
- レコード編集でカードを `非表示にする` と、そのレコードだけの `visible = false` 差分として保存する。
- 非編集時は非表示カードを表示しない。編集時は非表示カードを破線や `非表示` チップで表示し、`表示に戻す` 操作で再表示できる。

### マスタ管理モード

- `single_select` 用の選択肢グループと選択肢を管理する。
- レコードタグとタググループを管理する。
- タグ所属判定は `record_tag_group_links` を正とする。

## Tauri command

主な command は以下を提供する。

- DB: `get_startup_database_status`, `create_database_file`, `setup_open_database_file`, `open_database_file`, `update_database_directory`, `rename_database_file`
- テーブル: `bootstrap_app`, `create_table`, `get_table_detail`, `add_column`, `update_column`, `delete_column`, `reorder_columns`, `update_label_column`
- レコード: `save_record`, `delete_record`, `get_reference_choices`
- 閲覧目次: `list_view_nav_nodes`, `create_view_nav_folder`, `delete_view_nav_folder`, `get_view_table_sections`, `add_view_nav_folder_records`, `remove_view_nav_folder_record`, `reorder_view_nav_folder_records`
- レイアウトテンプレート: `list_all_folder_layout_templates`, `list_view_layout_templates_for_folder`, `create_view_layout_template`, `rename_view_layout_template`, `duplicate_view_layout_template`, `delete_view_layout_template`, `assign_view_layout_folder_template`, `assign_view_layout_record_template`, `clear_view_layout_record_template`
- カード枠: `get_view_layout_template_cards`, `save_view_layout_template_cards`, `get_resolved_view_field_layout`, `list_view_layout_card_column_bindings`, `save_view_layout_card_column_bindings`, `save_view_layout_card_overrides`, `reset_view_layout_card_override`, `reset_view_layout_card_overrides`
- タグ: `list_record_tags`, `list_record_tags_for_record`, `save_record_tag_group`, `delete_record_tag_group`, `save_record_tag`, `delete_record_tag`, `attach_record_tag_group`, `detach_record_tag_group`, `attach_record_tag`, `create_and_attach_record_tag`, `detach_record_tag`

削除済みの廃止済みレイアウト command は提供しない。

- `get_view_field_layout`
- `save_view_field_layout`
- `list_view_layout_templates(tableId)`
- `assign_view_layout_template`
- `save_view_layout_template_items`

## 新規DBで作成する閲覧レイアウト系テーブル

- `view_layout_templates`
- `view_layout_folder_template_assignments`
- `view_layout_record_template_assignments`
- `view_layout_template_cards`
- `view_layout_card_column_bindings`
- `view_layout_card_overrides`

## 新規DBで作成しない廃止済みレイアウト系テーブル

- `view_field_layouts`
- `view_layout_template_items`
- `view_field_layout_overrides`
- `view_layout_template_assignments`

## 開発ルール

- Lint は常にエラー0を維持する。
- `pnpm run build` が通る状態を維持する。
- Tauri 側は `cargo check` が通る状態を維持する。
- 既存仕様にない廃止済みテーブルや廃止済みAPIを新規DB向け仕様へ戻さない。

## 関連ドキュメント

- DB仕様: `docs/DB_SPEC.md`
- ERメモ: `docs/ER_MEMO.md`
