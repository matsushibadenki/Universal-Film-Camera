# Universal Imaging Camera Roadmap

更新日: 2026-09-01
対象: macOS / iOS / Android / Windows / Linux  
正本仕様: [`Universal Film & Color Imaging Engine.md`](Universal%20Film%20%26%20Color%20Imaging%20Engine.md)

## Status legend

- [Done] implemented in the current codebase
- [Next] high-priority unfinished work
- [Later] planned, but not the closest next step

この文書はプロジェクト全体の進捗indexである。個別機能の技術契約と検証記録は各詳細文書を正本とする。「設計済み」と「実装済み」を混同せず、コードまたは検証可能なartifactが存在する項目だけを`[Done]`にする。

## 現在の開発優先方針

iOSを唯一のactive platformとする。既存のmacOS／Android codeと共通coreは退行を防ぐためbuild／test対象に残すが、新機能開発は保留する。Windows／Linux、Android、macOS固有拡張、iOS以外とのpeer transfer、他platform向けGPU最適化はすべて`[Later]`である。共通coreを変更する場合も、直近の受け入れ先がiOSであることを必須とする。

## 現在地

現在は、macOS向けプロカメラのnative縦切り、Universal Imaging Pipeline、CPU finishing、CPU monitor scope、Media／Nearby recovery基盤まで完了している。GPU実時間経路と、保有していない実機／他OS／配布証明書を要する受け入れは`[Later]`である。

| 領域 | 現在の段階 | 最も近い完了条件 |
|---|---|---|
| iOS camera | [Done] native Preview／Still／Video compile、Simulator lifecycle回帰、署名済み実機install／launch | [Next] iPhone実機camera受け入れ |
| macOS camera | [Later] validated MVPを保守のみ | iOS優先期間終了後に再評価 |
| Professional UI | [Done] capture-first shell + CPU scope contract | native／GPU monitoring overlayを接続 |
| Imaging Pipeline model | [Done] schema v1 + render snapshot + CPU finishing | measured profile dataを追加 |
| Profile system | [Done] typed Profiles + loader + migration registry | profile署名／配布方針を確定 |
| Film renderer | [Done] CPU synthetic finishing縦切り | measured Print／ColorCheckerを追加 |
| GPU renderer | [Later] architecture only | wgpu texture pipelineとreference比較 |
| Nearby sharing | [Later] Apple暗号化Original転送＋resume／recovery／partial管理を保守 | iOS camera安定化後に実機2台受け入れ |
| Android | [Later] CameraX Still／Videoを保守のみ | iOS優先期間終了後に再評価 |
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
- [Done] HEIF／RAW、codec／container／bitrate／audio channelの型付き能力モデルとbackend別出力組合せ
- [Done] window close／background時の録画finalize、低頻度device切断監視、foreground再発見と共通復旧action
- [Next] `AVCaptureVideoDataOutput → CVPixelBuffer → Metal texture`のzero-copy preview renderer

詳細: [`APPLE_CAMERA_BACKEND.md`](APPLE_CAMERA_BACKEND.md)

Asset contract: [`CAPTURED_ASSET_CONTRACT.md`](CAPTURED_ASSET_CONTRACT.md)

## Milestone 2 — Professional camera UI

- [Done] 光学絞り・センサー・動画方向を統合した媒体非依存のプロ向けアプリアイコンと全platform asset
- [Done] 技術的・暗色・撮影画面優先のresponsive layout
- [Done] 320／375／414／768／1280pxのlayout検証
- [Done] right rail／bottom railで中央正円capture controlを維持
- [Done] phone landscapeのブランド表示を廃止し、解像度表示幅とcapture周囲の操作禁止帯を確保した2×2 menu rail
- [Done] 後日取込用のLUT／シャッター音project staging directoryと受入条件
- [Done] scope／monitor tools menuをnative preview外へ配置
- [Done] 3分割（3×3）／方眼／対角線guideと環境設定からの永続切替
- [Done] 英語、日本語、简体中文の主要UI
- [Done] 実機active formatとmanual control可否を表示
- [Done] 対応組合せから生成するformat／FPS panel
- [Done] Still／Video別output preset、実filesystem残容量、概算撮影可能量表示
- [Done] Still／Video別の撮影開始前容量検査、256 MiB安全予約、容量不足時の中央capture control無効化
- [Done] Finalized／Incomplete／Failedを確認できる三言語Media画面
- [Done] waveform、vectorscope、false color、focus peakingの決定論的BGRA8 CPU reference renderer
- [Done] iOS live monitor frameを用いたSobel focus peaking、Rec.709／sRGB／Display P3 monitor transform、内蔵look／外部3D `.cube` LUTのライブ適用
- [Done] iOSのdevice別format永続化／起動時復元と、画角を維持する透明focus-peaking overlay
- [Done] 360px／10Hz CPU monitor fallback、約92 IRE Zebra、狭幅monitor tool label
- [Done] LUT／カラープロファイル／ピーキング／ゼブラを統合したWebGL2 shader renderer、処理時最大約30Hz／非処理時10Hzの動的取得
- [Next] iOSのCVPixelBuffer→Metal texture zero-copy rendererでWebView IPCとRGB CPU変換を廃止
- [Done] foreground録画中の2秒間隔残容量監視と共通停止経路による安全な自動停止／Finalize
- [Done] Android CameraX native容量monitor、onPause停止、Finalize結果保持
- [Done] Apple AVFoundation sessionのWebView非依存容量monitorと単発stop要求
- [Next] iOS実機でOS suspension／AVCaptureSession interruption時の停止・復帰・asset回収を検証
- [Later] button remapping、workspace customization、external monitor layout

詳細: [`CAMERA_UI_LAYOUT.md`](CAMERA_UI_LAYOUT.md)

アイコン仕様: [`APP_ICON.md`](APP_ICON.md)

## Milestone 2.1 — Media persistence and library

- [Done] CapturedAsset schema v2のversioned JSON manifest
- [Done] manifestのatomic writeとmedia finalize失敗時rollback
- [Done] Finalized／Incomplete／Failedを区別するMedia index
- [Done] corrupt／unknown manifestを黙って無視しないvalidation
- [Done] Tauri `get_media_index` command
- [Done] 英語、日本語、简体中文のMedia一覧とstate filter
- [Done] Media表示中にnative preview layerを停止し、camera復帰時に再開
- [Done] asset詳細dialogとFailed／Incompleteの確認付きcleanup
- [Done] root直下のorphanを自動削除せずFailedへ記録するreconciliation
- [Done] Failed／Incompleteを非破壊probeする再検査とStill／Video再撮影導線
- [Done] Finalized写真をMedia Index ID／canonical captures検証付きpreview commandでMediaカードへ表示し、サムネイル／詳細表示を切替
- [Done] サムネイルの長押し／右クリック／キーボードcontext menu、確認付き明示削除、iOS PhotosへのAdd Only書き出し
- [Done] iOS app update時のdata container UUID変更を吸収するMedia resource再解決と、詳細リスト専用縦スクロール
- [Done] 環境設定Media tabから完成済み写真をiOS Photosへ全件保存し、全件成功後だけアプリ内コピーを一括削除する確認付き移行
- [Next] 原本JPEGのbase64 IPCを置換する永続縮小thumbnail生成とcache invalidation
- [Later] video poster／proxy生成、pagination、検索

詳細: [`MEDIA_INDEX_AND_MANIFEST.md`](MEDIA_INDEX_AND_MANIFEST.md)

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
- [Done] CapturedAsset schema v2のderivativeへrender snapshot／parent／engine version／seedを保持
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
- [Done] `peer-transfer-core`のephemeral peer identity、invitation、capability、transfer state machine
- [Done] version 1 Transfer Manifest、basename／容量／chunk／SHA-256 validation
- [Done] contiguous ack、cancel、content verification前のFinalized禁止
- [Done] `.incomplete`受信writer、durable contiguous ACK、resume ledger、既存byte再hash
- [Done] 容量予約、symlink拒否、実file SHA-256、完成先atomic rename
- [Done] CapturedAsset original／derivative／両方のAsset Transfer Manifest選択model
- [Done] Original受信のMedia Incomplete → probe → Finalized／Failed lifecycle adapter
- [Done] JPEGのEXIF／XMP／IPTC／comment／未知APP segmentを実byteから除去し、再hashする`StripDeviceAndLocation` sanitizer
- [Done] 不透明な`SanitizedJpeg`証明型と再probe／byte長／SHA-256照合をAsset Transfer Manifest source builderへ接続
- [Done] 既存親assetへのDerivative受信をparent resource ID／render snapshot／engine version／seed保持付きでfinalize
- [Done] Original＋Derivative一括受信のsource→local resource ID map、依存順序、循環／欠落検査を行うbundle coordinator
- [Done] ChaCha20-Poly1305 chunk frame、offset／transfer binding、durable prefix hashによるresume proof
- [Done] X25519 ephemeral key agreement、transcript由来6桁確認code、HKDF-SHA256 session key導出
- [Done] OS CSPRNG session secret生成とbounded binary frameをTCP stream adapterへ接続
- [Done] encrypted chunk／resume checkpoint／durable ACKのwire round-tripとoversize事前拒否
- [Done] stop-and-wait durable ACK、timeout設定、cancel、切断後resume、finalizeを束ねるtransport lifecycle
- [Done] Apple Bonjour互換mDNS advertise／browse、Apple P2P interface、TCP listener、Tauri discovery commands
- [Done] Apple local-network／Bonjour permission宣言と英語／日本語／简体中文の用途説明
- [Done] 撮影previewを停止して開くNearby専用画面、発見開始／停止、短期ID／peer一覧、1.5秒更新、撮影画面復帰を英語／日本語／简体中文で接続
- [Done] 発見peer＋Finalized Media選択、実file hash済みManifest、2分Invitation、transcript由来6桁code、local明示承認の三言語UI
- [Done] bounded TCP control frameでHandshake Offer／remote Approvalを交換し、transcript／双方能力を照合して同一暗号codecを生成する共通相互handshake
- [Done] Apple listener非blocking accept／outbound address connect、発見peer照合、incoming invitation三言語UI、双方承認後のsecure session保持
- [Done] secure sessionからOriginalを暗号chunk転送し、受信Incomplete→hash／probe／Media Finalized、送信側remote Finalized確認まで接続
- [Done] Apple transfer task開始時にreceiver durable checkpointを交換し、送信元prefix SHA-256一致位置から暗号化送信を再開
- [Done] Apple可視セッション内でtransport切断後も同じtransfer ID／Offerを保持し、再handshake→checkpoint交換へ戻る再接続制御
- [Done] durable ACK単位progress、転送容量／割合、local cancel要求とwire Cancelを三言語UIへ接続
- [Done] 接続中断時に検証済みpartial保持を示し、明示的な「再接続して再開」を提供する三言語UI
- [Done] disconnect／timeout／integrity／storage／Invitation失効／cancel／protocolを分類し、安全なretry可否を示す三言語recovery UI
- [Later] macOS／iOS実機2台でdiscovery→code比較→相互承認→切断再開を検証（検証機材がないため保留）
- [Done] 暗号化送受信progress／cancelの三言語UI
- [Done] disconnect-resumeの三言語UI
- [Done] 受信partialの確認付き明示discardと、期限切れ／非retry失敗から新しい承認を準備する三言語導線
- [Done] 7日partial保持期限、起動／Media読込時cleanup候補、確認付き利用者選択一括管理（自動削除なし）
- [Later] mobile background／network切替時のdisconnect-resume実機検証（実機2台調達後に再開）
- [Later] 一定時間だけ受信可能にするvisibilityと自動停止
- [Later] 選択的`StripLocation` EXIF再構築とMOV／MP4 metadata sanitizer
- [Later] EXIF位置情報／device metadataの共有範囲を送信前に選択するUI
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

詳細: [`PEER_TRANSFER_PROTOCOL.md`](PEER_TRANSFER_PROTOCOL.md)

## Milestone 7 — Mobile

- [Done] Tauri 2 iOS／Android projectを初期化しdebug artifactをbuild
- [Done] 英語、日本語、简体中文のiOS camera／microphone権限文言
- [Done] Android CAMERA／RECORD_AUDIO permission宣言
- [Done] iOS AVFoundation native preview host／Still／Video command縦切りをcompile
- [Done] iOS session health commandとforeground中のAVCaptureSession停止検出／安全な再発見経路
- [Done] iOS Simulator targetのRust／AVFoundation／UIKit compile・link回帰
- [Done] iPhone 16e Simulatorでbundle build／install／launchとSafe Area内の狭幅portrait表示を確認
- [Done] iPhone 16e Simulatorでbackground／foreground、portrait／landscape／upside-down、Media empty recoveryを操作回帰
- [Done] Development Team設定、署名済みarm64 IPA build、接続iPhoneへのinstall／process launch
- [Done] 320／390px幅で下部railを2段化し、スコープ展開時も中央シャッターとのtouch target重複を解消
- [Done] iOS LENS／固定IRIS実値表示、manual SHUTTER／WB AVFoundation制御、ダブルタップ拡大抑止
- [Done] iOS native previewをWKWebView背面へ合成し、映像座標・メニュー・グリッドoverlayのz-orderを修正
- [Done] WKWebView／内部scroll surfaceの透明化と、実camera frame由来32-bin histogram／実audio dB meter
- [Done] iOS実レンズ切替、固定IRIS表示、段階式EI／SHUTTER／WB、判読性を上げたFPS format panel
- [Done] 英語／日本語／简体中文のタブ式環境設定ページ、monitoring color space選択と端末内永続化
- [Done] 環境設定LUTタブ、17種のgeneric film/process archetype catalog、選択永続化、検証付き外部3D `.cube` import
- [Done] iOSの画面角度をAVFoundation角度へ変換（縦90°／横左0°／横右180°／逆さ縦270°）し、Preview／Still／Videoへ統一適用
- [Done] iPhone portraitは下部rail、狭高さlandscapeは右side railへ切り替え、回転後のSafe Area／DOM確定後にnative previewを再配置
- [Done] 撮影画面を100dvh内へ固定し、横向きright rail上のスワイプによるページ移動／rubber-bandを抑止
- [Done] HistogramのHISTラベルを除去し、Audio level meterと高さ・50%透過monitor背景を統一
- [Next] 選択LUTをnative preview／Still／Video rendererへ適用し、LUT ID・hash・domainをasset metadataへ保存
- [Later] 測色データと正当なライセンスを持つ実在film stock別profile／LUT package
- [Next] Rec.709／Display P3／sRGB設定をnative previewの実output transformと保存asset metadataへ接続
- [Next] iPhone実機でpreview透過合成、histogram応答、audio meter応答を受け入れる
- [Next] iPhone実機で縦／横左／横右／逆さ縦のPreviewと保存JPEG／MOVが被写体上方向を維持することを受け入れる
- [Next] narrow device、Safe Area、回転、background lifecycleをiPhone実機で受け入れる
- [Done] Android CameraX permission／discovery／capability／PreviewView
- [Done] Android CameraX ImageCapture／VideoCaptureと共通CapturedAsset finalize境界を実装しarm64 APKをbuild
- [Later] Android実機のStill／音声付きVideo／回転／background中断／manifest受け入れ（実機調達後に再開）
- [Done] Androidの選択解像度／FPSをCameraX use caseへ明示し、保存結果を要求値と照合するcodeをAPK build
- [Later] Android実機でformat別use-case組合せとCamera2 FPS rangeの受け入れ表を作成（実機調達後に再開）
- [Done] Android実機conformanceのinstall／snapshot harnessと必須試験matrixを作成
- [Later] Androidの追加開発・実機受け入れ全般（iOS優先方針のため保留）
- [Later] RAW、LOG、manual controlが必要な端末向けCamera2経路
- [Next] iOS CVPixelBuffer／Metal zero-copy GPU bridge
- [Later] Android AHardwareBuffer／VulkanまたはOpenGL ES zero-copy GPU bridge（iOS優先方針のため保留）

詳細: [`MOBILE_PLATFORM_BOOTSTRAP.md`](MOBILE_PLATFORM_BOOTSTRAP.md)
Android実機受け入れ: [`ANDROID_CAMERA_CONFORMANCE.md`](ANDROID_CAMERA_CONFORMANCE.md)

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
- [Done] CapturedAsset derivativeの再現情報、非上書き、非循環parent contract
- [Done] atomic asset manifestとFinalized／Incomplete／Failed Media index
- [Done] 三言語Media Catalogue UIと320／375／414／768／1100px検証
- [Done] 確認付きrecovery cleanup、orphan reconciliation、asset詳細dialog

次の順序:

1. [Done] Screen OrientationをAVFoundation角度へ変換し、Preview／Photo／Movie connectionへ同期して保存mirrorを分離する
2. [Next] iOS実機でpermission、Preview、Still、音声Video、portrait／landscape-left／landscape-right／upside-down／front-camera mirrorを検証する
3. [Done] CapturedAsset derivativeへrender snapshot／parent／engine version／seedを保存する
4. [Done] Finalized／Incomplete／Failedを扱うMedia indexとatomic manifestを実装する
5. [Done] Media一覧とFinalized／Incomplete／Failed filterを実装する
6. [Done] 安全なcleanup／orphan reconciliationとasset詳細を実装する
7. [Done] iOS／Android Tauri projectを初期化しdebug artifactをbuildする
8. [Done] iOS native preview hostとStill／Video commandを実装しSimulator buildを通す
9. [Done] Android CameraX permission／discovery／capability／native previewを実装する
10. [Done] Android Still／VideoをCameraXから共通CapturedAsset lifecycleへ接続しAPKをbuildする
11. [Next] 接続済みiPhone実機でpreview、Still、Video、orientation、lifecycleを検証する
12. [Later] Android実機でpreview、Still、Video、orientation、lifecycleを検証する（iOS優先期間終了後）
13. [Later] measured Print dataset確定後にresponse／ColorChecker fixtureを追加する

Nearby Peer Transferは上記3–5で`CapturedAsset`とMedia管理が成立した後に着手するため、現時点では`[Later]`とする。

優先キューを変更するときは、依存関係、受け入れ条件、変更理由を本書か[`DECISIONS.md`](DECISIONS.md)へ残す。

## Verification baseline

2026-09-01時点:

```text
npm run check
  TypeScript / Vite production build: passed
  Rust workspace tests: 101 passed, 0 failed

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

mobile scaffold
  iOS arm64 simulator debug bundle: passed
  iOS native Preview / Still / Video code compile and link: passed
  Android aarch64 debug APK with JDK 21: passed
  Android CameraX native preview code compile: passed
  Android CameraX Still / audio Video / CapturedAsset bridge compile: passed
  Android strict ResolutionSelector / Camera2 FPS request compile: passed
  iOS permission localization en / ja / zh-Hans: passed
  iOS Photos Add Only permission／native export link／signed device build: passed
  Android CAMERA / RECORD_AUDIO declaration: passed
```

未検証:

- [Later] iOS／Android camera runtime（実機待ち）、Windows／Linux build／runtime（対応host／backend実装待ち）
- [Done] 外部cameraの低頻度presence監視と切断時の安全停止／再発見経路
- [Later] 外部camera hot plug／切断復旧の実機受け入れ（対応camera調達後）
- [Later] 英語／简体中文OSでのpermission prompt実表示（該当localeの実機検証環境準備後）
- [Next] iPhone実機でPhotos Add Only prompt、書き出し成功、拒否時案内を受け入れる
- [Later] 4K60 performance budget（4K60対応実機とGPU frame bridge準備後）
- [Done] monitor scope CPU reference conformance fixture
- [Later] CPU／GPU画像同値conformance（GPU renderer実装後）

## Release gates

### macOS technical preview

- [Done] native preview、Still、Video、format選択
- [Done] orientation connection同期、metadata probe、設定永続化
- [Next] iOS orientation実機検証、Media runtime検証
- [Done] lifecycle／device disconnect recoveryの共通方針、window/background finalize、再発見経路
- [Done] Development Teamによるdebug実機署名、IPA生成、install／launch
- [Later] 配布用署名、notarization、正式bundle identifier確定（配布方針決定後）

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
[Done] iOSピーキング／ゼブラ解析出力をネイティブプレビューと同じ回転・ミラー座標系へ統一
[Done] 環境設定でピーキングカラーを6色から選択・保存し、ライブ解析表示へ即時反映
[Done] 環境設定で通常・爽やか・一眼レフ風・無音のシャッター操作音を選択・保存
[Done] 「無音」撮影時にiOS 18+の正式なシャッター音抑制APIを対応端末・地域で要求
[Done] 外部シャッター音源（MP3／M4A／WAV／CAF）の読み込み・試聴・永続保存・選択
[Done] アプリ全体を映像優先のWorkbenchへ再設計し、設定タブ・画面遷移・表示切替・撮影補助をアイコン中心へ簡略化
[Done] 横位置の環境設定を左カテゴリ＋右編集ペイン、メディアを左フィルター＋右カタログの分割レイアウトへ再構成
[Done] フォーマットの解像度表記を幅×高さだけに統一し、4K／UHD／1080p／720pの重複表記を廃止
[Done] 縦向き下部railでスコープ・メディア・近距離共有をシャッター左側へ集約し、環境設定だけを右端に固定
[Next] iPhone実機の横位置でSettings／Media分割表示、各ペインscroll、行き先menu自動closeを目視受け入れする
