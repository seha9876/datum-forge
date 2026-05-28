<script setup lang="ts">
import { computed, nextTick, ref } from "vue";

import { useConfirmDialog } from "../../../composables/useConfirmDialog";

import type {
  ViewNavFolderRecord,
  ViewNavTreeNode,
  ViewSelection
} from "../../../types";

/**
 * カスタム目次に必要なフォルダ一覧と操作関数です。
 * フォルダ作成・削除・選択の実処理は親が持ち、この部品は UI と入力状態を担当します。
 */
const props = defineProps<{
  isExpanded: (folderId: number) => boolean;
  nodes: ViewNavTreeNode[];
  onCreateFolder: (parentId: number | null, name: string) => Promise<void>;
  onDeleteFolder: (node: ViewNavTreeNode) => Promise<void>;
  onReorderFolderRecords: (
    folderId: number,
    records: ViewNavFolderRecord[]
  ) => Promise<void>;
  onSelectFolder: (node: ViewNavTreeNode) => void;
  onSelectFolderRecord: (record: ViewNavFolderRecord) => void;
  onToggleFolder: (folderId: number) => void;
  searchQuery: string;
  selectedItem: ViewSelection | null;
  showRecordIds: boolean;
}>();

const confirmDialog = useConfirmDialog();

/**
 * v-treeview に渡す 1 行分のデータ型です。
 * フォルダ行とレコード行で持つ情報が違うため、kind で分岐できるようにしています。
 */
type ManualTreeItem =
  | {
      id: string;
      kind: "folder";
      title: string;
      searchText: string;
      node: ViewNavTreeNode;
      children: ManualTreeItem[];
    }
  | {
      id: string;
      kind: "record";
      title: string;
      searchText: string;
      record: ViewNavFolderRecord;
    };

/** 目次上でドラッグ中の項目を、マウスへ追従させるプレビュー表示の情報です。 */
interface ManualTreeDragPreview {
  id: string;
  kind: ManualTreeItem["kind"];
  title: string;
}

/** Pointer Events でドラッグを始める前の押下情報です。クリック選択とドラッグを見分けるために使います。 */
interface ManualTreePendingDrag {
  item: ManualTreeItem;
  pointerId: number;
  startX: number;
  startY: number;
}

/** ドラッグ中のレコードが、どのレコードの前後へ入るかを示す表示状態です。 */
interface ManualTreeDropIndicator {
  targetItemId: string;
  position: "before" | "after";
}

/** クリックとドラッグを見分けるため、マウスがこの距離以上動いたらドラッグ扱いにします。 */
const DRAG_START_DISTANCE = 4;

/** ルートフォルダ作成フォームを表示しているかどうかです。 */
const isAddingRoot = ref(false);
/** ルートフォルダ名を保存する前の一時入力値です。 */
const rootFolderName = ref("");
/** 子フォルダ作成フォームを表示している親フォルダ ID です。 */
const addingParentId = ref<number | null>(null);
/** 子フォルダ名を保存する前の一時入力値です。 */
const childFolderName = ref("");
/** ドラッグ中の tree item ID です。行の見た目の強調に使います。 */
const draggingItemId = ref<string | null>(null);
/** ドラッグ中に表示する、独自の浮遊プレビューです。 */
const dragPreview = ref<ManualTreeDragPreview | null>(null);
const dragPreviewElement = ref<HTMLElement | null>(null);
/** 有効なレコード並び替え先に表示する、挿入位置の強調表示です。 */
const dropIndicator = ref<ManualTreeDropIndicator | null>(null);
/** まだドラッグ開始距離に達していない、押下中の候補情報です。 */
const pendingDrag = ref<ManualTreePendingDrag | null>(null);
let latestPreviewPoint = { x: 0, y: 0 };
let previewAnimationFrame: number | null = null;

/** フォルダ行を v-treeview 内で一意に見分ける ID を作ります。 */
function folderItemId(folderId: number) {
  return `folder:${folderId}`;
}

/** フォルダ内レコード行を v-treeview 内で一意に見分ける ID を作ります。 */
function folderRecordItemId(folderRecordId: number) {
  return `folder-record:${folderRecordId}`;
}

/** `1:田中 太郎` のような保存済みラベルから、表示用の名前部分を取り出します。 */
function folderRecordLabel(record: ViewNavFolderRecord) {
  return record.recordLabel.replace(/^\d+:/, "") || record.recordLabel;
}

/** 削除確認で表示するため、指定フォルダ配下にある子孫フォルダ数を数えます。 */
function countDescendants(node: ViewNavTreeNode): number {
  return node.children.reduce(
    (total, child) => total + 1 + countDescendants(child),
    0
  );
}

/**
 * フォルダ行を検索するときに使う文字列です。
 * 祖先フォルダ名も含めることで、親フォルダ名で配下の項目も探しやすくします。
 */
function folderSearchText(node: ViewNavTreeNode, ancestors: string[]) {
  return [...ancestors, node.name].join(" ");
}

/**
 * フォルダ内レコードを検索するときに使う文字列です。
 * レコード名、ID、テーブル名、祖先フォルダ名をまとめて検索対象にします。
 */
function folderRecordSearchText(
  record: ViewNavFolderRecord,
  label: string,
  ancestors: string[]
) {
  return [
    ...ancestors,
    record.recordId,
    record.recordLabel,
    label,
    record.tableName,
    record.tableDisplayName
  ].join(" ");
}

/**
 * v-treeview の検索で、画面表示用のタイトルではなく searchText を見ます。
 * カスタム目次はフォルダ名・祖先フォルダ名・レコード情報を searchText にまとめているため、
 * Vuetify の custom-filter に渡る filter key の値だけを見て検索漏れを防ぎます。
 */
function searchManualTreeItem(value: string, query: string) {
  const keyword = query.trim().toLocaleLowerCase();
  if (!keyword) {
    return true;
  }

  return value.toLocaleLowerCase().includes(keyword);
}

/**
 * 保存済みフォルダを、v-treeview が扱える親子データへ変換します。
 * 子フォルダを先に、その下にフォルダへ紐づけたレコードを並べます。
 */
function buildFolderItem(
  node: ViewNavTreeNode,
  ancestors: string[] = []
): ManualTreeItem {
  const currentPath = [...ancestors, node.name];

  return {
    id: folderItemId(node.id),
    kind: "folder",
    title: node.name,
    searchText: folderSearchText(node, ancestors),
    node,
    children: [
      ...node.children.map((child) => buildFolderItem(child, currentPath)),
      ...node.records.map((record) => {
        const label = folderRecordLabel(record);

        return {
          id: folderRecordItemId(record.id),
          kind: "record" as const,
          title: label,
          searchText: folderRecordSearchText(record, label, currentPath),
          record
        };
      })
    ]
  };
}

/** 親から渡されたフォルダ一覧を、v-treeview 標準のネスト構造へ変換します。 */
const treeItems = computed(() =>
  props.nodes.map((node) => buildFolderItem(node))
);

/** v-treeview に渡す展開中フォルダ ID です。検索中は一致項目が隠れないよう全展開します。 */
const openedFolderIds = computed(() => {
  const opened: string[] = [];

  function collect(node: ViewNavTreeNode) {
    if (props.searchQuery.trim() || props.isExpanded(node.id)) {
      opened.push(folderItemId(node.id));
    }

    node.children.forEach(collect);
  }

  props.nodes.forEach(collect);
  return opened;
});

/** 選択中フォルダ/レコードを、一覧側で強調表示できる ID に変換します。 */
const activatedItemIds = computed(() => {
  const selected = props.selectedItem;
  if (!selected) {
    return [];
  }

  if (selected.type === "folder") {
    return [folderItemId(selected.folderId)];
  }

  const selectedRecord = selected;
  const matchedRecordIds: string[] = [];

  function collect(node: ViewNavTreeNode) {
    for (const record of node.records) {
      if (
        record.tableId === selectedRecord.tableId &&
        record.recordId === selectedRecord.recordId
      ) {
        matchedRecordIds.push(folderRecordItemId(record.id));
      }
    }

    node.children.forEach(collect);
  }

  props.nodes.forEach(collect);
  return matchedRecordIds;
});

/**
 * v-treeview から渡された ID に対応するフォルダ行を探します。
 * 再帰的に children をたどるので、深い階層のフォルダも見つけられます。
 */
function findFolderItem(
  items: ManualTreeItem[],
  itemId: string
): Extract<ManualTreeItem, { kind: "folder" }> | null {
  for (const item of items) {
    if (item.kind === "folder" && item.id === itemId) {
      return item;
    }

    if (item.kind === "folder") {
      const childMatch = findFolderItem(item.children, itemId);
      if (childMatch) {
        return childMatch;
      }
    }
  }

  return null;
}

/**
 * v-treeview から渡された ID に対応するレコード行を探します。
 * 見つからない場合は、フォルダ行などが押されたものとして null を返します。
 */
function findRecordItem(
  items: ManualTreeItem[],
  itemId: string
): Extract<ManualTreeItem, { kind: "record" }> | null {
  for (const item of items) {
    if (item.kind === "record" && item.id === itemId) {
      return item;
    }

    if (item.kind === "folder") {
      const childMatch = findRecordItem(item.children, itemId);
      if (childMatch) {
        return childMatch;
      }
    }
  }

  return null;
}

function findFolderNodeById(
  nodes: ViewNavTreeNode[],
  folderId: number
): ViewNavTreeNode | null {
  for (const node of nodes) {
    if (node.id === folderId) {
      return node;
    }

    const childMatch = findFolderNodeById(node.children, folderId);
    if (childMatch) {
      return childMatch;
    }
  }

  return null;
}

/**
 * 現在のポインター位置から、同じフォルダー内レコードへの有効な挿入先を求めます。
 * tooltip の activator 構造を変えないため、レコード名 span に付けた data 属性だけを判定対象にします。
 */
function getRecordDropTarget(event: PointerEvent) {
  const pending = pendingDrag.value;
  // フォルダー自体のドラッグは保存対象外です。並び替えるのはフォルダー内レコードだけです。
  if (!pending || pending.item.kind !== "record") {
    return null;
  }

  // elementFromPoint でマウス直下の DOM を拾い、レコード名 span かどうかを調べます。
  // 今の判定範囲が小さく感じる場合は、この data 属性を付ける範囲を広げるのが調整点です。
  const targetElement = document
    .elementFromPoint(event.clientX, event.clientY)
    ?.closest<HTMLElement>("[data-manual-record-item-id]");
  const targetItemId = targetElement?.dataset.manualRecordItemId;
  // 自分自身へのドロップは順序が変わらないため、候補にしません。
  if (!targetElement || !targetItemId || targetItemId === pending.item.id) {
    return null;
  }

  const targetItem = findRecordItem(treeItems.value, targetItemId);
  // 別フォルダーのレコードへは移動させず、同じフォルダー内だけを並び替えます。
  if (
    !targetItem ||
    targetItem.record.folderId !== pending.item.record.folderId
  ) {
    return null;
  }

  // 対象レコード名の上半分なら前、下半分なら後ろに入れるための判定です。
  const targetRect = targetElement.getBoundingClientRect();
  const position =
    event.clientY > targetRect.top + targetRect.height / 2 ? "after" : "before";

  return { targetElement, targetItem, position } as const;
}

/** ルートフォルダ作成フォームを開閉します。閉じるときは入力値も捨てます。 */
function toggleRootForm() {
  isAddingRoot.value = !isAddingRoot.value;
  if (!isAddingRoot.value) {
    rootFolderName.value = "";
  }
}

/** 入力された名前でルートフォルダを作成し、保存後にフォームを閉じます。 */
async function submitRootFolder() {
  const trimmedName = rootFolderName.value.trim();
  if (!trimmedName) {
    return;
  }

  await props.onCreateFolder(null, trimmedName);
  rootFolderName.value = "";
  isAddingRoot.value = false;
}

/** 子フォルダ作成フォームを、指定したフォルダの直下に表示します。 */
function toggleChildForm(node: ViewNavTreeNode) {
  if (addingParentId.value === node.id) {
    addingParentId.value = null;
    childFolderName.value = "";
    return;
  }

  addingParentId.value = node.id;
  childFolderName.value = "";
}

/** 入力された名前で子フォルダを作成し、保存後にフォームを閉じます。 */
async function submitChildFolder(parentId: number) {
  const trimmedName = childFolderName.value.trim();
  if (!trimmedName) {
    return;
  }

  await props.onCreateFolder(parentId, trimmedName);
  childFolderName.value = "";
  addingParentId.value = null;
}

/** 削除前に確認ダイアログを出し、OK なら親から渡された削除処理を呼びます。 */
async function confirmDeleteFolder(node: ViewNavTreeNode) {
  const descendantCount = countDescendants(node);
  const message =
    descendantCount > 0
      ? `「${node.name}」と配下のフォルダ ${descendantCount} 件を削除しますか？`
      : `「${node.name}」を削除しますか？`;

  const confirmed = await confirmDialog.open({
    title: "フォルダの削除",
    message,
    confirmText: "削除",
    color: "error"
  });

  if (!confirmed) {
    return;
  }

  await props.onDeleteFolder(node);
}

/**
 * v-treeview 側でフォルダの開閉が変わったときに呼ばれます。
 * 検索中は一時的な全展開なので、親の展開状態は変更しません。
 */
function updateOpenedFolders(nextOpened: unknown[]) {
  if (props.searchQuery.trim()) {
    return;
  }

  const nextOpenedSet = new Set(nextOpened.map(String));

  function sync(node: ViewNavTreeNode) {
    const itemId = folderItemId(node.id);
    const shouldBeOpen = nextOpenedSet.has(itemId);

    if (shouldBeOpen !== props.isExpanded(node.id)) {
      props.onToggleFolder(node.id);
    }

    node.children.forEach(sync);
  }

  props.nodes.forEach(sync);
}

/** tree item の ID から対象を探し、フォルダまたは登録済みレコードの選択処理を呼びます。 */
function selectTreeItemById(itemId: string) {
  const folderItem = findFolderItem(treeItems.value, itemId);

  if (folderItem) {
    props.onSelectFolder(folderItem.node);
    return;
  }

  const recordItem = findRecordItem(treeItems.value, itemId);
  if (recordItem) {
    props.onSelectFolderRecord(recordItem.record);
  }
}

/** click:select で渡された ID を、共通の選択処理へ渡します。 */
function selectTreeItem(event: { id: unknown }) {
  selectTreeItemById(String(event.id));
}

/**
 * v-treeview はフォルダのような親ノードを activated で強調表示します。
 * そのため activated の更新も拾い、旧実装と同じくフォルダクリックで右側画面を切り替えます。
 */
function selectActivatedTreeItem(nextActivated: unknown[]) {
  const itemId = nextActivated[nextActivated.length - 1];
  if (itemId === undefined || itemId === null) {
    return;
  }

  selectTreeItemById(String(itemId));
}

/**
 * requestAnimationFrame で独自プレビューの座標を反映します。
 * pointermove ごとに直接 DOM 更新せず、描画タイミングへ寄せて追従を安定させます。
 */
function applyDragPreviewPosition() {
  previewAnimationFrame = null;

  if (!dragPreviewElement.value) {
    return;
  }

  dragPreviewElement.value.style.transform = `translate3d(${
    latestPreviewPoint.x + 4
  }px, ${latestPreviewPoint.y + 4}px, 0)`;
}

function scheduleDragPreviewPosition() {
  // すでに次フレームの更新予約がある場合は、重複予約せず最新座標だけを使います。
  if (previewAnimationFrame !== null) {
    return;
  }

  previewAnimationFrame = window.requestAnimationFrame(
    applyDragPreviewPosition
  );
}

function cancelDragPreviewPosition() {
  // ドラッグ終了時に、まだ実行されていないプレビュー更新予約を取り消します。
  if (previewAnimationFrame === null) {
    return;
  }

  window.cancelAnimationFrame(previewAnimationFrame);
  previewAnimationFrame = null;
}

/**
 * ドラッグの候補を記録します。
 * すぐにドラッグ扱いにすると通常クリックの選択を邪魔するため、移動距離を見てからプレビューを出します。
 */
function beginExperimentalPointerDrag(
  event: PointerEvent,
  item: ManualTreeItem
) {
  // 左クリック以外はドラッグ開始として扱いません。
  if (event.button !== 0) {
    return;
  }

  event.preventDefault();

  // ここではまだドラッグ確定にせず、押し始めた位置だけを保存します。
  // pointermove で一定距離を超えたら、クリックではなくドラッグとして扱います。
  pendingDrag.value = {
    item,
    pointerId: event.pointerId,
    startX: event.clientX,
    startY: event.clientY
  };

  const target = event.currentTarget;
  if (target instanceof HTMLElement) {
    // マウスが要素外へ少し出ても pointermove/up を受け取れるようにします。
    target.setPointerCapture(event.pointerId);
  }
}

/**
 * ドロップ時に、同じフォルダー内のレコード順を before/after 判定どおりに作り直して保存します。
 * フォルダー行や別フォルダーへのドロップは保存対象にしません。
 */
async function reorderDraggedRecord(event: PointerEvent) {
  const pending = pendingDrag.value;
  // プレビューが出ていない場合は、クリック扱いで終わったものとして保存しません。
  if (!pending || !dragPreview.value || pending.item.kind !== "record") {
    return;
  }

  const sourceItem = pending.item;
  const dropTarget = getRecordDropTarget(event);
  // 有効な挿入先がなければ、順序は変えません。
  if (!dropTarget) {
    return;
  }

  const folder = findFolderNodeById(props.nodes, sourceItem.record.folderId);
  if (!folder) {
    return;
  }

  const sourceIndex = folder.records.findIndex(
    (record) => record.id === sourceItem.record.id
  );
  const targetIndex = folder.records.findIndex(
    (record) => record.id === dropTarget.targetItem.record.id
  );
  if (sourceIndex < 0 || targetIndex < 0 || sourceIndex === targetIndex) {
    return;
  }

  // props.nodes は親の状態なので直接変更せず、保存用の新しい配列を作ります。
  const nextRecords = [...folder.records];
  const [movedRecord] = nextRecords.splice(sourceIndex, 1);
  // 元の位置から取り除いた後は、後ろ方向へ動かすと targetIndex が 1 つ前にずれます。
  const adjustedTargetIndex =
    sourceIndex < targetIndex ? targetIndex - 1 : targetIndex;

  nextRecords.splice(
    adjustedTargetIndex + (dropTarget.position === "after" ? 1 : 0),
    0,
    movedRecord
  );

  // 保存成功後の sortOrder 同期は、親から渡された処理側で行います。
  await props.onReorderFolderRecords(folder.id, nextRecords);
}

/**
 * Pointer Events の座標で独自プレビューを更新します。
 * ネイティブ drag イベントより座標が安定するため、Tauri でもマウスへ追従しやすくなります。
 */
function moveExperimentalPointerDrag(event: PointerEvent) {
  const pending = pendingDrag.value;
  // 別の pointerId のイベントは、今回のドラッグ操作ではないため無視します。
  if (!pending || pending.pointerId !== event.pointerId) {
    return;
  }

  const movedX = event.clientX - pending.startX;
  const movedY = event.clientY - pending.startY;
  const movedDistance = Math.hypot(movedX, movedY);

  // 押下位置から少ししか動いていない間は、通常クリックの可能性があるので何もしません。
  if (!dragPreview.value && movedDistance < DRAG_START_DISTANCE) {
    return;
  }

  latestPreviewPoint = { x: event.clientX, y: event.clientY };

  if (!dragPreview.value) {
    // しきい値を超えた最初の move で、ドラッグ中の見た目へ切り替えます。
    draggingItemId.value = pending.item.id;
    dragPreview.value = {
      id: pending.item.id,
      kind: pending.item.kind,
      title: pending.item.title
    };
    void nextTick(() => scheduleDragPreviewPosition());
  } else {
    scheduleDragPreviewPosition();
  }

  // マウス直下が有効な挿入先なら、対象レコード名の上端/下端へ線を出します。
  const dropTarget = getRecordDropTarget(event);
  dropIndicator.value = dropTarget
    ? {
        targetItemId: dropTarget.targetItem.id,
        position: dropTarget.position
      }
    : null;

  event.preventDefault();
}

/** ドラッグまたはドラッグ候補が終わったら、並び替え保存を試みてから表示状態を解除します。 */
function endExperimentalPointerDrag(event?: PointerEvent) {
  if (event) {
    // pointerup 時点のマウス位置で、最後に見えていた挿入先と同じ判定を使って保存します。
    void reorderDraggedRecord(event).catch(() => undefined);
  }

  if (event) {
    const target = event.currentTarget;
    if (
      target instanceof HTMLElement &&
      target.hasPointerCapture(event.pointerId)
    ) {
      // setPointerCapture した要素を解放し、次の操作へ影響しないようにします。
      target.releasePointerCapture(event.pointerId);
    }
  }

  // ドラッグ表示とドロップ候補を必ず消して、通常表示へ戻します。
  pendingDrag.value = null;
  draggingItemId.value = null;
  dragPreview.value = null;
  dragPreviewElement.value = null;
  dropIndicator.value = null;
  cancelDragPreviewPosition();
}
</script>

<template>
  <!-- カスタム目次全体です。フォルダ作成ボタンと保存済みツリーを表示します。 -->
  <div>
    <!-- ルート直下に新しいフォルダを追加するためのボタンです。 -->
    <div>
      <v-btn
        prepend-icon="mdi-folder-plus-outline"
        variant="tonal"
        color="primary"
        @click="toggleRootForm"
      >
        フォルダ追加
      </v-btn>
    </div>

    <!-- ルートフォルダ名を入力する一時フォームです。 -->
    <v-card v-if="isAddingRoot" variant="outlined">
      <v-card-text>
        <v-text-field
          v-model="rootFolderName"
          bg-color="surface"
          color="primary"
          density="comfortable"
          variant="outlined"
          hide-details
          label="フォルダ名"
          @keydown.enter.prevent="submitRootFolder"
        />
      </v-card-text>
      <v-card-actions>
        <v-btn size="small" color="primary" @click="submitRootFolder">
          保存
        </v-btn>
        <v-btn size="small" variant="text" @click="toggleRootForm">
          キャンセル
        </v-btn>
      </v-card-actions>
    </v-card>

    <!-- 保存済みフォルダと登録済みレコードを、Vuetify の treeview で表示します。 -->
    <!-- searchText は Vuetify 内部 item の raw に残るため、raw.searchText を検索対象にします。 -->
    <v-treeview
      v-if="nodes.length > 0"
      :items="treeItems"
      item-title="title"
      item-value="id"
      item-children="children"
      :search="searchQuery"
      :filter-keys="['searchText']"
      :custom-filter="searchManualTreeItem"
      :opened="openedFolderIds"
      :activated="activatedItemIds"
      activatable
      density="compact"
      indent-lines
      variant="text"
      color="primary"
      expand-icon="mdi-chevron-right"
      collapse-icon="mdi-chevron-down"
      no-data-text="保存済みフォルダはまだありません。"
      @update:opened="updateOpenedFolders"
      @update:activated="selectActivatedTreeItem"
      @click:select="selectTreeItem"
    >
      <!-- 行の左側に、フォルダかレコードか分かるアイコンを出します。 -->
      <template #prepend="{ item }">
        <v-icon
          v-if="item.kind === 'folder'"
          size="18"
          icon="mdi-folder-outline"
        />
        <v-icon v-else size="17" icon="mdi-file-document-outline" />
      </template>

      <!-- 行の中央に、フォルダ名またはレコード名を表示します。 -->
      <template #title="{ item }">
        <v-tooltip v-if="item.kind === 'folder'" :text="item.title">
          <template #activator="{ props: tooltipProps }">
            <span
              v-bind="tooltipProps"
              class="manual-tree-drag-handle"
              @pointerdown="beginExperimentalPointerDrag($event, item)"
              @pointermove="moveExperimentalPointerDrag"
              @pointerup="endExperimentalPointerDrag"
              @pointercancel="endExperimentalPointerDrag"
            >
              {{ item.title }}
            </span>
          </template>
        </v-tooltip>
        <v-tooltip v-else :text="item.title">
          <template #activator="{ props: tooltipProps }">
            <span
              v-bind="tooltipProps"
              class="manual-tree-drag-handle"
              :class="{
                'manual-tree-drop-before':
                  dropIndicator?.targetItemId === item.id &&
                  dropIndicator.position === 'before',
                'manual-tree-drop-after':
                  dropIndicator?.targetItemId === item.id &&
                  dropIndicator.position === 'after'
              }"
              :data-manual-record-item-id="item.id"
              @pointerdown="beginExperimentalPointerDrag($event, item)"
              @pointermove="moveExperimentalPointerDrag"
              @pointerup="endExperimentalPointerDrag"
              @pointercancel="endExperimentalPointerDrag"
            >
              <span v-if="showRecordIds"> #{{ item.record.recordId }} </span>
              {{ item.title }}
            </span>
          </template>
        </v-tooltip>
      </template>

      <!-- フォルダ行の右側には、子フォルダ追加と削除の操作を置きます。 -->
      <template #append="{ item }">
        <div v-if="item.kind === 'folder'" class="manual-tree-actions">
          <v-tooltip text="子フォルダーを追加" location="bottom">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                size="small"
                density="comfortable"
                variant="plain"
                color="primary"
                icon="mdi-folder-plus-outline"
                aria-label="子フォルダーを追加"
                @click.stop="toggleChildForm(item.node)"
              />
            </template>
          </v-tooltip>

          <v-tooltip text="フォルダーを削除" location="bottom">
            <template #activator="{ props: tooltipProps }">
              <v-btn
                v-bind="tooltipProps"
                size="small"
                density="comfortable"
                variant="plain"
                color="error"
                icon="mdi-delete-outline"
                aria-label="フォルダーを削除"
                @click.stop="confirmDeleteFolder(item.node)"
              />
            </template>
          </v-tooltip>
        </div>
      </template>

      <!-- 子フォルダ追加中だけ、対象フォルダの直下に入力フォームを表示します。 -->
      <template #footer="{ item }">
        <v-card
          v-if="item.kind === 'folder' && addingParentId === item.node.id"
          variant="outlined"
        >
          <v-card-text>
            <v-text-field
              v-model="childFolderName"
              bg-color="surface"
              color="primary"
              density="comfortable"
              variant="outlined"
              hide-details
              label="子フォルダ名"
              @keydown.enter.prevent="submitChildFolder(item.node.id)"
            />
          </v-card-text>
          <v-card-actions>
            <v-btn
              size="small"
              color="primary"
              @click="submitChildFolder(item.node.id)"
            >
              保存
            </v-btn>
            <v-btn
              size="small"
              variant="text"
              @click="toggleChildForm(item.node)"
            >
              キャンセル
            </v-btn>
          </v-card-actions>
        </v-card>
      </template>
    </v-treeview>

    <!-- フォルダがまだ 1 件もないときの案内です。 -->
    <p v-else class="view-empty-hint">
      保存済みフォルダはまだありません。まずはルート直下に追加してください。
    </p>

    <!-- ドラッグ中だけ、ブラウザ標準ゴーストの代わりに小さなプレビューを表示します。 -->
    <Teleport to="body">
      <div
        v-if="dragPreview"
        ref="dragPreviewElement"
        class="position-fixed pointer-events-none manual-tree-drag-preview"
      >
        <v-card
          class="pointer-events-none d-inline-flex align-center ga-2 pa-2"
          color="surface"
          elevation="8"
          rounded="lg"
        >
          <v-icon
            size="17"
            :icon="
              dragPreview.kind === 'folder'
                ? 'mdi-folder-outline'
                : 'mdi-file-document-outline'
            "
          />
          <span class="text-truncate">{{ dragPreview.title }}</span>
        </v-card>
      </div>
    </Teleport>
  </div>
</template>
