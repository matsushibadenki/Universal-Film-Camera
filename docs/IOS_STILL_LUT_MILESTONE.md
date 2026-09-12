# iOS Still LUT保存の節目

更新日: 2026-09-12

## 完了範囲

- [Done] 選択LUTをJPEGに焼き込み、MediaのthumbnailとPhotos書き出しが同じ加工済み画像を扱う。
- [Done] 内蔵16系統のlookと外部3D `.cube`（2〜65格子、4MiBまで）。内蔵は一般的な近似表現で、銘柄の測色再現ではない。
- [Done] `.cube`のred-fastest順序をnative／CPU preview／WebGL textureで統一。以前の外部LUTは赤・青軸が逆転する場合があった。外部LUTの見え方が変わるのはこの修正による。
- [Done] invalid／NaN／Infinity／重複・逆転domainの拒否、読み出し前のサイズ制限、選択を素早く変えたときの古いpayload上書き防止。
- [Next] Metal zero-copy native previewとVideoのLUT baking、各output color spaceのnative renderer接続。
- [Next] iPhone実機受け入れ。今回、端末install・実機撮影は行っていない。非iOS新規開発と2台検証は保留。

## 保存契約

`capture_photo(lutId)`は撮影開始前にLUTを解決して固定する。未指定／`none`は従来のJPEG保存経路を維持する。
有効時は `AVCapturePhotoOutput → .incomplete JPEG → Core Image → .look.jpg → probe → fsync → 原本一時pathへrename → finalize＋manifest`。
Core Imageのworking／outputをsRGBへ固定し、domainを正規化してCIColorCubeを適用する。
新しいJPEGは品質0.95で再符号化し、ImageIOのsource propertiesでEXIFを維持する。
画素の回転は行わずorientation tagを保持するため、既存の撮影寸法照合とthumbnail向き補正を維持する。

この経路の`original`は**LUT適用済みの完成JPEG**を指す。未加工コピーとの二重保存ではなく、LUTの後解除はできない。
未加工JPEGが必要な場合はLUTを`none`にする。既存写真への一括加工やRAW現像は行わない。
レンダー／検証／fsync失敗時は未加工JPEGを`.incomplete`に残してFailed記録を作り、自動Photos移動へ進まない。
manifest確定失敗は既存のrollback経路により完成JPEGをincompleteへ戻す。

`original.color.applied_lut`は後方互換のoptional metadataで、ID、cube size、normalized payload SHA-256、domain min/max、input/output encoding、renderer versionを保持する。
domainはf32値のround-trip可能な十進文字列。hashはファイルそのものではなく、version prefix・size・domain・red-fastest f32 sample列のlittle-endian byte列から計算する。
これはasset JSON manifest内の情報であり、Photosへ単体JPEGを書き出す際にLUT履歴を別途保存するものではない。

## 色管理の境界

今回のLUTは**sRGB display-referred入力／sRGB出力**として扱う。Log素材専用LUTの入力色空間を自動推測する機能はない。
モニターのRec.709／Display P3選択は引き続き表示設定であり、今回の写真出力はsRGB固定。
動画はまだ表示用LUTのみ。環境設定のLUTタブに三言語で明記した。
WebGL previewは8bit cube texture、nativeはfloat cubeのため量子化差があり、プレビューと保存の完全な測色一致は未達。

参考: [Apple CIColorCubeWithColorSpace](https://developer.apple.com/documentation/coreimage/cicolorcubewithcolorspace)（working color spaceの明示）。native実装はCIContextのsRGB working space＋CIColorMatrix／CIColorCubeを使用する。

## 検証

- Rust workspace全108 test合格（Core Imageテストは描画可能な通常macOS環境）。
- native画素test: identity LUTの色維持、白黒look、赤固定cube、domain倍率、EXIF orientation 6保持、埋め込みprofile、既存file上書き拒否、入力原本非変更。
- Web production build、iOS arm64 cargo check合格。
- UI: `http://127.0.0.1:1420/?control-fixture=1`、390×844／844×390、環境設定→LUT→白黒選択の状態変化、横overflowなし、Vite overlayなし。
- Browser plugin not available。frontend-testing-debuggingスキルに従いPlaywright＋既存Chromeを使用。page identity／非空表示／操作／screenshot確認済み。consoleは既存favicon.icoの404のみでアプリ例外なし。
- CPUの非対称RGB identity補間とWebGL用texture配列のred-fastest順を追加検証。ブラウザfixtureは実カメラやnative IPCの代替検証ではない。
- 証跡: `/private/tmp/ufc-lut-390.png`、`/private/tmp/ufc-lut-844.png`（一時ファイル）。

Core Imageはサンドボックスでcontext初期化できない場合がある。factoryのnilを検出し、panicではなく失敗として返す。
通常環境でのテスト成功はiPhone実機のGPU性能・発熱・48MPメモリ使用量を保証しない。
