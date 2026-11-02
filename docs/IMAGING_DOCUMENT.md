# Imaging Document: LUT and monitoring settings

更新日: 2026-11-02

## 🟢 [Done] 現在の責務

- `AppState.imaging_document`がアプリプロセス内の正本を所有する。各WebViewはsnapshotを表示し、変更commandを送る。
- `get_imaging_document(legacyLut?)`は初回だけ保存ファイルを読み込む。ファイル未作成時のみ旧localStorageのLUT IDを検証して移行する。未知の旧IDは`none`にする。破損ファイル・未対応schemaは上書きせずエラーにする。
- `set_imaging_lut(selectedLut, expectedRevision)`は管理対象LUTを検証し、revision一致を確認する。`.json.partial`へのwrite／syncとrenameに成功してからメモリの正本を更新する。失敗時は前の正本を維持する。
- 保存先は`app_data_dir()/imaging-document.json`。schema 1は`schema_version`、`revision`、`selected_lut`と追加の`monitoring`／`capture_policy`／`shutter_sound`。画像・音声・LUT全サンプルはsnapshotへ含めない。追加field欠落を移行対象と判定する後方互換拡張（旧アプリへのdowngrade互換性は保証しない）。
- `imaging-document-changed`で小さいsnapshotを配信する。JSは古いrevisionを採用しない。競合時は保存を拒否して再取得する。通知失敗だけで保存成功を失敗扱いしない。
- 写真commandはWebViewの`lutId`を受け取らず、Rust正本のIDとLUT表を露光前に固定する。その後の選択変更は処理中の写真へ影響しない。
- Web専用開発時だけlocalStorageの従来経路を残す。ネイティブでの永続化失敗をlocalStorageへ黙ってfallbackしない。

ファイル更新は同一プロセスのMutexで直列化する。別プロセスが同じapp-dataファイルを同時更新する利用は対象外。外部LUTをアプリ外から削除した場合、撮影時のLUT解決はエラーとなり、無断で別LUTへ置き換えない。

## 検証

Rustテストは保存／復元、保存失敗時の既存設定保護、破損／未対応schema拒否、古いrevision拒否、撮影用コピーの不変性を確認する。並列テストのtemporary directoryはプロセスID＋atomic counter＋時刻で分離する。

2026-10-22: `npm run build`、`cargo test --workspace --offline`、`./scripts/check_ios_rust.sh`に合格。workspaceテストの既存Metal GPUケースはsandbox内でadapterを取得できないため、承認済みのsandbox外実行で再確認して合格した。追加Documentテスト4件はsandbox内でも合格。既存の手動GPU benchmark 1件は通常どおりignored。

## 🟠 [Next] 残る移行

LUTとmonitoring、Photos移動policy、シャッター音IDのRust移行は実装済み。native texture importと動画処理が次の主工程。カスタム音源の大容量blobは設定snapshotへ埋め込まない。

## 🟢 [Done] シャッター音IDと撮影snapshot

`shutter_sound`はoptional enum（standard／fresh／dslr／silent／custom／c137／f2／h503／m4／small）。`set_shutter_sound(sound, expectedRevision)`は未知のID・ファイルpathを受け付けない。旧選択の移行はfield欠落時のみ。設定更新はLUT・monitoring・転送設定を保持し、失敗時はUIがRust snapshotへ戻る。無音が非対応の場合に選択を非表示にするUIとnative capability checkは維持した。localeや国名から制限を回避する処理は追加しない。

`ImagingDocument.freeze_capture()`でLUT ID・転送policy・音IDを同じrevisionのDocumentからcloneする。写真receiptの`shutter_sound`はその音ID、動画receiptはnull。JSは現在のselect値ではなくreceiptで音を決める。保存のschemaは引き続き1の後方互換field追加で、旧binaryへのdowngradeは保証しない。

🔴 [Later] custom音源のversion固定、native音源ストア、native再生。今回のcustom IDはIndexedDB内の変更可能な音源を指すため、音源本体まで再現可能に固定されたとは扱わない。再生処理はWebViewのまま。Photos移動処理の完了待ちで撮影音が遅れる可能性も残る。

⭕️ [Pending] iPhoneの実音、Apple APIの無音対応／非対応、カスタム音源の入替、撮影途中の設定変更、転送完了まで時間がかかるケースを確認する。

検証（シャッター音追加、2026-11-02）: `npm run build`、`cargo test --workspace --offline`、`./scripts/check_ios_rust.sh`に合格。Documentテストは11件で、全10音IDのserde往復・未知ID／path拒否・同一snapshot内のLUT／音／転送policy固定・保存失敗／競合時の既存ファイル保護を確認。GPUテストとiOSのSwift cache書き込みは承認済みsandbox外実行。

frontend-testing-debuggingの手順でPlaywright＋Chromeを使用（Browser plugin not available: browser skill未提供）。`http://127.0.0.1:1420/?control-fixture=1`、390×844／844×390で、音選択変更→再読込で復元→保存失敗時に以前の音へ戻る→撮影タブのエラー表示を確認。タイトル・非空表示・framework overlayなし・page errorなしを確認。consoleは既存favicon 404のみ。`/private/tmp/camera-sound-mobile.png`と`/private/tmp/camera-sound-landscape.png`が証跡。native command模擬によるUI試験であり、実音の測定・iPhone実機試験ではない。

## 🟢 [Done] 撮影後Photos移動policy

Documentの`capture_policy.move_to_photos`は省略可能な後方互換field。欠落時だけ`get_imaging_document(legacyCapturePolicy?)`で移行し、未指定ならfalse。`set_capture_policy(policy, expectedRevision)`は既存のrevision・Mutex・原子的保存を使う。保存失敗時はUIを戻し、英語／日本語／简体中文の既存エラー見出しを撮影タブ内に表示する。

写真はcommand開始時、動画は開始時にpolicyをcloneし、動画終了時の設定は参照しない。原本検証とmanifest確定後にRustが転送処理を実行する。Photos保存成功後だけ既存の`MediaIndex.delete_finalized`へ進む。転送失敗は撮影エラーに変換せず、receipt内の`auto_transfer.error`で通知する。削除失敗ならPhotos側とアプリ側にコピーが残る可能性を既存errorで示す。

JSはreceiptの`auto_transfer`（`moved`、optional `error`）を表示するだけで、転送を再実行しない。古いreceiptに結果がなければ自動削除を行わない。receiptは撮影commandの応答であり、撮影manifestのschemaは変更していない。process終了後の自動retry／冪等なPhotos asset識別は未実装。

⭕️ [Pending] iPhone実機のPhotos権限拒否、容量不足、録画中の設定変更、Photos成功後の原本削除、削除失敗・途中終了を受け入れ確認する。非iOSのPhotos転送は未対応のまま。

検証（撮影policy追加）: Web build、Rust workspace全体テスト、iOS arm64 compileに合格。Documentテストは9件合格し、default opt-out、設定の永続化、録画中変更後もclone済みpolicyが不変、保存失敗時の既存設定保護を確認した。Playwright＋Chromeのnative command模擬環境では、自動転送checkboxを変更→再読込で復元→保存失敗時に元へ戻る→撮影タブ内にエラーを表示する流れを確認。390×844／844×390でスクロール後の通知表示も確認した。runtime error／framework overlayなし、consoleは既存favicon 404のみ。画像証跡は`/private/tmp/camera-policy-mobile.png`と`/private/tmp/camera-policy-landscape.png`。Browser plugin not available（browser skill未提供）のためPlaywrightを使用。これは実際のPhotos転送試験を代替しない。

## 🟢 [Done] 2026-11-02: Monitoring

`monitoring`は`color_space`（rec709／display-p3／srgb）、`look_enabled`、`guide_style`（thirds／grid／diagonal）、`peaking_color`（cyan／red／green／yellow／magenta／white）。Rustの`set_monitoring_settings(patch, expectedRevision)`で指定フィールドだけを更新し、未知のfieldと不正な選択値を拒否する。LUTの変更と同じrevisionを使い、他フィールドの消失を防ぐ。

旧Documentのmonitoring欠落時だけ、`get_imaging_document(legacyMonitoring?)`で旧WebView設定を移行する。不正な旧設定一式は既定値へ戻す。保存済みmonitoringが不正ならエラーとし、黙って置換しない。部分更新の失敗時は以前の永続Documentを維持し、UIは最新snapshotを再取得する。保存エラーの見出しは英語・日本語・简体中文で用意した。

WebViewは設定の表示と操作を担当し、native commandではlocalStorageへfallbackしない。ブラウザ単独の開発環境だけ従来localStorageを使う。monitoring通知でLUT IDが変わらない場合、LUT sample再取得を省く。色空間選択は保存されるが、native output transformや保存写真のプロファイルが新たに実装されたという意味ではない。

UI QA: Browser plugin not available（browser skill未提供）のためPlaywright＋Chromeを使用。URLは`http://127.0.0.1:1420/?control-fixture=1`、390×844／844×390。native commandの模擬応答を使い、4設定の変更・再読込復元、保存失敗時のrollback、日本語エラー、通知の新旧順序を確認する。スクリーンショットは`/private/tmp/camera-monitoring-mobile.png`と`/private/tmp/camera-monitoring-landscape.png`。実機のsafe area、カメラ映像、GPU色変換はこのQAの対象外。

検証結果（2026-11-02）: Web production build、Rust workspace全体テスト、iOS arm64 compile checkに合格。Documentテストは7件合格。UI模擬テストも合格し、ページタイトル・非空表示・framework overlayなし・runtime errorなし・通知の逆順耐性・保存失敗rollbackを確認した。consoleには既存の`/favicon.ico` 404のみあり、アプリ動作のエラーはなかった。MetalテストとSwift cache書き込みを伴うiOS checkは承認済みsandbox外実行。端末インストールは未実施。

CVPixelBuffer→Metal／wgpuのnative描画と動画へのLUT焼き込みは別工程。この変更によるpreview FPS向上は主張しない。

## ⭕️ [Pending] iPhone受け入れ

内蔵／外部LUT選択後の再起動復元、写真への焼き込み結果、設定保存失敗時の表示と再操作、Photos転送との組み合わせを実機で確認する。署名済みアプリの端末インストールは今回未実施。
