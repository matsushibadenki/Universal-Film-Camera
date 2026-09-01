# Camera App Implementation Handoff

更新日: 2026-09-01
対象: macOS / Windows / Linux / iOS / Android  
UIシェル: Tauri 2 + TypeScript  
共有コア: Rust

プロジェクト全体の現在地と優先順位は [`ROADMAP.md`](ROADMAP.md) を正本とする。

2026-09-01、iOS姿勢同期を修正した。Webのportrait-up 0度をAVFoundationへそのまま渡さず、縦90度、横左0度、横右180度、逆さ縦270度へ変換する。Preview／Photo／Movieは同じconnection角度を使用し、front cameraはpreviewだけ鏡像、保存物は非鏡像である。PhotoOutputの再接続とformat再適用後にも保持角度を再設定するため、JPEGとMOVの保存経路もPreviewと同じ被写体上方向になる。回転時はSafe AreaとDOMの確定を待ってnative preview frameを二段階再同期する。portraitでは下部rail、狭高さlandscapeでは右side railを使う。録画中の回転は単一MOV内の途中transform変更を避けて拒否し、停止直後に現在姿勢を反映する。実装／build完了と、実機で4姿勢のPreviewおよび保存JPEG／MOVを目視確認する受け入れは区別し、後者を`[Next]`に残す。

同日、横向きright rail上のスワイプで撮影画面全体が移動する問題を修正した。`html/body`をviewportへ固定してoverflow／overscrollを止め、camera shellを`100dvh`内へ拘束し、スクロール不要なtool railには`touch-action: none`を指定した。Media／Nearby／環境設定は各section内部の既存`overflow-y: auto`を維持するため、長い一覧のスクロールは失わない。

同日、撮影monitorのHistogramから冗長な`HIST`ラベルを除去し、HistogramとAudio level meterを4.5remの同一高さへ固定した。両panelはmonitor背景色50%＋2px backdrop blurへ統一し、撮影映像を透過させつつRGB波形、channel、dB表示の判読性を維持する。

同日、Media CatalogueへFinalized写真の実画像previewとサムネイル／詳細表示切替を追加した。初版のTauri asset protocolはiOS実機で画像を解決できずplaceholderへfallbackしたため採用を取り消し、`get_media_photo_preview(id)`へ切り替えた。commandはUIからpathを受け取らずMedia Index IDだけを受理し、Finalized Photo、対応container、canonical captures配下の通常file、1 byte〜64 MiBを検証してからdata URL用base64を返す。サムネイル表示は写真中心の正方形grid、詳細表示は同じpreviewにファイル名、日時、寸法、duration、ID、診断、詳細dialog導線を併記する。動画、未完了、失敗、画像decode失敗は種類／状態placeholderへ安全にfallbackする。現在は原本JPEGを一度だけIPCへ通す暫定実装であり、永続縮小thumbnail生成と動画posterを次工程とする。

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

続いて`peer-transfer-core`をworkspaceへ追加した。platform discovery／transportから独立して、sessionごとのephemeral peer identity、6桁確認付き期限付き招待、protocol／transport／chunk能力交渉、version 1 Transfer Manifest、ACK、cancel、verify、Finalizedの状態遷移を所有する。BLEだけのpeer間ではasset転送を開始せず、高速transportの共通項を必須にした。basename、100 GiB上限、16 KiB〜4 MiB chunk、64桁SHA-256を受信前に検証し、byte数とhash一致前はFinalizedへ進めない。

同crateへ`ReceiveWriter`を追加した。受信前に残りbyte＋256 MiB予約を検査し、`.incomplete/peer-transfer`へ連続chunkだけを書き、`sync_data`後にledgerをatomic更新してdurable ACKを返す。再開時はledger、manifest、part長を一致確認し、保存済みbyteをSHA-256へ再投入する。全byteの`sync_all`とhash一致後だけ完成basenameへrenameし、hash不一致はpartを残したままFailedにする。managed directory外canonical pathと既存symlinkを拒否する。peer transfer testは8件、workspace全70 testが成功した。次工程はCapturedAsset selectionとMedia Incomplete manifestへの接続である。

続いてAsset Transfer ManifestへOriginal、指定Derivative、Original＋指定Derivativeの選択modelを追加した。source CapturedAssetから各実fileの長さとSHA-256を読み、未知／重複resource IDを拒否する。Original／Derivativeのroleは型で分離し、DerivativeをOriginal adapterへ渡すとfile作成前に拒否する。Original受信は`IndexedOriginalReceive`でMedia Indexへ開始時Incompleteを記録し、全byte検証後に既存JPEG／ISO BMFF probe、送信元CaptureMetadataとの寸法／FPS validation、CapturedAsset作成、atomic Media manifest保存を通してFinalizedへ遷移する。probe／validation／manifest失敗はFailedとして診断を残し、manifest失敗時は完成resourceをIncompleteへrollbackする。未実装のmetadata除去を宣言だけで装わないため、builderは現在`Preserve`だけを許可し、Strip指定を拒否する。workspace全74 testが成功した。

## 現在地

- [Done] 元仕様 `Universal Film & Color Imaging Engine.md` の責務分離、色処理、ゼロコピー、カメラ抽象、エンコード要件を初期設計へ反映
- [Done] Cargo workspaceとTauri 2アプリシェルを作成
- [Done] `media-core` に共通フレーム記述、色空間、転送関数、CPU／ネイティブハンドルの境界を定義
- [Done] `camera-core` にスチル／動画モード、能力モデル、状態機械、バックエンド／セッションtraitを定義
- [Done] `film-core` にACEScg前提の画像、FilmRecipe、品質レベル、エンジンtraitを定義
- [Done] `imaging-core` にCamera Body/Exposure、Lens、Film/Digital Sensor、Chemical/RAW Development、Print/Output Transform、Displayの共通Pipelineを定義
- [Done] SignalDomainによる接続検証とObserved/Simulated/Transformのprovenanceを実装
- [Done] Tauri IPCはカメラ状態とモード選択だけに限定し、UIは英語・日本語・简体中文に対応
- [Done] 光学絞り・センサー・動画方向を統合した媒体非依存のベクター正本からdesktop/iOS/Android向けアイコンを生成（`APP_ICON.md`）
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

### 2026-08-30: Nearby JPEG privacy sanitizer

`peer-transfer-core::sanitize_jpeg_for_transfer`を追加した。`StripDeviceAndLocation`ではJPEGを一時fileへ再構築し、EXIF APP1、XMP、IPTC APP13、COM、未知のAPP segmentを除去する。色再現に必要なICC APP2、Adobe APP14と標準JFIF／JFXX APP0は保持し、SOS以後のpixel entropyはbyte-for-byteでコピーする。`sync_all`後に完成先へrenameし、出力byte数とSHA-256を返す。

選択的`StripLocation`はTIFF IFD pointerを安全に再構築できるまで拒否し、MOV／MP4も未対応とする。sanitizerはpathをprivateにした不透明な`SanitizedJpeg`を返し、`AssetTransferManifest::from_sanitized_jpeg_original`だけがこの証明から`StripDeviceAndLocation` manifestを作る。builderは再probeで元Originalと画素寸法を照合し、現在のbyte長とSHA-256がsanitizer直後のreportから変化していれば拒否する。通常のbuilderは引き続きStrip policyを受け付けず、元fileを除去済みと偽装できない。peer-transfer-coreは13 testが成功した。

### 2026-08-30: Nearby Derivative provenance finalize

`TransferResource`へoptionalな`derivative_provenance`を追加し、source builderはDerivativeごとにparent resource ID、完全なrender snapshot、engine version、seedを格納する。`IndexedDerivativeReceive`はこの来歴が存在し、受信先のFinalized `CapturedAsset`に同じparent resource IDがあり、media typeが一致するときだけIncomplete writerを開始する。hash確定後にprobeし、`CapturedAsset::add_derivative`の来歴検証を通して親asset manifestを更新する。成功後は一時的なrecovery recordを除去し、Derivativeを独立OriginalとしてMediaへ公開しない。

この縦切りは、受信側に同じresource IDを持つ親assetがすでに存在する場合を完成させた。Original＋Derivativeを同時受信して新しいlocal IDへ割り当てる処理には、bundle coordinatorとsource→local resource ID mapが必要であり次工程とする。peer-transfer-coreは14 test、workspace全77 testを期待値とする。

### 2026-08-30: Original＋Derivative bundle coordinator

`BundleReceiveCoordinator`を追加した。`OriginalAndDerivatives` manifestについてOriginalが1件、Derivativeが1件以上であること、selectionのID集合とresource集合、transfer ID／resource IDの一意性、全resourceのmedia type、全親参照の存在、依存graphの非循環性を受信開始前に検証する。

Originalは利用者が選んだ安全なlocal asset IDで先に確定する。確定結果を明示的に`mark_original_finalized`へ渡した時点だけsource Original IDからlocal Original IDをmapへ登録する。Derivativeは親source IDがmapにある場合だけ準備でき、local親IDへprovenanceを変換して確定する。各Derivativeも確定結果を確認後にだけmapへ追加するため、受信予定や失敗resourceを完了済みとして扱わない。各resourceの`TransferSession`は従来どおりInvitation承認とtransport交渉を必須とし、coordinatorはその境界を迂回しない。peer-transfer-coreは15 test、workspace全78 testを期待値とする。

### 2026-08-30: Authenticated chunk transport contract and resume proof

`EncryptedChunkCodec`を追加し、32-byte session keyでChaCha20-Poly1305を使用する。frameはtransfer ID、offset、平文長、nonce、ciphertextを持ち、AADへprotocol version、transfer ID、offset、平文長、asset総長を含める。nonceはsession固有prefix、key、transfer ID、offset、平文hashから導出するため、同じoffsetで内容が変わってもnonceを再利用しない。key materialは`Zeroizing`で保持する。改ざん、offset変更、別transfer／asset長への転用はwriterへ渡す前に拒否する。

`ReceiveWriter::resume_checkpoint`はledgerとpart fileがdurableになった`persisted_bytes`と、そのprefix SHA-256だけを返す。送信側は`verify_resume_checkpoint`で元fileの同じprefixを再hashし、`TransferSession`がresume対応を交渉済みの場合だけoffsetを採用する。checkpoint以後もencrypted chunkは通常のdurable ACK、全file SHA-256、atomic rename境界を通る。

この段階のkeyは上位handshakeから注入する契約であり、ephemeral key agreement、確認code binding、実socket adapterは未実装である。したがって製品としてend-to-end encryption完成とは扱わない。peer-transfer-coreは16 test、workspace全79 testを期待値とする。

### 2026-08-31: X25519 handshake and transcript confirmation

`EphemeralKeyPair`と`AgreedSessionSecrets`を追加した。platform CSPRNGが供給する32 byteからsession限定X25519 key pairを作り、low-order／all-zero shared secretと自己public keyを拒否する。secretと導出keyはzeroize対応型で保持する。

6桁確認codeは事前共有PINではない。X25519 shared secret、sort済み双方public key、Invitation ID、sender ephemeral ID、transfer ID、asset SHA-256、byte長から双方が同じ値を導出し、利用者が二画面で比較する。公開鍵、Invitation、Manifestのいずれかが差し替えられるとcodeが変わる。双方が確認したcodeをHKDF-SHA256 saltへ含め、同じtranscriptから32-byte chunk keyと16-byte nonce prefixを導出する。これを既存ChaCha20-Poly1305 codecへ直接変換できる。

Rust共通層は乱数を自作せず、OS別adapterがCSPRNG secretを供給する。socket transport、CSPRNG adapter、session終了／cancel／error時の統合key lifecycle試験は次工程である。peer-transfer-coreは17 test、workspace全80 testを期待値とする。

### 2026-08-31: OS CSPRNG and bounded local-network framing

`EphemeralKeyPair::generate`を追加し、Rust `getrandom`経由で各OSのCSPRNGから32-byte session secretを取得する。all-zero検査と既存zeroize lifecycleを通るため、applicationが通常経路でsecret byteを生成・保持する必要はない。native security provider固有の監査が必要なplatformでは、従来の`from_secret_bytes`へ明示的に供給できる。

`LocalNetworkTransport`は`TcpStream`へbounded binary protocolを接続する。messageはEncryptedChunk、ResumeCheckpoint、DurableAckの3種類で、magic、protocol kind、u32 payload長を共通headerに持つ。受信側は最大chunk＋固定header上限をallocation前に検査し、transfer ID、ciphertext長、hash形式、末尾余剰data、未知kindを拒否する。JSONで暗号byte列を膨張させない。

テストはsocket bind可能な環境ではIPv4 loopback TCPを往復し、sandboxがsocketを禁止する環境では同じ`Read`／`Write` binary framingをmemory streamで検証する。oversize headerはpayloadを確保・読込する前に拒否する。connection timeout、cancel、切断後resume orchestration、Bonjour／Nearby discovery、Tauri command接続は次工程である。peer-transfer-coreは18 test、workspace全81 testを期待値とする。

### 2026-08-31: Encrypted transport lifecycle milestone

`EncryptedTransferSender`と`EncryptedTransferReceiver`を追加した。生成時に`TransferSession`が承認・高速transport交渉後のTransferringであること、codecとmanifest identityが一致することを要求する。senderは元file全体をmanifest SHA-256と照合してから開き、交渉chunk長で1 chunkだけを暗号化する。対応するdurable ACKが正確なend offsetを返すまで次chunkを送らないstop-and-wait方式とした。

切断時はsenderを`PeerDisconnected`へ移し、receiverのdurable prefix checkpointをsession resume能力と元file prefix hashの双方で検証した場合だけ再開する。receiver再生成は既存ledger／part fileを再hashする従来境界を通る。最終ACK後にだけCompleteとなり、`ReceiveWriter`を既存の全file hash／atomic rename finalizeへ渡せる。User／Timeout／PeerDisconnected cancel wire messageを追加し、Cancelledからの送信、Complete後のcancel、ACK飛越しを拒否する。TCP read／write timeoutは0以外だけ設定できる。

暗号化、切断、checkpoint resume、残chunk、全体hash、完成renameを1本のtestで通した。peer-transfer-coreは19 test、workspace全82 testを期待値とする。次の大きい境界はApple Bonjour discoveryとTauri commandであり、mobile background／network切替は実機検証が必要である。

### 2026-08-31: Apple Bonjour discovery and Tauri command boundary

Tauri applicationへ`nearby_discovery` stateを追加し、`start_nearby_discovery`、`get_nearby_discovery`、`stop_nearby_discovery` commandを登録した。startは最初にIPv4 unspecified addressへTCP listenerをbindし、port 0ならOS選択portを取得する。その後OS CSPRNG由来X25519 key pairを生成し、`_ufcamera._tcp.local.`をmDNS advertise／browseする。Apple P2P interfaceを明示的に含める。

TXT recordはprotocol version、public key由来12桁ephemeral ID、64桁public key、利用者が任意入力した32文字以内labelだけを持つ。端末名、永続device ID、secret key、確認codeは広告しない。受信TXTはversion、IDとpublic keyの対応、hex形式、port、resolved addressを検証し、自己serviceを除外する。daemon errorはsnapshotの`last_error`へ保持する。stop／application state dropでbrowse、register、daemon、listener、ephemeral secretを終了する。

Apple `Info.plist`へ`NSLocalNetworkUsageDescription`と`NSBonjourServices`、sandbox entitlementへnetwork client／serverを追加した。用途説明は英語、日本語、简体中文を用意した。`mdns-sd` 0.21.0を固定し、async runtimeに依存しないdaemon channelをTauriのpoll型snapshotへ接続した。現段階ではlistenerを予約・広告するところまでで、accept後のInvitation／handshake／transfer taskとUIは次工程である。workspace全84 testを期待値とする。

### 2026-08-31: Nearby discovery UI milestone

撮影画面のright／bottom railへNearby入口を追加し、専用画面で`start_nearby_discovery`、`get_nearby_discovery`、`stop_nearby_discovery`を接続した。専用画面へ移る前にnative camera previewを停止し、戻ると検出を停止してcamera discovery／previewを再開する。周囲へのadvertiseが画面を閉じた後も意図せず残らないことを優先した。application unloadでもstopを要求する。

画面は英語、日本語、简体中文で、local ephemeral ID、peer label／ephemeral ID、protocol version、resolved endpoint、privacy説明、daemon errorを表示する。active中は1.5秒ごとにsnapshotを更新する。IPv6 endpointは`[address]:port`表記にして曖昧さを避ける。開発用`?nearby-fixture=1`は実networkへ接続せず2 peerを表示し、desktop／375px幅のvisual QAに使える。

この節目は発見UIまでである。peer cardを押しても接続済みとは扱わない。次工程はMedia asset選択、Invitation、双方の6桁確認code、明示承認を1つのstate machineとしてnative listener／outbound connectionへ接続する。workspace全84 testを期待値とする。

### 2026-08-31: Outgoing transfer approval preparation

Nearby画面で発見peerとFinalized Media Originalを選択し、native `prepare_nearby_approval`へ渡す縦切りを追加した。native側はMedia indexからIDを再解決し、Incomplete／Failedやasset本体を持たないentryを拒否する。`AssetTransferManifest::from_captured_asset`が実fileのbyte長とSHA-256を計算し、256 KiB chunk、Preserve metadata policyのOriginal manifestを作る。

Invitation IDはOS CSPRNGの16 byteから生成し、有効期限は2分とした。発見sessionのlocal X25519 key、mDNSで検証したpeer public key、Invitation ID、sender ephemeral ID、Manifest identityから既存protocolの6桁codeを導出する。UIは英語、日本語、简体中文でpeer／asset選択、code比較、cancel、local approveを表示する。375×812と1280×800でfixture操作を検証した。

local approve後の`TransferSession`はNegotiatingであり、転送開始済みではない。TCP control frameでremoteへInvitation／Manifestを送り、相手側が同じcodeを導出・承認した事実を認証して受け取るまでは`AgreedSessionSecrets`を生成しない。次工程はincoming acceptとoutbound connectを同じ相互handshake stateへ接続することである。workspace全85 testを期待値とする。

### 2026-08-31: Mutual handshake wire and codec completion

`PeerWireMessage`へHandshake OfferとHandshake Approvalを追加した。OfferはInvitation、単一resource Transfer Manifest、sender X25519 public key、sender capabilityを運ぶ。ApprovalはInvitation ID、transfer ID、同じ6桁code、approver public key、approver capability、明示approved flagを運ぶ。既存UFC1 frameを使うが、chunk上限とは別にcontrol payloadを64 KiBへ制限し、header読取後・payload allocation前に拒否する。

`complete_mutual_handshake`はlocal sessionがNegotiatingで、OfferのInvitation／Manifestと完全一致し、Approvalが同じInvitation ID／transfer ID／codeへbindingされ、双方public keyが異なり、local keyがいずれかのpartyである場合だけ進む。双方のLocalNetwork capability、chunk上限、resume能力を交渉し、その後にX25519／HKDFを実行してTransferring sessionとEncryptedChunkCodecを同時に返す。

memory framingに加えてIPv4 loopback TCPでOffer→Approvalを往復し、sender／receiver双方でcompleteしたcodec間だけChaCha20-Poly1305 chunkを復号できるtestを追加した。異なるtransfer IDのApprovalはcontext mismatchで拒否する。共通protocolは完成したが、Apple discoveryが保持するlistenerをacceptするtask、outbound address選択、incoming UI、timeout／cancelは次工程である。peer-transfer-coreは20 test、workspace全86 testを期待値とする。

### 2026-08-31: Apple two-party handshake task boundary

Bonjour start時に確保したnonblocking TCP listenerをsnapshot pollへ接続した。接続が来た場合は2秒timeoutで最初のHandshake Offerを読み、Offer sender ephemeral IDとpublic keyが現在のmDNS discovery結果へ完全一致するときだけincoming approval stateを作る。受信側もlocal keyからconfirmation codeを再導出し、Offer内codeとの不一致をUIへ出す前に拒否する。

送信側の`connect_nearby_transfer`はlocal approve後だけ実行でき、resolved addressを順に5秒timeoutで接続する。Offer送信後は最大125秒remote Approvalを待つが、その間Nearby state mutexを保持しない。remote contextを検証後に再lockし、同じapprovalがcancel／置換されていないことを確認してからsession keyを消費する。受信側はlocal approve時にApprovalを返し、双方とも`complete_mutual_handshake`を通したTransferring session、EncryptedChunkCodec、TCP transportをnative stateへ保持する。

三言語UIはincoming offerをpoll snapshotから自動表示し、方向、asset名／容量、peer ID、6桁codeを示す。送信側はlocal approve後にoutbound commandを非同期開始し、remote approval完了後に「安全なセッションを確立しました」と更新する。fixture UIと共通loopbackは検証済みだが、Bonjourを介したmacOS／iOS実機2台の受け入れは未完了である。次工程は保持したsecure sessionをEncryptedTransferSender／Receiver、受信Media lifecycleへ接続する。

### 2026-08-31: Encrypted Original to Media Finalized vertical slice

Handshake OfferへOriginal `TransferResource`とCaptureMetadataを追加し、受信側が送信元pathへ依存せずIndexedOriginalReceiveを作れるようにした。Offer validationはresource roleがOriginal、derivative provenanceなし、resource manifestとhandshake manifestが完全一致することを要求する。

secure session確立後、送信側は`EncryptedTransferSender`でFinalized Originalをnegotiated chunkごとに暗号化し、各DurableAckを既存TransferSessionへ反映する。受信側は保存先filesystem残容量と256 MiB reserveをwriter作成前に検査し、`.incomplete/peer-transfer`へ認証済み平文だけを書き、sync済み連続offsetだけをACKする。全byte後は実SHA-256、media probe、CapturedAsset validation、Media manifest保存を通る`finalize_indexed_original`を実行する。

receiver確定後にtransfer IDとManifest SHA-256を持つ`TransferFinalized` control messageを返す。senderはこれを照合するまでUIを完了にしないため、receiverのhash／probe／Media保存失敗を送信成功と誤認しない。Encrypted IndexedOriginalReceiveのtestを追加し、改ざん認証境界からMedia Finalizedまでを検証した。

既存resume ledgerのpersisted offsetが0より大きい場合、現時点のApple orchestrationは明示エラーで停止する。先頭からの再送でpartialを上書きしない。次工程はreceiver checkpointをhandshake直後に送り、sender prefix hash照合後にそのoffsetから再開することと、progress／cancel UIである。peer-transfer-coreは21 test、workspace全87 testを期待値とする。

### 2026-08-31: Apple durable checkpoint transfer start

Apple secure transfer taskはasset chunk送信前にreceiverから`ResumeCheckpoint`を必ず返す。receiverは既存のMedia Incomplete ledger、part file長、保存済みprefix hashを`IndexedOriginalReceive::create_or_resume`で検証してからcheckpointを送る。senderの新しい`EncryptedTransferSender::open_at_checkpoint`は元file全体のManifest hashに加え、checkpoint位置までのprefix SHA-256、transfer ID、offset上限、resume交渉状態を確認し、そのoffsetを最初のencrypted chunk位置にする。

fresh transferもoffset 0 checkpointを通るため、fresh／resumeで別の非認証開始経路を持たない。不正prefixをchunk送信前に拒否するtestを追加し、peer-transfer-coreは22 test、workspace全88 testが成功した。socket切断後に同じtransfer IDを保持して再発見／再handshakeする制御、live progress、cancel／retry UI、実機2台試験は未完了である。

### 2026-08-31: Durable progress and cancel milestone

Prepared Approvalとsecure sessionが共有するatomic progress stateを追加した。転送taskはglobal Nearby mutexを外して動作し、receiverのDurableAckをsessionへ反映した後だけ`transferred_bytes`を進める。snapshotは`transfer_active`、`cancel_requested`、durable byte数を返すため、既存1.5秒pollのままnative taskを妨げず進捗を取得できる。

`cancel_nearby_secure_transfer`はactive taskへcancel flagを設定する。sender／receiverはchunk境界で検出し、同じtransfer IDの`PeerWireMessage::Cancel(User)`を相手へ送って停止する。UIは英語、日本語、简体中文で割合、転送済み容量／総容量、progress bar、安全停止中状態を表示し、active中のdialog closeと重複cancelを無効化する。

Browser fixtureでdesktopと375×812を使い、peer選択→Finalized asset選択→6桁code承認→42% progress→cancelを操作確認した。Browser単体ではTauri IPCがないためnative preview初期化の既知errorが1件出るが、Nearby fixture由来のconsole errorはない。TypeScript buildとworkspace全88 testは成功。socket read中の即時cancel、切断再接続、失敗分類／retry、Apple実機2台は未完了である。

### 2026-08-31: Apple same-transcript reconnect milestone

X25519 ephemeral key pairを一度のsocket handshakeで消費せず、Nearby可視セッションownerが検出停止まで保持するよう変更した。`complete_mutual_handshake`はkey pairをborrowし、同じ承認済みtranscriptで再handshake codecを導出できる。永続keyにはせず、Nearby画面離脱／discovery stop／application終了で従来どおりzeroize対象のownerごと破棄する。

transfer taskが切断エラーで終わるとsecure socketだけを破棄し、Prepared Approval、同じOffer／transfer ID、durable progress、受信partialを`retry_available`として保持する。senderの明示retryは新しいTCP接続と`TransferSession`を作る。receiver pollは現在発見中の同じephemeral ID／public keyかつ完全一致Offerだけを自動再承認し、双方がsecure sessionへ戻る。直後に既存checkpoint交換が走るため、保存済みprefix SHA-256が一致したoffsetから暗号化転送を再開する。

UIは英語、日本語、简体中文で「接続が中断されました・検証済みの受信データは保持されています」と「再接続して再開」を表示する。fixtureで42%中断→retry→暗号化転送中への復帰をdesktopで確認した。core testは同じvisibility keyから双方codecを2回導出し相互復号できることを検証する。workspace全88 testは成功。Invitation 2分失効後、background／interface変更、自動backoff、Apple実機2台は未完了である。

### 2026-08-31: Two-device validation deferred

macOS／iOS実機2台を使うNearby end-to-end検証は、現在検証機材がないため`[Later]`へ移した。これはcode実装の後退や失敗ではなく、Bonjour discovery、二画面code比較、相互承認、暗号化転送、物理的なnetwork切断とcheckpoint再開を実環境で確認するQA項目の保留である。

直近の開発順序やmilestone完了条件へ実機2台試験を置かない。機材確保後は、既存loopback／fixture結果をbaselineとして、異なる端末間の発見、Still／Video、途中切断、再接続、最終hash、受信Media Finalizedを一連で確認する。

### 2026-08-31: Failure taxonomy and recovery policy

Nearby transfer失敗をDisconnected、Timeout、Integrity、Storage、InvitationExpired、Cancelled、Protocolへ分類し、snapshotへ`failure_kind`を追加した。retryはDisconnected／TimeoutかつInvitation有効・cancel未要求の場合だけ許可する。connect段階とchunk transfer段階の双方が同じ分類記録を通るため、接続失敗がUIの一時errorだけで消えない。

UIは英語、日本語、简体中文で失敗ごとの次動作を表示する。IntegrityはMedia未公開、Storageは空き容量確保、Expiredは新しい確認code、Protocolは承認のやり直しを案内する。Integrity／Storage等に「再接続して再開」は表示しない。partialは診断・復旧用に保持し、この工程では自動削除しない。

分類とretry可否のunit testを追加し、universal-film-cameraは8 test、peer-transfer-coreは22 testを期待値とする。Browser fixtureでIntegrityがretry不可、Timeoutがretry可能であることをdesktop／375×812で確認した。次工程は不要partialの明示discardとInvitation失効後の新規承認導線である。

### 2026-08-31: Explicit partial discard and renewed approval path

peer-transfer-coreへ`discard_incomplete_transfer`を追加した。transfer IDからmanaged part／ledgerだけを解決し、safe token、canonical directory、symlink、ledger schema、ledger Manifest identityを検査してから、既存Media Indexのrecoverable cleanup境界でpartと非Finalized manifestを削除し、resume ledgerを除去する。path文字列をTauri IPCから受け取らず、Finalized Media cleanupを拒否する既存契約を再利用した。

Tauri `discard_nearby_partial`はIncoming、失敗分類あり、task非active、非Finalized、secure sessionなしの場合だけ実行できる。UIは三言語の確認dialogで、削除対象とFinalized非対象を明記する。「復旧用に保持」は状態を残し、「途中データを破棄」の明示確定後だけnative削除する。Outgoingの期限切れ／非retry失敗はfile削除を行わず、Approvalを閉じて同じpeer／asset選択から新しい確認codeを準備できる。

core testは対象part／ledger／Media記録が消え、path traversal IDが拒否されることを検証する。peer-transfer-coreは23 test、workspace全90 testを期待値とする。Browser fixtureでIncoming Integrity failureの保持／破棄、Outgoing InvitationExpiredから新規承認への復帰、375×812 dialogを確認した。Browser単体のTauri IPC不在による既知native preview error以外に対象flowのerrorはない。

### 2026-08-31: Next queue closure — capability, lifecycle, scopes, retention

`CameraCapabilities`へ型付き`CaptureOutputCapability`を追加し、JPEG／HEIF／DNG／QuickTime／MP4、JPEG／HEVC／RAW／H.264／ProRes／AAC／PCM、bit depth、bitrate範囲、audio sample rate／channel数を表現可能にした。Apple backendは現行実装で成立するJPEG StillとH.264＋AAC QuickTimeだけを組として報告する。未実装codecをUIが推測して選択可能にしてはいけない。

window closeは録画をfinalizeしてpreviewを停止してから破棄し、background遷移は即座に録画停止、foregroundはdevice再発見を行う。foreground中は5秒間隔でactive deviceのpresenceを確認し、confirmed disconnect時だけ安全停止／再発見へ移る。共通`CameraLifecycleEvent`／`CameraRecoveryAction`はnative notification adapterが同じ判断を再利用する境界である。

`media-core`へBGRA8のRGB histogram、waveform、vectorscope、false color、focus-peaking CPU referenceを追加した。これは将来のMetal／Vulkan／D3D rendererのconformance正本で、4K60実時間CPU処理を意図しない。Media recoveryは7日保持をadvisory candidateとして返し、三言語UIから確認付き一括削除できる。Finalizedを含むbatchはnativeで拒否し、自動削除は行わない。

`npm run build`とRust workspace全95 testが成功した。実機orientation／permission／background、実機2台Nearby、Windows／Linux runtime、4K60 GPU、署名／notarizationは環境または製品IDが必要なため、再開条件付き`[Later]`へ移した。これらをcode完成として`[Done]`に読み替えてはいけない。

### 2026-08-31: iOS-only active development policy

ユーザー方針によりiOSを唯一のactive platformへ変更した。macOS／Androidは既存artifactと回帰testだけを維持し、Windows／Linux、他OS peer adapterを含む追加開発は保留する。Roadmapの`[Next]`はiOS Simulator lifecycle回帰とiPhone実機camera受け入れだけである。

Apple preview runtimeのhealthを返す`get_camera_runtime_health`を追加した。foreground中の5秒監視はdevice presenceに加えて、preview hostが残ったままAVCaptureSessionだけ停止した状態を検出する。録画pendingなら共通finalizeを要求し、WebView側のactive runtimeを破棄してdevice再発見へ戻す。これはOS interruptionの完全なnative通知置換ではなく、通知を取りこぼした場合の回復防衛線である。

`cargo build -p universal-film-camera --target aarch64-apple-ios-sim`は成功し、追加したRust／AVFoundation／UIKit境界がiOS Simulator targetでcompile／linkできることを確認した。最初の`tauri ios build`は古い`build/arm64-sim/Universal Film Camera.app`を出力先としてrenameしようとして`Directory not empty`になった。archiveだけでなくこの最終productをXcode cleanする必要がある。

`scripts/build_ios_simulator.sh`はXcode projectのgenerated productだけをcleanしてからTauri正規buildを実行する。この手順でbundle生成に成功し、iPhone 16e Simulatorへinstall／launchした。portrait狭幅の実画面でstatus bar、parameter strip、preview、scope、bottom rail、中央正円capture controlがSafe Area内に収まり、不自然な見出し改行がないことを確認した。実機installはこの工程では行っていない。

再実行でXcode cleanはTauri独自の`build/arm64-sim`を消さないことが判明したため、wrapperはその生成済み`.app`一件を明示削除してからbuildする形へ修正した。削除対象pathは固定で、app dataや撮影assetへ到達しない。

Computer UseでiPhone 16e Simulatorを操作し、Homeによるbackground、foreground復帰、landscape、portrait upside-down、Media empty画面を確認した。landscapeでframe guideがtimecodeを横切る表示を発見し、狭高さlandscapeのtimecodeへ半透明背景を追加して再build／再install後に解消を確認した。iPhoneのsupported orientationsへPortraitUpsideDownも追加した。Simulator回帰は完了し、次のactive gateは接続済みiPhone実機でpermission、Preview、Still、音声Video、保存asset、orientationを受け入れることである。

### 2026-08-31: signed iPhone build and launch

利用者が指定したDevelopment Team `3WH28SSRZC`をTauri iOS bundle設定へ追加し、`scripts/build_ios_device.sh`でarm64 debug buildとdebugging exportを再現可能にした。Apple ID、署名証明書ID、provisioning profile UUID、端末UDIDはrepositoryへ保存しない。Xcodeの既存Keychain認証とautomatic provisioningを利用する。

署名済み`Universal Film Camera.ipa`を生成し、bundle identifierとTeamIdentifierを確認した。接続済みiPhoneへのinstallと`app.universalfilm.camera`のprocess launchも成功した。`scripts/install_ios_device.sh <device-udid>`はIPAを一時directoryへ展開して同じ操作を再現する。ここまでを`[Done]`とし、端末画面上のcamera／microphone permission許可、native Preview、Still、音声Video、Media保存物、回転、background復帰は人による操作確認が必要なため`[Next]`に残す。

同実機のportrait画面で、狭幅時の下部right railが中央シャッターへ侵入し、スコープボタンのタップ領域までシャッターに奪われる不具合を確認した。原因は、7個の48pt touch targetと56ptシャッターを単一行へ配置していたことだった。480px以下では補助操作と撮影navigationを2段、スコープ展開時はpaletteを含む3段へ分け、中央列をシャッター専用にした。320×700と390×844のbrowser回帰で、閉状態／展開状態とも出力、スコープ、Media、Nearbyの矩形とシャッターの交差が0であることを確認した。

続く実機指摘で、capability適用時にLENS／IRISを常に`—`、WBを常にdisabledへしており、SHUTTERも表示だけでnative適用commandがないことを確認した。Apple capabilityへlocalized lens名、固定lens aperture、現在の露出時間、manual WB対応と現在の色温度を追加した。SHUTTERはAVFoundation custom exposureを現在ISOのまま1/24〜1/1000秒へ、WBはcustom device gainsを介して2000〜10000Kへ適用する。iPhoneの絞りは固定なので実ƒ値を表示し、可変操作とは扱わない。調整panelはnative previewに隠れる領域からparameter stripへ移した。viewportの拡大上限と`touch-action: manipulation`も設定し、撮影操作のダブルタップ拡大を抑止した。

次の実機指摘では、映像の上ずれ、メニューが映像の下へ潜ること、グリッド等のoverlayが出ないことが同時に発生した。原因はiOS preview hostをWKWebViewの末尾childとして追加し、native UIViewが全Web UIより前面にあったことと、その回避として`has-native-preview`中のoverlayをCSSで非表示にしていたことだった。preview hostをWKWebViewのsuperviewへ座標変換してWKWebView直下へ挿入し、WKWebViewとDOM preview領域を透明化した。resize時もWKWebView座標からcontainer座標へ毎回変換する。Web側のsafe frame、centre mark、timecode、monitor status、histogram、audio meter、調整menuは前面に残す。

カメラparameterの操作性を続けて修正した。Apple discoveryへUltra Wide／Wide／Telephotoの物理device typeを追加し、LENS selectorからpreview sessionを安全に交換する。IRISは選択可能な情報panelを開くが、iPhoneの物理絞りは固定なので単一の`ƒ/x.x · FIXED`だけを提示し、可変絞りを偽装しない。EIは現在露出時間を保持したAVFoundation custom ISOへ接続した。

SHUTTERの1刻みrangeは廃止し、1/24、1/25、1/30、1/40、1/48、1/50、1/60から1/1000までの撮影用離散値へ変更した。EIも1/3段相当の代表値、WBも2000K〜10000Kの代表値へ止まるnative selectとした。FPS panelは狭幅で1列表示し、解像度を`1920×1080`のような幅×高さだけで表示する。`4K`、`UHD`、`1080p`、`720p`などの重複する別名は付けない。cadenceは`24 fps · CINEMA`、`25 fps · PAL`、`30 fps · NTSC`、50/60以上をHFRとして表示する。390×844 fixtureで全selector、値列、console無errorを確認した。

撮影画面が再び黒くなった原因は、WKWebView本体を透明化しても内部`UIScrollView`が不透明なまま、背面の`AVCaptureVideoPreviewLayer`を覆っていたことだった。iOS attach時に両方のbackground／opaqueをclearへ統一した。

実機ではさらにCSS `:root`の暗色backgroundがWeb content canvasを塗っていたため、`body`以下だけを透明化しても映像は見えなかった。native preview状態を`html`と`body`の両方へ同期し、root canvasも透明化した。telemetryが動くのに映像だけ黒い場合は、capture sessionではなくこのcompositing chainを先に調べる。

固定SVGだったhistogram／audio meterを実測値へ置換した。`AVCaptureVideoDataOutput`は遅延frameを破棄する専用serial queueで受け、端末内で約160×90点へ間引いて32-bin RGB histogramを生成する。BGRA以外のnative planar formatでは第1planeのlumaをRGB共通分布として扱う。WebViewへcamera pixelは渡さずbin集計だけを約150ms周期で取得する。audio meterは権限済みならpreview開始時からmicrophone inputをsessionへ追加し、AVFoundation audio channelの`averagePowerLevel`（dB）を表示する。権限なしではstill previewを失敗させず0表示とする。

この変更はarm64 iOS camera crate check、本番Web build、workspace全95 test、署名済みarm64 IPA buildまで成功した。実機で被写体の明暗を変えたときのhistogram、発声時のmeter、preview透過合成を目視確認するまでは受け入れ完了ではない。

環境設定を独立pageとして追加し、表示／撮影／カラーのtabへ分けた。カラーtabではmonitoring targetとしてRec.709、Display P3、sRGBを選択でき、WebViewのlocal storageへschema付きkeyで保存して次回起動時に復元する。撮影画面のmonitor labelにも即時反映する。内部working spaceは規範仕様どおりscene-linear ACEScg固定として読み取り専用表示する。

この選択は現段階ではUI preference／monitor labelであり、native `AVCaptureVideoPreviewLayer`のpixelへODTを適用していない。Display P3やsRGBを選んだだけで実映像が変換済みだと扱ってはいけない。native preview output transform、Still／Video asset metadata、選択中のdevice formatが持つcolor space能力との照合はRoadmapの`[Next]`である。

環境設定へLUT tabを追加した。内蔵catalogはClean、daylight／tungsten／pastel／consumer negative、neutral／vivid／warm reversal、warm／cool release print、bleach bypass、faded archive、panchromatic／orthochromatic monochromeの17項目で、実在film stock名を使わないgeneric archetypeである。測色データや現像条件のない実在銘柄を「再現」と表示してはいけない。

外部LUTは3D `.cube`だけを受理する。最大4 MiB、`LUT_3D_SIZE` 2〜65、宣言した三乗個のRGB sample、有限値と許容範囲、TITLE長を検証し、1D LUTと破損／欠落sampleを拒否する。ユーザーfile nameは保存pathへ直接使わずASCII safe stemへ変換し、Application Supportの`luts` directoryへ`.partial`のflush／sync後renameで保存する。

重要: 現在完了しているのはcatalog、選択、import、永続化である。選択LUTのnative preview／Still／Video pixelへの適用は未実装で、Roadmapの`[Next]`である。仕様どおりFilm Simulation全体をLUTと同一視せず、LUTはbaked approximationとしてdomain、input/output color space、hashをasset metadataへ残す必要がある。

### 2026-09-01: Media context actions and iOS Photos export

Media cardへ長押し、右クリック、Shift+F10／Context Menu keyで開く三言語context menuを追加した。pointer移動が10pxを超えると長押し判定を解除するため、縦スクロールを妨げない。menuはviewport端から16px内側へclampし、完成済み写真だけに「写真アプリに保存」を表示する。削除は写真／動画／復旧対象の全cardから選べるが、必ず別dialogで対象file名を示して再確認する。

完成済みassetの削除は`MediaIndex::delete_finalized`へ集約した。originalと全derivativeおよびmanifestをIDから解決し、全resourceがcanonical captures directory内のregular fileであることを事前検査する。resourceを先に、manifestを最後に削除する。途中失敗時にmanifestを残して診断可能にする設計で、recoverable cleanupからFinalizedを削除できない既存境界は維持する。

iOS Photos書き出しは`PHAccessLevelAddOnly`だけを要求し、libraryの読み取り権限は要求しない。Media IndexでFinalized Photoを確認し、canonical captures内のfileだけを`PHAssetCreationRequest`へ渡す。`NSPhotoLibraryAddUsageDescription`は英語、日本語、简体中文で追加済み。書き出しはcopyなので、成功後もアプリ内originalを保持する。Photos側の重複判定やalbum分類は現時点では行わない。

検証はcamera-core Media Index 11 test、TypeScript／Vite production build、aarch64-apple-ios target checkに合格した。390×844 browser fixtureではcontext menuの「写真アプリに保存」「削除」、削除確認dialog、16px safe margin、console errorなしを確認した。実際のPhotos permission promptと保存結果は署名済みiPhoneでの手動受け入れ対象である。

実機で削除が失敗した原因は、iOSのアプリ更新でdata container UUIDが変わっても、manifest内の絶対pathが撮影時のcontainerを指し続けていたためである。OSは撮影fileとmanifestを新containerへ移行していた。Media Indexは記録pathが存在しない場合だけ、`captures`以下の相対部分を現在のcanonical captures directoryへ再解決し、範囲内のregular fileであることを再検査する。削除、preview、Photos書き出しが同じ解決境界を使う。任意pathをIPCから受け取る設計にはしていない。

詳細表示の一覧は外側containerの`overflow: clip`に閉じ込められていたため、Media grid自体を`min-height: 0`、`overflow-y: auto`、`touch-action: pan-y`の専用scroll regionへ変更した。390×844、13件のfixtureで`clientHeight 401 / scrollHeight 4069`、gesture後`scrollTop 0 → 495`、console errorなしを確認した。

### 2026-09-01: framing guides and safe bulk migration to iOS Photos

撮影guideは3分割（3×3）、20%間隔の方眼、四隅を結ぶ対角線の3形式をSVG overlayとして実装した。環境設定の撮影tabで選び、`ufc.guide-style.v1`へ保存する。撮影画面のguideボタンは表示／非表示、環境設定は線種の選択という独立した役割である。overlayはpointer eventを受けず、native previewより前面の既存monitor overlay層に置く。

環境設定へMedia tabを追加した。「すべて保存してアプリ内コピーを削除」はMedia Indexの`Finalized + Photo`だけを対象とし、動画、Incomplete、Failedには触れない。件数と不可逆削除を別dialogで再確認し、iOS PhotosのAdd Only APIへ全写真を順番に保存する。1件でもPhotos保存に失敗した場合、アプリ内削除は1件も開始しない。

Photosへの全件保存後、`MediaIndex::delete_finalized_many`が全ID、original、derivative、manifestを最初にpreflightしてから削除を開始する。これにより後半assetの欠落や不正pathが原因で先頭assetだけ消える事態を防ぐ。filesystem削除そのものはtransactionではないため、実削除中のI/O障害ではPhotos側copyをrollbackできず、アプリ内cleanupが部分完了する可能性は残る。エラー時はPhotos側copyを維持し、残ったアプリ内assetを再試行可能にする。再試行時のPhotos重複排除は未実装である。

390×844 browser fixtureで3形式の切替、12写真の件数表示、最終確認dialogを検証した。長い日本語actionが横にはみ出したため、一括移行dialogのactionを縦積みにし、44pt以上のtouch targetと自然な折返しを確保した。検証では最終確定を押しておらず、実機の利用者写真は転送も削除もしていない。実機でのPhotos permission、全件保存、成功後cleanupは手動受け入れ対象である。

### 2026-09-01: live focus peaking and monitor color processing

従来のFocusボタンはcentre markしか切り替えておらず、LUT選択もcatalog preferenceに留まっていた。Appleの既存`AVCaptureVideoDataOutput` delegateで、histogramと同じcamera frameからaspect ratioを保った240px幅RGB monitor frameを生成する。BGRAは直接RGBへ変換し、iOSのbi-planar YCbCrはluma／chroma planeからvideo-range変換する。camera原寸pixelや撮影原本はIPCへ渡さない。

Web monitor layerは縮小RGB frameにSobel edge detectionを行い、閾値を超える輪郭をcyanで表示する。Focusボタンを有効にするとprocessed canvasがnative preview前面へ出て、無効にすると通常の高品質`AVCaptureVideoPreviewLayer`へ戻る。これは240px／約6.7fps telemetryのCPU monitor pathであり、4K60の収録処理やGPU zero-copy pathではない。

環境設定Color tabへ「カラープロファイルとLUTをライブ表示へ適用」を追加した。明示的に有効化した場合だけ、Rec.709 transfer、sRGB、Display P3 matrix monitor transformと、選択LUTをprocessed previewへ適用する。内蔵lookはgeneric film/process archetypeの決定論的変換で、測色済み実在stock再現ではない。外部3D `.cube`は管理directory内のcanonical fileだけをIDから再解決し、DOMAINとsampleを読み、RGB cubeをtrilinear interpolationする。

この処理はmonitoring専用で、Still／Video original、asset metadata、histogram入力を変更しない。LUT適用中の収録assetへmonitor look IDをsidecar記録する処理は未実装である。次世代経路は`CVPixelBuffer → Metal texture → 3D texture LUT／peaking kernel → MTKView`とし、現在のCPU pathをreference／fallbackとして残す。

390×844 browser検証ではColor tabの明示toggle、Bleach Bypass選択、撮影画面Focus操作、processed canvas表示、Safe Area内のbottom railを確認し、browser console error／warningは0件だった。Browser fixtureにはnative camera frameがないため、実際の輪郭追従、色、frame cadence、熱／電力は署名済みiPhoneでの手動受け入れ対象である。

実機確認でFocus有効時に画角が変わった原因は、ピーキング単独でも240px monitor frameを全面opaque表示し、`AVCaptureVideoPreviewLayer`を置換していたことだった。Focus単独時はcanvasの非edge pixelを完全透明にし、cyan edgeだけをnative previewへ重ねる。monitor color／LUTを明示的に有効化した場合だけprocessed frameを全面表示する。canvasはalpha contextとし、各frameでclearして過去のedgeを残さない。

iOSでformat適用後に「保存できませんでした: unknown」と出た原因は、`apply_camera_format`がiOSだけ永続化を明示的に無効化し、`settings_persisted=false`かつ`settings_warning=None`を返していたことだった。`camera_settings` module、app-data settings path、save／restoreをiOSにも有効化した。選択formatはdevice ID単位で`camera-format-v1.json`へatomic保存し、次回preview開始時に復元する。保存失敗時は実際のI/O／schema errorを表示し、`unknown`へ落とさない。

実機画像ではピーキング輪郭が実映像より粗く、動きに対して遅れて見えた。monitor sampleを240px／150msから360px／100msへ変更し、Sobel閾値を92から180へ上げた。これにより拡大時の1 sampleあたりの線幅、過剰検出、最大表示遅延を抑える。CPU／IPC fallbackであるため、素早いパン時の完全同期はMetal zero-copy版まで保証しない。

4つのmonitor toolは左からFocus Peaking、Zebra、Frame Guides、Scopesである。従来ZebraはUI stateしか切り替えず描画がなかった。現在はmonitor lumaが235/255（約92 IRE）以上のpixelへ半透明の白黒斜線を描く。Focus／Zebraは同時表示でき、monitor look無効時は該当overlay pixel以外を透明に保つ。狭いiPhoneの展開menuにも三言語labelを常時表示し、iconだけに意味を依存しない。

縦向きの下部撮影railは7列の対称gridとし、Scopes、Media、Nearbyを中央シャッターの左側、Settingsだけを右端へ配置する。中央の固定3.5rem列と左右3列ずつを対称にするため、シャッターの幾何学的中心と正円を維持できる。22rem以下では44pt級の操作領域を同一行へ安全に収められないため、既存の複数段fallbackを使う。横向きは撮影映像を優先する右side railを維持する。

### 2026-09-01: WebGL2 LUT preview acceleration

従来のlive lookは幅360pxのRGB frameを100msごとに取得し、JavaScriptのpixel loopで外部3D LUTの三線形補間、monitor color transform、Sobel peaking、zebraを順番に処理していた。この経路は約10HzのCPU reference／fallbackであり、LUTを有効にするとWebView main threadの負荷と表示遅延が増える。

高速経路としてWebGL2 fragment shaderを追加した。RGB source texture、RGBA8 3D LUT texture、Rec.709／sRGB／Display P3変換、Sobel peaking、約92 IRE zebraを1 passへ統合する。内蔵lookは選択時に33³ textureへ一度compileし、外部`.cube`は読み込んだgridをtexture layoutへ並べ替えてcacheする。画面のaspect-fill cropはshaderのtexture座標で行い、native previewと同じ画角を維持する。WebGL2が利用できない場合は既存Canvas 2D CPU rendererへ自動fallbackする。

monitoring処理中はsnapshot要求を最大約30Hz、無効時はhistogram／audio用の10Hzへ動的に切り替える。`get_camera_monitor_snapshot(include_preview)`は非処理時にRGB vectorのcloneとbase64化を行わない。Browserの合成fixtureではWebGL2経路、358×693 canvas、target 30fpsに対して観測約24.5fps、console error／warning 0件を確認した。これは静止fixtureとdesktop WebViewの値で、iPhone camera frameでのfps／frame latency／thermalは別途実測が必要である。

この改善でも`CVPixelBuffer → CPU RGB縮小 → base64 → Tauri IPC → WebGL texture upload`は残る。他のプロ向けcamera appと同等の30／60fps、低遅延、低発熱へ到達する最終経路は`CVPixelBuffer → CVMetalTextureCache → Metal 3D texture／kernel → native drawable`である。RoadmapではiOS Metal zero-copyを`[Next]`へ昇格し、現在のWebGL2／Canvas 2Dをreference／fallbackとして残す。

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
### Peaking / zebra coordinate alignment (2026-09-01)

- The native preview layer already received the current device rotation and front-camera mirroring, but `AVCaptureVideoDataOutput`, which supplies analysis frames, did not. This made peaking and zebra use a different coordinate system from the visible image.
- `apply_orientation_locked` now applies the preview rotation and `preview_mirrored` value to the monitor output connection as well. Preview, peaking, and zebra therefore share the same orientation before the common aspect-fill crop is applied.
- Regression checks should cover portrait, both landscape directions, rear camera, and front camera. A monitor overlay must never use `capture_mirrored`; it must follow `preview_mirrored` because it is composited over the preview rather than written to the captured file.
### Configurable peaking color (2026-09-01)

- Settings > Display provides cyan, red, green, yellow, magenta, and white peaking colors with a visible swatch.
- The selection is persisted as `ufc.peaking-color.v1` and is applied immediately by the CPU peaking renderer. Cyan remains the fallback for absent or invalid stored values.
- Labels and save confirmation are localized in English, Japanese, and Simplified Chinese.
### Shutter feedback sound (2026-09-01)

- Settings > Capture offers `standard`, `fresh`, `dslr`, and `silent`; the selection is stored in `ufc.shutter-sound.v1` and is previewed when changed.
- Sounds are generated locally with Web Audio, avoiding bundled third-party audio and licensing/provenance concerns. The selected feedback plays only for still capture, not video record start/stop.
- `silent` suppresses the app-generated feedback only. It cannot and must not attempt to bypass a mandatory iOS system shutter sound imposed by device or regional policy; the localized UI states this limitation.
### Native shutter suppression correction (2026-09-01)

- The earlier `silent` setting suppressed only Web Audio feedback. Still capture now passes `suppressShutterSound` through Tauri to `AVCapturePhotoSettings.shutterSoundSuppressionEnabled` on iOS.
- The property is set only when `AVCapturePhotoOutput.shutterSoundSuppressionSupported` is true. Apple documents that iOS returns false in jurisdictions where shutter sound production cannot be disabled; forcing the setting while unsupported throws an exception and is therefore intentionally avoided.
- Consequently `silent` is best-effort through Apple's public API, not a bypass. Unsupported devices/regions retain the system shutter sound, and the localized Settings copy explains this accurately.
### Imported shutter sound files (2026-09-01)

- Settings > Capture accepts MP3, M4A, WAV, CAF, and other iOS/WebKit-decodable `audio/*` files up to 5 MiB. Import immediately selects and previews the sound.
- The copied `Blob` and original filename are persisted in IndexedDB database `ufc-shutter-sounds-v1`, object store `sounds`, key `custom`; the source file is not referenced after import.
- The selected `custom` value remains in `ufc.shutter-sound.v1`. On startup the custom blob is restored before applying the selection. Re-import replaces the single managed custom sound and revokes the previous object URL.
- Audio decoding ultimately depends on the installed iOS/WebKit version. Playback failures are surfaced in the localized settings status instead of silently falling back.
### Icon-first interface redesign (2026-09-01)

- `design.md` now locks the app-wide system: professional Workbench, live-image priority, dark technical canvas, cyan navigation state, and icon-first navigation.
- The exposure strip overlays the live image with a restrained translucent technical surface instead of consuming a separate monitor row. In landscape the utility rail is reduced to 4.5rem so the camera viewport receives more area.
- Settings tabs, back/refresh actions, media view switching, monitor tools, capture mode, and destination navigation are visually icon-only. Localized `aria-label`, `title`, or visually hidden text remains; essential values, form labels, warnings, filenames, and destructive confirmations remain textual.
- Settings panels are flatter and internally scrollable. This avoids card-within-card density while retaining the existing five-tab information architecture and all behavior.

### Landscape Settings and Media split panes (2026-09-01)

- 横位置のSettingsは`Master–Detail`構成である。左側に表示／撮影／カラー／LUT／メディアのカテゴリをアイコン＋名称で固定し、右側の選択パネルだけを縦スクロールさせる。縦位置では従来どおり5個のアイコンタブへ戻すため、狭いiPhoneでカテゴリ名が横幅を消費しない。
- 横位置のMediaは`Catalogue`構成である。左側にAll／Finalized／Incomplete／Failed filterと件数、右側に表示切替、状態、サムネイル／詳細gridを置く。実体のcollection/project機能は未実装なので、参考画像にある架空project sidebarは追加していない。
- 844×390級の横位置では左ペインを9.5〜12rem、右ペインを`minmax(0, 1fr)`とし、thumbnailは3列、detailsは2列とする。90rem以上ではthumbnail 5列、details 4列まで拡張する。
- 撮影画面の右アプリrail、中央シャッター、Safe Area、44pt以上のtouch targetは維持する。Settings panelとMedia gridがそれぞれscroll ownerであり、ページ全体を二重scrollにしない。
- 横位置で展開する行き先menuは、Media／Nearby／Settingsへの遷移完了時に必ず閉じる。遷移後も`is-open`を残すと右下で内容ペインへ重なるため、`closeDestinationMenu()`を画面遷移境界から外さない。
- アプリrailからセクション間を直接移動できる。Media／Nearbyのopen境界も、既に表示中のSettingsまたは別ライブラリを先に閉じる。複数の主要sectionを同じgrid cellへ同時表示しない。
- Mediaのfilterまたはthumbnail／details表示を切り替えたときは`mediaGrid.scrollTop = 0`へ戻す。旧layoutのscroll offsetを保持すると、件数の少ないfilterで先頭cardが途中から見える。
- 30rem以下ではCamera／Media／Nearby／Settingsの固定`min-height`を解除する。320×568では下部tool railが168pxまで二段化されるため、25rem固定の作業面を残すと32px重なる。作業面をgridの残り高さへ縮め、各内部scroll regionで内容へ到達させる。
- Browser QAは844×390のSettings／Media、320×568、375×667、414×896、768×1024で実施した。全viewportでhorizontal overflow 0、44pt未満の可視button 0、主要面とtool railの間隔8px、console error／warning 0を確認した。Settings panelは`scrollTop 0 → 176`、Media detailsは`scrollTop 0 → 260`で独立scrollし、filter後は0へ復帰した。
- `npm run check`はWeb production buildとRust workspace 101 testに合格した。Development Team `3WH28SSRZC`でarm64 debug IPAを再生成し、接続中の`littlebuddha's iPhone13`へ`app.universalfilm.camera`をinstall／launchした。横位置の実画面による最終目視は利用者受け入れ対象である。
