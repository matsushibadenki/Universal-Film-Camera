# Universal Imaging Camera Roadmap

更新日: 2026-08-24
対象: macOS / iOS / Android / Windows / Linux  
正本仕様: [`Universal Film & Color Imaging Engine.md`](Universal%20Film%20%26%20Color%20Imaging%20Engine.md)

## Status legend

- [Done] implemented in the current codebase
- [Next] high-priority unfinished work
- [Later] planned, but not the closest next step

この文書はプロジェクト全体の進捗indexである。個別機能の技術契約と検証記録は各詳細文書を正本とする。「設計済み」と「実装済み」を混同せず、コードまたは検証可能なartifactが存在する項目だけを`[Done]`にする。

## 現在地

現在は、macOS向けプロカメラの最初のnative縦切りと、Universal Imaging Pipelineの記述・Profile基盤まで完了している。実際のFilm／Sensor画素処理renderer、GPU scope、iOS／Android、Windows／Linux backendは未完了である。

| 領域 | 現在の段階 | 最も近い完了条件 |
|---|---|---|
| macOS camera | [Done] validated native MVP | iOS姿勢実機検証とMedia管理 |
| Professional UI | [Done] capture-first shell | native／GPU monitoring overlayを接続 |
| Imaging Pipeline model | [Done] schema v1 + render snapshot | CPU finishing縦切りを追加 |
| Profile system | [Done] typed Profiles + loader + migration registry | assetへsnapshotを保存 |
| Film renderer | [Done] CPU synthetic finishing縦切り | measured Print／ColorCheckerを追加 |
| GPU renderer | [Later] architecture only | wgpu texture pipelineとreference比較 |
| Nearby sharing | [Later] transport方針確定 | CapturedAsset／Media管理後にpeer転送MVP |
| iOS／Android | [Next] 未初期化 | Tauri mobile project、権限、native preview |
| Windows／Linux | [Later] 未実装 | platform camera backendの最初のpreview |

プロジェクト全体の最終構想に対する単一の進捗率は使用しない。macOS Camera MVP、Imaging Pipeline、科学renderer、各OS backendは規模と完了条件が異なるため、上表とmilestone単位で判断する。

## 依存順序

```text
Profile Schema / Loader
  → Lens・Sensor・Development・Display typed Profile
  → scene_linear → virtual_exposure adapter [Done]
  → CPU Reference executor
  → deterministic fixtures / conformance
  → GPU renderer
  → native zero-copy preview / encode
```

Camera captureの正確性はrendererと並行して進める。

```text
Selected camera format
  → Still / Video orientation and metadata
  → CapturedAsset lifecycle
  → Original + processed derivative management
  → Imaging Pipeline capture integration
  → Nearby Peer Transfer
```

## Milestone 0 — Architecture and workspace

- [Done] Cargo workspaceとTauri 2 desktop shell
- [Done] `media-core`のframe、pixel format、color metadata、native handle境界
- [Done] `camera-core`のStill／Video、capability、state machine、backend trait
- [Done] `film-core`のACEScg image、FilmRecipe、quality、renderer trait
- [Done] `imaging-core`のCamera、Lens、Film／Digital Sensor、Development、Print／Output、Display node
- [Done] SignalDomainによるFilm／Digital接続validation
- [Done] Version 0.2規範仕様、性能予算、Still／Video Asset Contract

詳細: [`IMPLEMENTATION_HANDOFF.md`](IMPLEMENTATION_HANDOFF.md)、[`IMAGING_PIPELINE_ARCHITECTURE.md`](IMAGING_PIPELINE_ARCHITECTURE.md)

## Milestone 1 — macOS native camera MVP

- [Done] AVFoundation camera／microphone権限とdevice discovery
- [Done] WKWebView上のnative `AVCaptureVideoPreviewLayer`
- [Done] responsive preview resizeとtitle bar／Safe Area補正
- [Done] JPEG Still captureと原子的保存
- [Done] H.264＋AAC MOV recordingとdelegate完了後のfinalize
- [Done] Photo／Videoを同格にした中央正円capture control
- [Done] resolution／FPS能力列挙、active format表示、1280×720／24 FPS適用
- [Done] 録画中のmode／format変更拒否
- [Done] 選択formatでJPEG／MOVを生成し、寸法・FPS・色metadataを保存後probeで検証
- [Done] format設定のdevice別永続化とsession開始時の復元
- [Done] Still／Video共通`CapturedAsset`、Incomplete→probe→Finalized公開境界
- [Done] EXIF orientation／MOV track rotationと保存metadataの共通読出し
- [Done] UI端末姿勢をPreview／Photo／Movie connectionへ同期し、preview／capture mirrorを分離
- [Done] EXIF 1–8とMOV quarter-turn／mirror行列のfixture検証
- [Next] iOS実機でportrait／upside-down／front-camera mirror caseを検証
- [Next] HEIF／RAW、codec／container／bitrate／audio channelの能力モデル
- [Next] window close、sleep、background、device切断時の復旧
- [Later] `AVCaptureVideoDataOutput → CVPixelBuffer → Metal texture`

詳細: [`APPLE_CAMERA_BACKEND.md`](APPLE_CAMERA_BACKEND.md)

Asset contract: [`CAPTURED_ASSET_CONTRACT.md`](CAPTURED_ASSET_CONTRACT.md)

## Milestone 2 — Professional camera UI

- [Done] 技術的・暗色・撮影画面優先のresponsive layout
- [Done] 320／375／414／768／1280pxのlayout検証
- [Done] right rail／bottom railで中央正円capture controlを維持
- [Done] scope／monitor tools menuをnative preview外へ配置
- [Done] 英語、日本語、简体中文の主要UI
- [Done] 実機active formatとmanual control可否を表示
- [Done] 対応組合せから生成するformat／FPS panel
- [Next] Still／Video別output presetと残容量表示
- [Next] 保存assetを確認できるMedia画面
- [Next] waveform、vectorscope、false color、focus peakingのnative／GPU renderer
- [Later] button remapping、workspace customization、external monitor layout

詳細: [`CAMERA_UI_LAYOUT.md`](CAMERA_UI_LAYOUT.md)

## Milestone 3 — Profile system

- [Done] 共通`ProfileEnvelope`とJSON Schema Draft 2020-12
- [Done] semantic version、RFC 3339、provenance、license、必須値の検証
- [Done] 未知same-major fieldを保持するround-trip
- [Done] `ProfileCatalog`の重複ID、自己参照、参照先、kind検証
- [Done] Film Profile v1 Schemaとtyped `FilmProfileData`
- [Done] Sensitometryの単位、非負density、strictな露光軸、補間／外挿contract
- [Done] Lens Profile v1 Schema、typed payload、焦点距離／絞り／image circle検証
- [Done] Digital Sensor Profile v1 Schema、typed payload、CFA／code range／ISO／分光感度検証
- [Done] Development／Print／Display／Output Transform Profile、Schema、synthetic examples
- [Done] recursive directory loaderとcontent hash付きrender snapshot
- [Done] explicit major-step schema migration registryと適用履歴
- [Later] 実在するlegacy schema用built-in migration
- [Later] 署名済みProfile package、registry、license enforcement

詳細: [`PROFILE_SCHEMA_AND_LOADER.md`](PROFILE_SCHEMA_AND_LOADER.md)

## Milestone 4 — Scientific reference pipeline

- [Done] 標準working spaceをscene-linear ACEScgへ確定
- [Done] ACES2065-1をinterchange／archive用途へ分離
- [Done] Film／Digital reference pipeline JSONとdomain validation test
- [Done] CPU ReferenceとGPU Realtimeの初期誤差基準を規定
- [Done] `scene_linear → virtual_exposure`のACEScg／D60、18%基準、校正露光、black floor、負値方針、数式を確定
- [Done] typed virtual exposure node、ACEScg制約、Film emulation Pipeline例、数式fixture
- [Done] CPU Reference executorでvirtual exposure nodeを実画素bufferへ適用
- [Done] linear／PCHIP RGB sensitometry evaluatorと全extrapolation方針
- [Done] straight alpha保持、負値error、補間fixture
- [Done] normal Developmentとmatrix output transformを含む最小Reference executor
- [Done] JSON golden exposure sweep fixture
- [Done] explicit synthetic Print responseとDisplay encoding
- [Later] measured Print responseと校正済みColorChecker fixture（測定dataset確定後）
- [Later] spectral sensitivity、dye density、chemical development、print response
- [Later] Grain、Halation、MTF／PSF、Lens optical model

詳細: [`Universal Film & Color Imaging Engine.md`](Universal%20Film%20%26%20Color%20Imaging%20Engine.md)、[`IMAGING_PIPELINE_ARCHITECTURE.md`](IMAGING_PIPELINE_ARCHITECTURE.md)

## Milestone 5 — GPU and zero-copy pipeline

- [Later] wgpu device／queue／texture ownership
- [Later] CPU ReferenceとのGPU conformance runner
- [Later] input transform、exposure、sensitometry、output shader
- [Later] LUT compilerと3D texture cache
- [Later] platform native texture import
- [Later] node fusion、texture lifetime、memory bandwidth最適化
- [Later] hardware encoderへ処理済みtextureを接続

開始条件: Milestone 4のCPU Reference縦切りとfixtureが`[Done]`であること。

## Milestone 6 — Nearby Peer Transfer

写真・動画を近くのユーザー同士で交換する。Bluetooth／BLEだけで大容量assetを送り切る設計にはせず、近接発見・招待・本人確認と実データ転送を分離する。

```text
Bluetooth LE / Bonjour / Nearby discovery
  → 双方の明示承認と短い確認コード
  → Wi-Fi Direct / peer-to-peer Wi-Fi / local networkへ昇格
  → end-to-end encrypted chunk transfer
  → content hash検証
  → incompleteからFinalized assetへ原子的に移行
```

- [Done] BLEを発見・接続確認、高速networkをasset転送に使う基本方針を確定
- [Done] Apple／Android／Windows／Linuxをplatform adapterで分離し、共通protocolをRustへ置く方針を確定
- [Later] `peer-transfer-core`のpeer identity、invitation、capability、transfer state machine
- [Later] versioned Asset Manifestとoriginal／processed／両方の選択
- [Later] chunk分割、ack、cancel、resume、content hash、atomic finalize
- [Later] ephemeral key、end-to-end encryption、双方の短い確認コード
- [Later] 一定時間だけ受信可能にするvisibilityと自動停止
- [Later] EXIF位置情報／device metadataの共有範囲を送信前に選択
- [Later] Apple adapter: Bonjour／local networkと対応OSの近距離API
- [Later] Android adapter: Nearby ConnectionsまたはBLE＋Wi-Fi経路
- [Later] Windows adapter: Bluetooth RFCOMM／DNS-SD＋Wi-Fi Direct
- [Later] Linux adapter: BlueZ GATT／mDNS＋TCPまたはQUIC
- [Later] 同一LAN上のmacOS同士で最初の暗号化Still転送MVP
- [Later] iOS↔Android、Windows、Linuxを同じprotocol conformanceへ接続

開始条件:

1. `CapturedAsset`のoriginal／derivative contractがコードへ実装済み
2. Media画面が受信中、失敗、完成assetを区別できる
3. path traversal、容量上限、MIME／codec、空き容量を受信前に検証できる
4. `.partial`／incomplete assetを完成品として公開しない

バックグラウンド転送はOSごとの実行制限に従う。発見・advertiseを常時有効にせず、ユーザーが開始した共有sessionだけを対象とする。本名、Bluetooth address、永続device IDを周囲へ公開しない。

## Milestone 7 — Mobile

- [Next] Tauri 2 iOS／Android projectを初期化
- [Next] 英語、日本語、简体中文のcamera／microphone権限文言
- [Next] iOS AVFoundation native preview／Still／Video縦切り
- [Next] narrow device、Safe Area、回転、background lifecycleの実機検証
- [Later] Android CameraX Preview／ImageCapture／VideoCapture
- [Later] RAW、LOG、manual controlが必要な端末向けCamera2経路
- [Later] CVPixelBuffer／AHardwareBufferのzero-copy GPU bridge

## Milestone 8 — Windows and Linux

- [Later] Windows Media Foundation device discovery／preview／Still／Video
- [Later] Windows D3D texture bridgeとhardware MFT
- [Later] Linux V4L2／libcamera／GStreamer採用判断
- [Later] Linux camera preview／Still／Video
- [Later] DMA-BUF対応時のzero-copy Vulkan bridge
- [Later] platform CI matrixと実機／device fixture

## 現在の優先キュー

今回完了:

- [Done] `scene_linear → virtual_exposure` adapterとFilm emulation Pipeline fixture
- [Done] CPU virtual exposure＋RGB sensitometry Reference executor
- [Done] Development／Print／Display／Output Transform typed Profile
- [Done] directory loaderとSHA-256付きrender snapshot
- [Done] normal Development、matrix Output Transform、golden exposure fixture
- [Done] synthetic Print responseとDisplay encoding
- [Done] explicit schema migration registry
- [Done] device別camera format設定のatomic保存とsession再開時の復元
- [Done] JPEG／QuickTimeを外部toolなしで検査する共通`CapturedAsset` probe
- [Done] 1280×720／24 FPSのStill／Video保存後実機validation
- [Done] Video出力時のPhotoOutput切替とsession preset再適用
- [Done] Preview／Photo／Movie orientation同期と全EXIF／MOV変換fixture
- [Done] Tauri／Vite開発serverを127.0.0.1:1420へ統一

次の順序:

1. [Done] UI姿勢をPreview／Photo／Movie connectionへ同期し、保存mirrorを分離する
2. [Next] iOS実機でportrait／upside-down／front-camera mirrorを検証する
3. [Next] CapturedAsset derivativeへrender snapshot／parent／engine version／seedを保存する
4. [Next] Finalized／Incomplete／Failedを扱うMedia indexとcleanup UIを実装する
5. [Next] iOS／Android Tauri projectを初期化する
6. [Later] measured Print dataset確定後にresponse／ColorChecker fixtureを追加する

Nearby Peer Transferは上記3–5で`CapturedAsset`とMedia管理が成立した後に着手するため、現時点では`[Later]`とする。

優先キューを変更するときは、依存関係、受け入れ条件、変更理由を本書か[`DECISIONS.md`](DECISIONS.md)へ残す。

## Verification baseline

2026-08-24時点:

```text
npm run check
  TypeScript / Vite production build: passed
  Rust workspace tests: 51 passed, 0 failed

macOS native runtime
  camera preview: passed
  JPEG Still capture: passed
  H.264 + AAC MOV recording: passed
  active format 1920×1080/30 → 1280×720/24: passed
  persisted format restore: 1280×720/24 passed
  selected-format JPEG: 1280×720 / EXIF sRGB passed
  selected-format MOV: H.264 1280×720 / 23.998 FPS / BT.709 + AAC mono passed
  orientation IPC at macOS 0°: preview / JPEG / MOV passed, not mirrored
  save-before-publish asset probe: passed
  continuous full-frame WebView IPC: none
```

未検証:

- [Next] iOS、Android、Windows、Linux build／runtime
- [Next] 外部camera hot plugとdevice切断復旧
- [Next] 英語／简体中文OSでのpermission prompt実表示
- [Next] 4K60 performance budget
- [Next] CPU／GPU画像conformance

## Release gates

### macOS technical preview

- [Done] native preview、Still、Video、format選択
- [Done] orientation connection同期、metadata probe、設定永続化
- [Next] iOS orientation実機検証、Media一覧
- [Next] lifecycle／device disconnect recovery
- [Next] 正式な署名、notarization、bundle identifier確定

### Cross-platform alpha

- [Later] macOS／iOS／Androidで同じCamera／Asset contractを通過
- [Later] 最小CPU Imaging PipelineをStillとVideo frameへ適用
- [Later] Profile snapshotから同一結果を再生成
- [Later] 2つ以上のOS間で暗号化Still転送、cancel／resume、hash検証に合格

### Imaging Engine 1.0

- [Later] CPU ReferenceとGPU rendererのconformance合格
- [Later] versioned Profile／Recipe／Pipeline／Asset migration
- [Later] Film、Lens、Sensor、Development、Print、Displayのprofile一式
- [Later] supported platformのperformance／quality gate合格

## 更新ルール

1. コード、Schema、test、検証artifactのいずれかが存在してから`[Done]`にする。
2. `[Next]`は依存関係上すぐ着手する項目に限定する。
3. 遠い構想は`[Later]`に置き、完了したように見せない。
4. 実機依存項目はcompile成功だけで`[Done]`にしない。
5. roadmap変更と同じturnで関連詳細文書も同期する。
