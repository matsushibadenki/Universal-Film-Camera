# Media Index and Atomic Asset Manifest

更新日: 2026-08-27
実装: `crates/camera-core/src/media_index.rs`、`apps/camera/src-tauri/src/lib.rs`

## Status

- [Done] CapturedAsset schema v2を含むversioned JSON manifest
- [Done] `.partial`へのflush／sync後renameするatomic manifest保存
- [Done] Finalized／Incomplete／Failedを区別する決定論的Media index
- [Done] Still／Video finalizeとmanifest保存のtransaction境界
- [Done] manifest保存失敗時に完成resourceを`.incomplete`へ戻すrollback
- [Done] Tauri `get_media_index` command
- [Done] corrupt manifestを黙って無視せず構造化errorにするtest
- [Done] manifest IDのpath traversal拒否
- [Done] Media画面を英語、日本語、简体中文で実装
- [Done] All／Finalized／Incomplete／Failed filterと状態別diagnostic
- [Done] Media表示前にnative previewを停止し、camera復帰時に再開
- [Done] 320／375／414／768／1100pxでoverflow／折返しを検証
- [Done] Failed／Incompleteの確認付きcleanupとasset詳細dialog
- [Done] root直下のorphanをFailedとして記録するreconciliation
- [Next] Failed／Incompleteの再検査とcapture再試行操作
- [Later] thumbnail／proxy生成とindex pagination

## Directory contract

```text
captures/
  UFC-<id>.jpg|mov             Finalized media resource
  .manifests/
    UFC-<id>.json              FinalizedまたはFailed record
    UFC-<id>.json.partial      書込み途中。indexへ公開しない
  .incomplete/
    UFC-<id>.jpg|mov           Capturing、validation失敗、rollback済みresource
```

完成assetの順序は`mediaをcaptures/へrename → manifestをflush／sync → manifestをrename`とする。manifest保存に失敗した場合はmediaを`.incomplete`へ戻し、Tauri commandを成功させない。したがってmanifestのない完成resourceを新規capture成功として公開しない。

OS crashがmedia renameとmanifest renameの間に発生した場合はroot直下にorphanが残り得る。Media読込み時のreconciliationは対応拡張子を持つroot直下の通常ファイルだけを検査し、対応manifestがなければ`Failed` recordへ記録する。resourceは自動削除せず、利用者が詳細と診断理由を確認できる状態にする。

## Record states

`MediaIndexEntry` schema version 1はID、state、media type、resource path、任意の完全`CapturedAsset`、任意のerror、更新UTC時刻を持つ。

- `Finalized`: 完全なCapturedAssetが必須で、ID、media type、path、stateがrecordと一致する。
- `Failed`: assetを持たず、空でないerrorを必須とする。validationに失敗したresourceは`.incomplete`に残す。
- `Incomplete`: manifestのない`.incomplete`内の対応拡張子から合成する。更新時刻はfile modification timeを使う。

同一IDのFailed manifestとincomplete fileがある場合は、診断理由を持つFailed recordを優先する。未知拡張子、manifest用`.partial`、directoryはMedia assetとして列挙しない。

record IDはASCII英数字、ハイフン、アンダースコアだけを受理する。separator、dot、Unicode lookalikeをfilenameへ展開しない。

## Error policy

壊れたmanifest、未知schema、不整合なFinalized recordは一覧から隠さずindex全体をerrorにする。Media UIはこれを「assetなし」と表示せず、library repairが必要な状態として提示すること。

cleanupは`Failed`と`Incomplete`だけを対象とし、`Finalized`はcommand側でも拒否する。UIはasset ID、path、診断理由を詳細dialogに示し、別の確認dialogを経てから実行する。commandは安全なIDを要求し、canonical pathがcaptures配下にある通常ファイルであることを検証してresourceと対応manifestだけを削除する。root directory、外部path、symlinkの外部targetは対象にしない。

resource削除後にmanifest削除が失敗した場合は診断recordが残る順序を採る。逆順でmanifestだけを消してresourceを孤立させるより復旧可能性を優先する。cleanupもreconciliationもderivative graphの完成素材を自動削除しない。

## Media UI contract

Media画面は装飾的なgalleryではなく、撮影現場で状態を判別するCatalogueとする。thumbnail／proxyが未実装の間は偽の画像を表示せず、Photo／Video種別、状態、filename、更新時刻、解像度、duration、asset ID、失敗理由を表示する。

各cardの詳細操作はnative dialogを開き、state、更新時刻、path、format、duration、asset ID、schema、diagnosticを表示する。cleanup actionは`Failed`／`Incomplete`だけに出し、英語、日本語、简体中文の明示確認を必須とする。

native `AVCaptureVideoPreviewLayer`はWebViewより前面に出るため、Mediaを開く前にpreview sessionを停止する。停止に失敗した場合はMediaへ遷移せずerrorを表示する。カメラへ戻るとdevice discoveryからpreviewを再開する。録画中はMedia遷移を受理しない。

狭い画面ではfilterを2列、assetを1列にする。40rem以上ではfilterを4列、assetを2列、60rem以上では3列、90rem以上では4列とする。すべての操作領域は44px以上、coarse pointerでは48px以上を維持する。
