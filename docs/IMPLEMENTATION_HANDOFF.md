# Camera App Implementation Handoff

更新日: 2026-08-22  
対象: macOS / Windows / Linux / iOS / Android  
UIシェル: Tauri 2 + TypeScript  
共有コア: Rust

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
- [Next] CPU reference rendererの露出・RGB sensitometryとfixtureテストを追加
- [Done] Profile共通metadata、JSON Schema、Rust loader、extension保持、Catalog参照検証を追加
- [Done] Film Profile専用Schema、typed payload、sensitometry単位／curve検証を追加
- [Next] 残るProfile kindのtyped payloadと`scene_linear → virtual exposure` adapterを追加
- [Next] Tauri mobileのiOS/Androidプロジェクトを初期化し、カメラ／マイク権限文言を追加
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
