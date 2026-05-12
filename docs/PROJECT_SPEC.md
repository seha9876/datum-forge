# PROJECT SPEC

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
- フォルダー内レコードを開くと、現在の有効テンプレートを使って表示する。

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
- フォルダー内レコードを開くときに割当がなければ、空のフォルダー専用テンプレートを自動作成して割り当てる。
- テンプレート編集ではカード枠の追加、移動、リサイズ、削除、スタイル編集を行う。
- テンプレート編集では任意のテーブル/レコードを選んで一時プレビューできる。プレビューはテンプレート本体に保存しない。
- テンプレート編集のプレビューでは、既存の `view_layout_card_column_bindings` を初期値として読み込み、一時紐付けで表示カラムを仮に変更できる。一時紐付けは保存しない。
- 一時紐付けUIは折りたたみ可能とし、未紐付けカードがある場合だけ自動展開する。
- テンプレート編集の非編集時は、プレビュー対象がある場合に実データを完成イメージとして表示する。編集時のみカード操作と一時紐付け操作を行える。
- レコード個別キャンバスでは、テンプレート選択やテンプレート編集は行わず、カード枠への個別差分だけを保存する。

#### カードとカラムの紐付け

- テンプレート本体はテーブル非依存のため、カード枠と表示カラムの対応は `view_layout_card_column_bindings` に `template_id + table_id + card_id -> column_id` として保存する。
- レコード表示時に対象テーブルの紐付けがない場合は、表示項目の紐付けUIを表示する。
- 紐付けUIではキャンバス上のカード位置を見ながら、カードごとに対象テーブルのカラムを選択できる。
- 紐付け済み後もレコード表示画面の `表示項目` から同じテーブル向けの紐付けを編集できる。
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
- 閲覧目次: `list_view_nav_nodes`, `create_view_nav_folder`, `delete_view_nav_folder`, `get_view_table_sections`, `add_view_nav_folder_records`, `remove_view_nav_folder_record`
- レイアウトテンプレート: `list_all_folder_layout_templates`, `list_view_layout_templates_for_folder`, `create_view_layout_template`, `rename_view_layout_template`, `duplicate_view_layout_template`, `delete_view_layout_template`, `assign_view_layout_folder_template`
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

