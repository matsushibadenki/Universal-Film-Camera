# CapturedAsset Contract and Capture Validation

更新日: 2026-08-26
実装: `crates/camera-core/src/asset.rs`、`apps/camera/src-tauri/src/lib.rs`

## 現在地

- [Done] Still／Video共通の`CapturedAsset` schema version 2
- [Done] `Incomplete → probe → Finalized`の公開境界
- [Done] JPEGの寸法、bit depth、EXIF orientation 1–8、ICC／EXIF sRGB probe
- [Done] QuickTime／ISO BMFFのvideo／audio codec、寸法、rational FPS、duration、track matrix、audio channel／sample rate、`colr` probe
- [Done] 選択camera formatと保存resourceの寸法／FPS照合
- [Done] container、video codec、audio track、正duration、color宣言のvalidation result
- [Done] synthetic JPEG／MOV fixture testと実asset用`probe_asset` diagnostic
- [Done] UI端末姿勢をPreview／Photo／Movie connectionへ同期し、preview mirrorと保存mirrorを分離
- [Done] EXIF orientation 1–8とMOV quarter-turn／mirror行列の全組合せfixture
- [Next] iPhone／iPadのportrait／upside-down／front-camera mirrorを実機検証
- [Done] derivativeへparent resource、render snapshot、engine version、seedを保存する型・検証・JSON往復契約
- [Next] asset manifest／sidecarとMedia indexを永続化

## 公開条件

StillとVideoはいずれも最初に`captures/.incomplete`へ保存する。JPEGのflushまたはAVFoundation recording delegate完了後、Rust probeが保存streamを読み、選択formatとの照合に合格してから完成pathへrenameする。

```text
AVFoundation output
  → captures/.incomplete/UFC-<id>.jpg|mov
  → camera-core probe
  → required checks
  → CapturedAsset(state = finalized)
  → captures/UFC-<id>.jpg|mov
```

寸法、container、Video codec、Video FPS、正duration、Video audio trackのいずれかが不一致または欠落した場合、Tauri commandはerrorを返し、assetを完成directoryへ移動しない。color metadata欠落は現段階では`warning`とし、欠落を推測して補わない。

## Schema version 2

`CapturedAsset`は`schema_version`、asset ID、media type、state、`original_resource_id`、original resource、derivatives、capture metadata、validation、UTC作成時刻を返す。`original.path`が完成resourceの正本であり、旧IPCのtop-level `path`は廃止した。

各`CapturedDerivative`は一意なresource ID、用途、別pathの`MediaResource`、UTC作成時刻に加え、次のprovenanceを保持する。

- 既存originalまたは先に追加済みderivativeを指す`parent_resource_id`
- Pipelineと必要Profile closureのcontent hashを含む`RenderProfileSnapshot`
- rendererの`engine_version`
- すべての確率的処理に渡す`seed`

追加APIはFinalized assetだけを受理し、未知parent、重複ID、original／既存derivativeと同じpath、不正または未整列のsnapshotを拒否する。parentは追加時点で存在するresourceに限定されるため、derivative graphは循環しない。schema v1の永続manifestはまだ製品に存在しないため、自動migrationは設けずschema v2を最初の永続化候補とする。

`MediaResource`は次を保持する。

- byte length、container、video／audio codec
- encoded pixel width／height、bit depth
- EXIF orientation、明示tag有無、rotation degree、mirror
- rational frame rate、duration
- audio channel count／sample rate
- embedded profileまたはprimaries／transfer／matrix／range

`CaptureMetadata.selected_format`はAVFoundationが撮影開始直前に返したactive width／height／FPSである。保存後probeはこの値とresourceを照合する。FPSは短い収録でのtimestamp揺らぎを考慮し、絶対差0.12 FPS以内を初期許容差とする。

## macOS出力構成

`AVCapturePhotoOutput`と`AVCaptureMovieFileOutput`を同時接続した状態では、内蔵cameraで720p session presetが受理されず、MovieFileOutputが1920×1080またはsquare formatへ再交渉することを実機で確認した。

Video開始時はPhotoOutputをsessionから外し、標準解像度では対応する`AVCaptureSessionPreset`をcommitしてからdevice `activeFormat`とframe durationを最終適用する。Video後のStill撮影時はPhotoOutputを戻し、同じ選択formatを再適用する。format mutation、output切替、録画開始／停止は既存`operation_lock`内で直列化する。

## Rotation／Mirror契約

`CaptureOrientation`はrotationを0／90／180／270度に限定し、`preview_mirrored`と`capture_mirrored`を別fieldにする。Web UIはScreen Orientationの角度とcamera positionから要求値を生成する。front cameraはpreviewだけを鏡像にし、Still／Videoの保存物は既定で非鏡像とする。

Apple backendは同じrotationを`AVCaptureVideoPreviewLayer`、`AVCapturePhotoOutput`、`AVCaptureMovieFileOutput`の各connectionへ設定する。AVFoundationはpreviewをlayer transform、StillをEXIF、VideoをQuickTime track matrixとして表現するため、pixelを別途回転しない。output切替やactive format再適用後にも保持値を再設定し、二重回転を防ぐ。

## 2026-08-24実機検証

内蔵camera、macOS debug bundle、1280×720／24 FPSで次を確認した。

```text
settings restore
  1280×720 / 24 FPS: passed after app relaunch

Still
  JPEG / 1280×720 / 8-bit / EXIF sRGB: passed
  orientation: implicit 1 / rotation 0° / not mirrored

Video
  QuickTime MOV / H.264 / 1280×720: passed
  measured average FPS: 100000/4167 ≈ 23.998
  AAC / 16 kHz / mono: passed
  color: BT.709 primaries / transfer / matrix
  duration: positive

Video → Still output restoration
  JPEG / 1280×720: passed

orientation connection同期後の再検証
  Preview start / Still / Video: passed
  JPEG: rotation 0° / not mirrored
  MOV track matrix: rotation 0° / not mirrored
```

検証中、選択formatと異なるMOVを2本検出し、意図どおり`.incomplete`へ隔離した。これはvalidation failure時のcleanup policy実装前の診断artifactであり、Media画面へは公開されない。自動cleanup／quarantine UIは`[Next]`である。

## Diagnostic

外部toolなしでcamera-coreのprobe結果を確認できる。

```bash
cargo run -p camera-core --example probe_asset -- photo /path/to/capture.jpg
cargo run -p camera-core --example probe_asset -- video /path/to/capture.mov
```

`ffprobe`、`sips`、`mdls`は実機のcross-checkには使えるが、製品の保存成功条件は外部commandへ依存しない。

## 制限

- connectionへのrotation／mirror設定とmetadata probeは実装済み。macOS 0度・非鏡像の保存結果は確認済みだが、iOSのportrait、upside-down、front-camera鏡像は実機未検証。
- MOV probeはQuickTime／一般的なISO BMFF sample tableを対象とする。fragmented MP4、複数video track、edit listによるpresentation duration、VFR詳細判定は未対応。
- `duration_ms`はmovie headerを優先し、なければvideo media headerへfallbackする。A/V start offsetとtimestamp単調性はまだ検証していない。
- color codeが未知の場合は文字列`unknown`として保持し、BT.709などへ推測変換しない。
- derivativeの再現情報はRust／IPC型とJSON round-tripまで実装済み。asset manifestのatomic file保存とMedia indexへの登録は次工程であり、現在はアプリ再起動後に復元されない。
