# Camera App Implementation Handoff

更新日: 2026-08-28
対象: macOS / Windows / Linux / iOS / Android  
UIシェル: Tauri 2 + TypeScript  
共有コア: Rust

プロジェクト全体の現在地と優先順位は [`ROADMAP.md`](ROADMAP.md) を正本とする。

2026-08-26、`CapturedAsset`をschema version 2へ更新した。originalとderivativeへ安定resource IDを与え、derivativeはparent、完全なrender snapshot、engine version、seedを保持する。追加時にFinalized状態、既存parent、一意ID、一意path、snapshot hash／順序を検証し、JSON往復testで再現情報を固定した。次工程はこの構造をatomic manifestへ保存し、Finalized／Incomplete／Failedを扱うMedia indexから復元することである。

2026-08-27、上記manifestとMedia indexを実装した。Still／Videoはmedia renameだけでは成功せず、CapturedAsset manifestのatomic保存後に返る。manifest失敗時はmediaを`.incomplete`へrollbackする。`get_media_index`はFinalized／Failed manifestとmanifest未作成のIncomplete fileを統合し、壊れたmanifestをerrorとして可視化する。次工程は三言語Media UI、安全なcleanup、起動時orphan reconciliationである。正本は[`MEDIA_INDEX_AND_MANIFEST.md`](MEDIA_INDEX_AND_MANIFEST.md)。

同日、三言語Media Catalogue UIを追加した。Media遷移前にnative previewを停止するため、AVCaptureVideoPreviewLayerが一覧文字を覆わない。All／完了／未完了／失敗filter、technical metadata、診断理由を表示し、カメラへ戻るとpreviewを再開する。320／375／414／768／1100pxで横overflowなし、44px以上の操作領域、filter labelの非折返しを確認した。

続いてasset詳細dialog、Failed／Incompleteだけを対象とする確認付きcleanup、root直下のorphan reconciliationを実装した。reconciliationは素材を消さずFailed診断として登録する。cleanupはFinalizedを拒否し、canonical pathがcaptures配下の通常ファイルであることをcommand側で検証する。次工程はFailed／Incompleteの再検査・capture再試行導線、または優先キューどおりiOS／Android Tauri project初期化である。

2026-08-28、iOS／AndroidのTauri 2 mobile projectを初期化してrepository管理対象にした。iOSはAVFoundation／CoreMedia link、英語・日本語・简体中文のpermission resourceを含むarm64 simulator bundle、AndroidはCAMERA／RECORD_AUDIO宣言を含むarm64 debug APKまでbuild済みである。Android buildはGradle互換性のためJDK 21を使用する。詳細と再現commandは[`MOBILE_PLATFORM_BOOTSTRAP.md`](MOBILE_PLATFORM_BOOTSTRAP.md)を正本とする。次工程はiOS native preview hostとAndroid CameraX adapterであり、mobile scaffoldのbuild成功をcamera runtime完成と扱ってはいけない。

同日、iOSの`UIView` preview hostを追加した。TauriのWKWebView main-thread closure内でAVCaptureVideoPreviewLayerをattachし、DOM viewport更新に合わせてresize、Media遷移時にdetachする。Preview／Still／Video commandをiOSでも有効にし、保存はmacOSと同じatomic CapturedAsset契約を通る。arm64 Simulator bundleはcompile／link済みだが、Simulatorをcamera runtime検証には使わない。次工程は開発Teamを利用者が明示したiPhone実機検証とAndroid CameraX adapterである。

続いてAndroidのCameraX Tauri mobile pluginを実装した。CAMERA permission、front／back discovery、Camera2から取得するstream size／AE FPS capability、native PreviewViewのattach／resize／stopを既存Tauri commandへ接続した。Activity pause／destroyでunbindし、連続frameはWebView IPCへ渡さない。JDK 21によるarm64 debug APK buildは成功している。次工程はAndroid実機preview検証とImageCapture／VideoCaptureのatomic CapturedAsset接続である。

同日、CameraX `ImageCapture`と`VideoCapture<Recorder>`を同じnative lifecycleへ追加した。JPEGと音声付きMP4は`.incomplete`へ書き、Still callbackまたはVideo `Finalize`後にだけRustへ返す。Rustは保存物を直接probeし、CapturedAsset validation、完成pathへのrename、atomic manifest保存まで成功してからUIへ返す。失敗時はFailed recordへ診断を残す。arm64 debug APKはbuild済みだが、実機撮影は未検証である。format未選択時のAndroid capture metadataはprobeした実出力から固定する。

続いてAndroidの`apply_camera_format`を実装した。選択解像度はfallback禁止のCameraX `ResolutionSelector`、FPSはCamera2 interopの`CONTROL_AE_TARGET_FPS_RANGE`としてPreview／ImageCapture／VideoCaptureへ同時適用する。再bind失敗時は以前の構成へrollbackし、要求値を黙って置換しない。撮影callbackにも要求formatを含め、Rust probeが保存寸法・FPSとの一致を検証する。codeとarm64 APK buildは完了したが、端末ごとの同時use-case組合せは実機受け入れ表が必要である。

ADB接続端末を確認したが、2026-08-28時点ではauthorized deviceが0台だった。runtimeを推測で完了扱いにせず、`scripts/android_camera_conformance.sh`と[`ANDROID_CAMERA_CONFORMANCE.md`](ANDROID_CAMERA_CONFORMANCE.md)を追加した。scriptは端末を1台に限定してAPK導入／起動を行い、getprop、Camera service、package permission、app-private files一覧、logcatを端末serial別に採取する。試験matrixはpermission、Preview、Still、Video、rotation、background、recovery、format rejectionを必須とする。

2026-08-30、Media recoveryへ非破壊再検査と再撮影導線を追加した。`reinspect_media_entry`はFailed／Incompleteだけを再probeし、resourceを変更せず診断manifestを更新する。probe成功でも元のcapture intentがないためFinalizedへ昇格しない。再撮影は既存resourceを残して対応するStill／Video modeへ復帰する。三言語UI、Rustの昇格防止test、desktop／320px browser QAを通過した。開発fixtureは`import.meta.env.DEV`かつ`?recovery-fixture=1`に限定する。

同日、Still／Video別output statusと保存先残容量を実装した。Rust commandは現行writerが保証するJPEGとH.264／AAC presetだけを返し、Unix系platformではcapture directoryの`statvfs`からavailable／total bytesを取得する。UIはJPEG 8 MiB／枚、Video 120 MiB／分のnominal estimateを三言語で表示し、撮影完了後に更新する。320px QAで出力button 44px、capture button 56px正円・水平中央、横overflowなしを確認した。将来の複数presetはnative writerが設定を実適用・probe検証できる場合だけ追加する。

同日、保存前容量preflightを追加した。Stillは8 MiB、Videoは開始時120 MiBの概算出力に加えて256 MiBを安全予約し、満たさない場合はUIの中央capture controlを無効化する。Rust側もApple／Androidの各撮影command直前に同条件を再検査し、競合やUI迂回時は明示的に拒否する。録画中に別processが容量を消費する場合への連続監視と安全な自動停止は未実装である。

続いてforeground録画中の容量監視を追加した。UIは2秒ごとにRustのfilesystem statusを取得し、Video閾値を下回ると手動停止と共通の単発停止処理を実行する。停止後はnative finalize、in-process probe、atomic rename、manifest保存を経たassetだけを成功表示し、自動停止理由を三言語で通知する。重複poll／重複停止はguardする。WebViewがpauseされるbackground録画を守るにはApple／Android native lifecycle側のmonitorが別途必要である。

続いてAndroid CameraX pluginへWebView非依存の2秒間隔容量monitorを追加した。Rustから渡す376 MiB閾値を保存先`usableSpace`が下回るとnative `Recording.stop()`を一度だけ呼び、Finalize結果をplugin内に保持する。`onPause`／`onDestroy`でも`close()`による即時破棄ではなくstop→Finalizeを優先し、復帰時のvisibility handlerがRust `stop_video_recording`から保持結果を受け取ってprobe／manifest確定し、native previewを再取得する。Rust／Web buildは成功。Android APK buildはRust cross compileまで成功したが、ローカルAndroid Studio JBR 25.0.2とGradle buildSrcの非互換でKotlin compile前に停止したため、JDK 21環境でのAPK再検証と実機容量低下試験が必要である。

続いてApple AVFoundation sessionへWebView非依存の容量monitorを追加した。録画ごとのPendingMovie IDとcamera stateを2秒間隔で照合し、保存先が376 MiB閾値を下回ると`request_recording_stop`を発行する。MovieRecordingはatomic stop flagを所有し、容量monitorと手動停止が競合しても`AVCaptureMovieFileOutput.stopRecording()`は一度だけ呼ばれる。delegate receiver、保存先、CapturedAsset finalize経路は従来のまま保持する。macOS compileとworkspace testは成功した。iOSでOSがprocessをsuspendする条件、AVCaptureSession interruption、復帰後asset回収は署名済み実機で検証するまで完了扱いにしない。

続いて`peer-transfer-core`をworkspaceへ追加した。platform discovery／transportから独立して、sessionごとのephemeral peer identity、6桁確認付き期限付き招待、protocol／transport／chunk能力交渉、version 1 Transfer Manifest、ACK、cancel、verify、Finalizedの状態遷移を所有する。BLEだけのpeer間ではasset転送を開始せず、高速transportの共通項を必須にした。basename、100 GiB上限、16 KiB〜4 MiB chunk、64桁SHA-256を受信前に検証し、byte数とhash一致前はFinalizedへ進めない。5件の境界testを含めworkspace全67 testが成功した。次工程はCapturedAsset selectionと`.incomplete`受信writerへの接続である。

## 現在地

- [Done] 元仕様 `Universal Film & Color Imaging Engine.md` の責務分離、色処理、ゼロコピー、カメラ抽象、エンコード要件を初期設計へ反映
- [Done] Cargo workspaceとTauri 2アプリシェルを作成
- [Done] `media-core` に共通フレーム記述、色空間、転送関数、CPU／ネイティブハンドルの境界を定義
- [Done] `camera-core` にスチル／動画モード、能力モデル、状態機械、バックエンド／セッションtraitを定義
- [Done] `film-core` にACEScg前提の画像、FilmRecipe、品質レベル、エンジンtraitを定義
- [Done] `imaging-core` にCamera Body/Exposure、Lens、Film/Digital Sensor、Chemical/RAW Development、Print/Output Transform、Displayの共通Pipelineを定義
- [Done] SignalDomainによる接続検証とObserved/Simulated/Transformのprovenanceを実装
- [Done] Tauri IPCはカメラ状態とモード選択だけに限定し、UIは英語・日本語・简体中文に対応
- [Done] Tauri用ベクター原稿からdesktop/iOS/Android向けアイコン52点を生成
- [Done] macOS上でRust test、TypeScript build、Tauri 2 debug application buildを検証
- [Done] プロ向けのdark camera UIへ更新し、Photo/Videoを同格の操作階層へ配置
- [Done] 320 / 375 / 414 / 768 / 1280pxで撮影画面のresponsive layoutと操作を実画面検証
- [Done] Apple AVFoundation backendでcamera権限状態／要求、内蔵・外部device列挙、Tauri IPC接続を実装
- [Done] macOS permission文言を英語・日本語・简体中文でbundle resourceへ追加
- [Done] macOSでAVCaptureSessionとAVCaptureVideoPreviewLayerをnative NSViewへ接続し、実機previewを表示
- [Done] `preview-surface`のDOM座標をnative viewへ同期し、title bar補正とresponsive resizeを実機確認
- [Done] シャッター／録画ボタンを撮影rail中央の正円として固定し、左右の操作群から独立
- [Done] Appleバックエンドへ`AVCapturePhotoOutput`を追加し、Still modeからJPEGを原子的に実保存
- [Done] Appleバックエンドへmicrophone inputと`AVCaptureMovieFileOutput`を追加し、音声付きMOVを実保存
- [Done] 中央録画ボタンをnative start／stopへ接続し、スチル／動画同格の最初の縦切りを完成
- [Done] macOS camera／audio-input EntitlementとLaunchServices経由のTCC検証手順を確立
- [Done] Appleのresolution／FPS／manual exposure/focus能力を列挙し、active formatを上部parameterへ同期
- [Done] 対応するresolution／FPS組合せから選択panelを生成し、sessionへ明示適用
- [Done] 元仕様をVersion 0.2へ更新し、Imaging Pipeline、ACEScg、Profile／Asset／性能／適合試験の規範契約を統合
- [Next] 選択formatで生成したphoto／movieの寸法・FPS・metadataを再検証し、設定を永続化
- [Next] RAW／HDR／LOGをformat単位の能力モデルへ拡張
- [Next] still／videoのorientation、rotation、保存metadataを統一
- [Done] CPU Reference rendererのvirtual exposure・PCHIP RGB sensitometry・alpha保持を追加
- [Done] CPU Referenceへnormal Development／matrix output transformとgolden fixtureを追加
- [Done] explicit synthetic Print responseとDisplay encodingを接続
- [Done] explicit major-step schema migration registryと適用履歴を追加
- [Done] Profile共通metadata、JSON Schema、Rust loader、extension保持、Catalog参照検証を追加
- [Done] Film Profile専用Schema、typed payload、sensitometry単位／curve検証を追加
- [Done] Lens／Digital Sensor専用Schema、typed payload、物理値／CFA／分光感度検証を追加
- [Done] ACEScg `scene_linear → virtual exposure` adapter、Film emulation Pipeline例、数式fixtureを追加
- [Done] Development／Print／Display／Output Transform typed Profile、Schema、synthetic例を追加
- [Done] recursive directory loader、Profile closure解決、SHA-256付きrender snapshotを追加
- [Done] Tauri mobileのiOS／Android project、権限宣言、debug build
- [Done] iOS native preview hostとStill／Video commandのSimulator build
- [Done] Android CameraX permission／discovery／capability／native preview
- [Done] Android CameraX Still／音声付きVideoと共通CapturedAsset保存境界のcompile／APK build
- [Next] mobile実機preview／Still／Video／orientation／background lifecycle検証
- [Done] Android CameraXの明示的format negotiationと保存結果照合codeのAPK build
- [Next] Android実機のformat別同時use-case受け入れ表
- [Done] Android実機conformance harness／判定matrix
- [Later] Android CameraX、Windows Media Foundation、Linux V4L2/GStreamerバックエンド
- [Later] wgpu処理、native texture相互運用、ハードウェアエンコード、OCIO/ACESの完全実装
- [Later] Grain、Halation、MTF、Print Film、Spectral reference model

## 確定した責務境界

```text
Tauri Web UI
  └─ 設定・状態・メタデータ・サムネイルだけをIPC
             ↓
camera-core（状態、能力、共通操作）
  ├─ camera-apple / camera-android / camera-windows / camera-linux
  ├─ media-core（VideoFrame、色・転送メタデータ、native handle）
  └─ imaging-core（全撮像工程、SignalDomain、profile参照、接続検証）
      ├─ lens renderer（将来）
      ├─ sensor / RAW developer（将来）
      ├─ film-core（Film、Chemical Development、Print）
      └─ display / output transform（将来）
             ↓
native preview surface + native/hardware encoder
```

WebViewへフル解像度フレームをbase64、Blob、Tauri eventで連続送信しない。IPCは制御面に限定する。macOSプレビューはWKWebView上のnative NSViewに配置し、重い処理はRust/GPU側で完結させる。公開APIだけを使うためpreview内部のWeb overlayは現時点で非表示にし、将来native layer／Metal compositorへ移す。Pipelineの詳細は [`IMAGING_PIPELINE_ARCHITECTURE.md`](IMAGING_PIPELINE_ARCHITECTURE.md) を参照する。

## スチルと動画の共通方針

1つのカメラセッションからpreviewを供給し、撮影操作のみを分ける。

- Still: 高解像度photo outputを使い、可能ならRAW/HEIF/JPEGとメタデータを保持する。Film処理済み画像とオリジナルを別資産として保存できる設計にする。
- Video: video + audioを単調増加timestampで収録し、停止時にmuxを確実にflushする。録画中はカメラモード、解像度、色空間の破壊的変更を拒否する。
- Preview: 低遅延を優先する。撮影出力と同一のImaging Pipeline/FilmRecipeを使うが、各rendererの`Preview`品質を許可する。
- Capture: 保存前に空き容量、権限、thermal stateを確認し、途中失敗でも再生可能なfragment/container戦略を検討する。

## 最初の縦切り実装

Appleを最初のreference backendとする。AVFoundationの1セッションにcamera input、microphone input、photo output、video data/movie output、preview layerを組み合わせる。Tauri mobile pluginのSwift側にnative viewとcapture sessionを所有させ、Rust/Tauri側へは状態、能力、撮影完了、エラーだけを通知する。Imaging Pipelineへ直接フレームを渡す段階では`AVCaptureVideoDataOutput`とMetal texture cacheを使い、CPU往復を避ける。

2026-08-20時点で、`camera-apple`へ権限、device discovery、macOS native preview、JPEG still capture、H.264/AAC MOV recording、resolution／FPS選択を実装した。StillとVideoはいずれも中央の正円ボタンから実ファイルを生成する。録画停止ではAVFoundationの最終delegateを待ってから完成assetを公開し、録画中のmode／format切替を拒否する。詳細、IPC contract、threading不変条件は [`APPLE_CAMERA_BACKEND.md`](APPLE_CAMERA_BACKEND.md) を参照する。

受け入れ条件:

- iOSとmacOSでcamera権限拒否から復帰できる
- 前後／外部カメラの列挙と切替ができる
- 写真1枚を撮影し、orientationと色メタデータを保って保存できる
- 音声付き動画を開始・停止し、生成物を再生できる
- 録画中のモード切替を拒否し、UIとnative stateが一致する
- UI停止／バックグラウンド移行時にsessionとencoderを安全に終了する

## プラットフォーム実装マップ

| Target | Capture / preview | Still | Video / audio | GPU bridge（目標） |
|---|---|---|---|---|
| macOS / iOS | AVFoundation | AVCapturePhotoOutput | AVCaptureMovieFileOutput、後にAssetWriter | CVPixelBuffer → Metal texture |
| Android | CameraXを第一候補。高度な手動制御はCamera2へ降りる | ImageCapture | VideoCapture / MediaCodec | Surface / AHardwareBuffer → Vulkan |
| Windows | Media Foundation Capture Engine | photo sink | record sink / hardware MFT | D3D11/12 texture → wgpu |
| Linux | V4L2 + libcamera検討 | native buffer/file | GStreamer/FFmpeg mux | DMA-BUF → Vulkan（対応時） |

Androidについて、元仕様はCamera2 NDKを中心としているが、最初の製品縦切りはCameraXのPreview、ImageCapture、VideoCaptureがスチル＋動画の組合せを短期間で検証しやすい。RAW、LOG、厳密なmanual control、native zero-copy要件がCameraXで満たせない端末だけCamera2/NDK経路を追加する。この判断は実機spike後にADRで確定する。

## 権限とセキュリティ

- Tauri capabilityは現在`core:default`のみ。カメラplugin追加時は`camera:allow-*`のように操作別permissionを定義し、main windowに必要最小限だけ付与する。
- remote URLへTauri APIを公開しない。CSPの`connect-src`もローカルIPC以外へ広げない。
- iOS/macOS: cameraとmicrophoneのusage descriptionが必要。Photosへ直接保存する場合だけPhotos権限を追加する。
- Android: CAMERA、動画で音声を使う場合はRECORD_AUDIO。MediaStore経由保存は対象APIごとの権限差を実機確認する。
- Windows/Linux: OS privacy設定、device busy、hot unplugを通常エラーとして扱う。

## 開発手順

```bash
npm install
npm run check
npm run tauri dev
```

モバイル生成物はTauri CLIとSDKが揃った環境で初期化する。

```bash
npm run tauri android init
npm run tauri ios init
npm run tauri android dev
npm run tauri ios dev
```

初期化前にbundle identifierを確定すること。現在の`app.universalfilm.camera`は仮値で、署名・entitlement・配布設定を始めた後の変更コストが高い。

## 検証記録

2026-08-16、macOS上でApple backend追加後に以下を確認した。

```text
cargo check -p camera-apple
  passed

cargo test --workspace
  8 passed, 0 failed

npm run build
  TypeScript type check: passed
  Vite production build: passed

npm run tauri build -- --debug --bundles app
  Tauri 2.11.5 macOS application bundle: passed
  output: target/debug/bundle/macos/Universal Film Camera.app

bundle inspection
  Info.plist: NSCameraUsageDescription / NSMicrophoneUsageDescription present
  Resources: en.lproj / ja.lproj / zh-Hans.lproj present and valid

native runtime
  built-in camera preview: passed
  1100 × 760 right-rail resize/alignment: passed
  880 × 650 bottom-rail resize/alignment: passed
  continuous frame transfer through Tauri IPC: none
  JPEG still capture: passed (1920 × 1080, Exif, sRGB)
  atomic partial-to-final save: passed
  microphone permission: passed via LaunchServices launch
  MOV recording start/stop: passed
  video stream: H.264, 1920 × 1080
  audio stream: AAC, 48 kHz, mono
  duration / size: 5.671 s / 24,189,059 bytes
  incomplete files after finalize: 0
  active preview format UI: 1920 × 1080 / 30 FPS / SDR
  format selector: supported combinations only
  active format apply: 1280 × 720 / 24 FPS passed
  unsupported manual controls: AUTO / disabled
```

debug bundleの実機検証では、生成後の`.app`を`Entitlements.plist`付きでadhoc再署名し、LaunchServices経由で起動する。bundle内実行ファイルをCodexやterminalの子processとして直接起動すると、TCCが親processをresponsible applicationとして扱い、アプリ固有のmicrophone promptが成立しない場合がある。配布手順へadhoc運用を持ち込まず、正式な署名とnotarizationを用意すること。

Windows、Linux、iOS、Androidではまだbuildしていない。CI matrixを追加するまで、各OS対応を完了扱いにしない。

2026-08-24、macOS capture asset milestoneで次を追加検証した。

```text
camera format settings
  device別JSON atomic save: passed
  relaunch restore 1280 × 720 / 24 FPS: passed

CapturedAsset probe
  JPEG: dimensions / bit depth / EXIF orientation / sRGB
  QuickTime: codecs / dimensions / rational FPS / duration / rotation / audio / colr
  required mismatch remains under captures/.incomplete

selected-format runtime
  JPEG: 1280 × 720 / 8-bit / EXIF sRGB
  MOV: H.264 1280 × 720 / 100000/4167 FPS / BT.709
  audio: AAC / 16 kHz / mono
  Video → Still output restoration: 1280 × 720 passed
```

AVFoundationではPhotoOutputとMovieFileOutputの同時接続がMovie出力formatを再交渉したため、Video開始前にPhotoOutputを外し、対応`AVCaptureSessionPreset`をcommit後にdevice active formatを最終適用する。Video後のStill captureではPhotoOutputを戻してformatを再適用する。この順序を崩すとUIは720pでも保存MOVが1080pまたはsquareになる。正本は[`CAPTURED_ASSET_CONTRACT.md`](CAPTURED_ASSET_CONTRACT.md)と[`APPLE_CAMERA_BACKEND.md`](APPLE_CAMERA_BACKEND.md)。

2026-08-24、rotation／mirror同期を次の境界で実装した。

```text
Screen Orientation + camera position
  → CaptureOrientation(0/90/180/270)
  → AVCaptureVideoPreviewLayer connection
  → AVCapturePhotoOutput connection → EXIF orientation
  → AVCaptureMovieFileOutput connection → QuickTime track matrix
```

front cameraはpreviewだけmirror、保存Still／Videoは既定で非mirrorとする。PhotoOutput切離し／再接続やactive format再適用後にも保持したorientationを再設定する。録画中のorientation変更は拒否し、停止後にUIが再同期する。EXIF orientation 1–8とMOVの4回転×mirror有無はfixture test済み。macOSの0度・非mirror保存は既存実機assetで確認済みだが、portrait／upside-down／front cameraはiOS実機受け入れが必要であり完了扱いにしない。判断根拠は[`DECISIONS.md`](DECISIONS.md) ADR-029を参照する。

同日の実機再検証ではorientation IPCを含むpreview start、1280×720 JPEG、H.264＋AAC MOVが成功し、両resourceのprobeでrotation 0度／非mirrorを確認した。`npm run tauri dev`はTauri `devUrl`が1420、Vite既定が5173だったため待機し続ける既存不具合があり、rootの`vite.config.ts`で127.0.0.1:1420へ固定した。開発起動設定を変更するときは`tauri.conf.json`と必ず同時に更新する。

## 未確定事項（実装前に決める）

1. 正式な製品名、bundle/application identifier、著作権表記
2. 保存先（アプリsandbox、OS写真ライブラリ、ユーザー指定folder）とオリジナル保持方針
3. MVP codec/container（推奨: H.264/AAC + MP4、AppleではHEVC/MOVも追加）
4. MVPの最低OS、端末、解像度、fps、HDR/LOG/RAW対応範囲
5. Lens、Sensor、Film、Development、Print、Display profileデータのライセンス、provenance、versioning
6. Linux backendをV4L2直結にするか、libcamera/GStreamerを採用するか

## 既知のリスク

- TauriのWebViewとnative camera viewのz-order、rotation、resize、safe areaは各OSで挙動が異なる。macOSのwindow chrome差分はAppKitで補正済みだが、iOS／他OSでは別途実機spikeが必要。
- `AppleCaptureSession`のAVFoundation mutationは`operation_lock`、NSView／layer mutationはmain threadで守る。この前提を外して`Send + Sync`対象のmethodを増やすとdata raceになり得る。
- カメラが出すNV12/P010とImaging Pipelineのscene-linear ACEScg間で、primaries、transfer、range、matrix情報を失うと色が再現できない。
- 同時photo/video output、4K60、HDR、stabilizationなどの組合せ可否は端末依存。UIは必ず`CameraCapabilities`から生成する。
- 4K60のCPU readbackは性能目標を満たさない。CPU rendererは正解画像とテスト用で、リアルタイム経路とは分離する。
- 音声と映像のclock、pause/resume、orientation変化を最初から記録モデルに含めないと後付け修正が大きい。

## 参考資料

- [Tauri 2 mobile plugin development](https://v2.tauri.app/develop/plugins/develop-mobile/)
- [Tauri 2 capabilities](https://v2.tauri.app/security/capabilities/)
- [Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/)
- [Apple: Setting up a capture session](https://developer.apple.com/documentation/avfoundation/setting-up-a-capture-session)
- [Apple: AVCapturePhotoOutput](https://developer.apple.com/documentation/avfoundation/avcapturephotooutput)
- [Apple: AVCaptureMovieFileOutput](https://developer.apple.com/documentation/avfoundation/avcapturemoviefileoutput)
- [Android: Capture an image with CameraX](https://developer.android.com/media/camera/camerax/take-photo)
- [Android: CameraX video capture](https://developer.android.com/media/camera/camerax/video-capture)
