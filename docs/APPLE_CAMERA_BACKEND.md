# Apple Camera Backend

更新日: 2026-08-24
対象: macOS / iOS  
実装: `crates/camera-apple`

## 現在地

- [Done] AVFoundationのcamera権限状態を`CameraAuthorizationStatus`へ正規化
- [Done] 権限要求を非同期Tauri commandからblocking workerへ移し、WebView/UI threadを停止させない
- [Done] `AVCaptureDeviceDiscoverySession`で内蔵／外部video deviceを列挙
- [Done] deviceのstable ID、表示名、positionを`CameraDevice`へ変換
- [Done] macOSのcamera／microphone usage descriptionを英語・日本語・简体中文でbundleへ収録
- [Done] 非Apple targetでは同じcrateがstub backendとしてcompileできる境界を用意
- [Done] macOSで`AVCaptureSession`、camera input、`AVCaptureVideoPreviewLayer`をRust側に所有
- [Done] preview layerをWKWebViewのnative `NSView`子viewとして配置し、pixel dataをIPCへ流さず実機映像を表示
- [Done] Tauri commandからpreviewのstart／resize／stopを制御し、responsive resizeへ追従
- [Done] AppKitのcontent layout／Safe Areaからtitle bar差分を補正し、parameter stripとの重なりを解消
- [Done] `AVCapturePhotoOutput`とdelegateをpreview sessionへ追加し、JPEGスチルを実保存
- [Done] JPEGを`.partial`へ書いて`sync_all`後にrenameする原子的保存とfixture testを追加
- [Done] microphone権限、audio input、`AVCaptureMovieFileOutput`を同じsessionへ追加
- [Done] 録画開始／停止をTauri IPCへ接続し、最終delegate完了後にMOVを`.incomplete`から完成assetへrename
- [Done] camera／audio-input entitlementをTauri 2のmacOS bundle署名設定へ追加
- [Done] `AVCaptureDeviceFormat`から対応resolution、FPS、ISO範囲、manual shutter/focusを実列挙
- [Done] session開始後のactive resolution／FPSを取得し、Tauri IPCから撮影UIへ反映
- [Done] format単位のresolution／FPS組合せを列挙し、選択値を`activeFormat`とframe durationへ明示適用
- [Done] 録画中のformat変更をnative stateで拒否し、UIも選択controlを無効化
- [Done] 選択formatをdevice別JSONへ原子的に保存し、次回session開始時に復元
- [Done] JPEG／QuickTime保存後probeと選択format照合を`CapturedAsset`へ統合
- [Done] 1280×720／24 FPSでJPEG StillとH.264＋AAC MOVを実機検証
- [Done] Video開始時のPhotoOutput切離しとVideo→Still時の再接続
- [Done] UI姿勢をPreview／Photo／Movie connectionへ同期し、front preview鏡像と非鏡像保存を分離
- [Done] output再接続／format再適用後のrotation／mirror再設定
- [Done] `vite.config.ts`でTauri `devUrl`と開発serverを127.0.0.1:1420へ固定
- [Next] RAW photo format、HDR／LOG color spaceをformat単位で能力モデルへ追加
- [Next] iPhone／iPad実機でportrait／upside-down／front-camera mirrorを検証
- [Next] JPEG／HEIF／RAW選択、保存先選択、オリジナル＋処理済みasset管理
- [Next] video connectionのrotation／orientation、codec、container、bitrate、audio channelを能力値と設定へ接続
- [Next] window close、sleep、background、device切断時のsession終了／復旧をイベント駆動にする
- [Next] preview上のguide／scopeをnative overlayまたはMetal compositorへ移す
- [Later] `AVCaptureVideoDataOutput` → CVPixelBuffer → Metal textureでImaging Pipelineへzero-copy接続

## 実装境界

`camera-apple`はcontrol planeとmacOS native previewを担当する。AVFoundation objectをWebViewへ渡さず、Tauri IPCにもpixel dataを流さない。

```text
TypeScript UI
  ├─ get_camera_discovery
  ├─ request_camera_authorization
  ├─ get_camera_capabilities
  ├─ apply_camera_format
  ├─ get/request_microphone_authorization
  ├─ start_camera_preview
  ├─ resize_camera_preview
  ├─ set_camera_orientation
  ├─ capture_photo
  └─ start/stop_video_recording
             ↓ metadata / state only
Tauri AppState
  ├─ Arc<dyn CameraBackend>
  └─ PreviewRuntime
      ├─ Arc<AppleCaptureSession>
      └─ MacPreviewHost
             ↓
AppleCameraBackend
  ├─ authorizationStatusForMediaType
  ├─ AVCaptureDeviceDiscoverySession
  └─ AVCaptureSession
      ├─ AVCaptureVideoPreviewLayer → NSView
      ├─ AVCapturePhotoOutput → delegate → incomplete JPEG
      └─ AVCaptureMovieFileOutput + microphone input
           → recording delegate → incomplete MOV
              ↓ camera-core JPEG / ISO BMFF probe
           validated CapturedAsset → atomic finalize
```

`CameraController`はアプリ状態機械、`AppleCaptureSession`はAVFoundation lifecycleの正本である。start／stopはblocking workerで実行し、`operation_lock`で直列化する。NSViewとpreview layerのattach／resize／detachは`MainThreadMarker`を要求する。この条件を根拠にAVFoundation objectへ`Send + Sync`を付与しているため、新しいsession mutationを追加するときは必ず同じ直列化または専用serial queueへ載せること。

`CaptureOrientation`は0／90／180／270度だけを受理する。UIのScreen Orientationを3つのvideo connectionへ同一角度で適用し、front cameraはpreviewだけmirror、Photo／Movieは既定で非mirrorとする。録画中の変更はtrack途中の表示変化を避けるためnative側で拒否し、停止後にUIが最新姿勢を再同期する。

macOS previewは公開APIだけでWKWebViewの上にnative viewを重ねる。private APIでWebViewを透過させて背面合成する方式は採用しない。このため現段階ではpreview内部のHTML guide、histogram、audio meterを映像表示中だけ隠す。上部parameter stripと右／下tool railはWeb UIのまま操作できる。

## IPC contract

`get_camera_discovery`と`request_camera_authorization`は次を返す。

```json
{
  "authorization": "authorized",
  "devices": [
    { "id": "platform-stable-id", "label": "Camera Name", "position": "external" }
  ]
}
```

`authorization`は`not_determined | restricted | denied | authorized | unavailable`。未許可時のdevice配列は空で、拒否や制限を「deviceなし」と混同しない。Still modeはphoto output接続後だけシャッターを有効化する。Video modeはmicrophone許可後だけ録画ボタンを有効化し、疑似録画を実収録として見せない。

preview commandの追加contract:

```json
start_camera_preview({ "deviceId": "stable-id", "viewport": { "x": 16, "y": 98, "width": 968, "height": 646 } })
resize_camera_preview({ "viewport": { "x": 16, "y": 92, "width": 852, "height": 473 } })
stop_camera_preview()
```

`start_camera_preview`は`{ "running": true, "device_id": "stable-id" }`を返す。viewportはDOMのCSS pixel座標で、macOS側がwindow chrome差分とAppKitのflipped座標を吸収する。

2026-08-20以降、`start_camera_preview`にはAVFoundationが実際に選択したformatも含まれる。

```json
{
  "running": true,
  "device_id": "stable-id",
  "active_format": { "width": 1920, "height": 1080, "fps": 30.0 }
}
```

`get_camera_capabilities({ "deviceId": "stable-id" })`は全`AVCaptureDeviceFormat`を走査し、重複を除いたresolution／frame rate、manual ISO範囲、manual shutter／focus可否を返す。23.976、29.97、59.94のようなrateはUI選択肢では24、30、60へ正規化し、非標準rateはrange端点の丸め値も保持する。

能力値には独立したresolution／frame rate集合に加えて、実際に選べる組合せを`formats`として返す。

```json
{
  "formats": [
    { "width": 1280, "height": 720, "frame_rates": [15, 23, 24, 25, 30] }
  ]
}
```

`apply_camera_format({ "width": 1280, "height": 720, "fps": 24 })`はPreviewing状態でのみ受け付ける。Apple backendは該当する`AVCaptureDeviceFormat`を選び、23.976／29.97／59.94のようなdevice実値へ許容差内でclampしてmin／max frame durationを固定する。標準解像度では対応するsession presetを使い、その他は`AVCaptureSessionPresetInputPriority`が受理された場合だけ使用する。preset transactionをcommitしてからdevice `activeFormat`を最終適用し、録画中はformatを変更しない。

選択値は`Application Support/app.universalfilm.camera/settings/camera-format-v1.json`へstable device ID単位で保存する。保存は`.partial`へflush後renameし、壊れたJSONや未知schemaを推測復旧しない。session開始時の復元に失敗した場合はcamera defaultで継続し、`settings_warning`を返す。

`capture_photo()`はStill modeかつPreviewing状態でのみ受け付け、共通`CapturedAsset`を返す。以下は主要fieldの抜粋である。

```json
{
  "schema_version": 1,
  "id": "UFC-...",
  "media_type": "photo",
  "state": "finalized",
  "original": {
    "path": "/.../captures/UFC-....jpg",
    "container": "jpeg",
    "pixel_width": 1280,
    "pixel_height": 720,
    "orientation": 1,
    "color": { "embedded_profile": "srgb_exif" }
  },
  "validation": { "status": "passed", "checks": [] }
}
```

AVFoundationが返すJPEG dataはWebView IPCへ渡さず、Rust側で直接保存する。保存先は現段階ではapp data directory配下の`captures`であり、Photos libraryへ書かないためPhotos権限は不要。ユーザー指定folder、Media画面、exportは別工程とする。

動画commandは次の状態条件を持つ。

```json
get_microphone_authorization()
request_microphone_authorization()
start_video_recording()
stop_video_recording()
```

`start_video_recording`はVideo modeかつPreviewingかつmicrophone許可済みの場合だけ受け付け、`Previewing → Recording`へ遷移する。Video開始前にPhotoOutputを外して標準session presetとactive formatを再適用する。`stop_video_recording`は`Recording → Stopping`の後、AVFoundationの最終recording delegateを待ち、MOV probe合格後に`.incomplete/UFC-....mov`を`captures/UFC-....mov`へrenameして共通`CapturedAsset`を返す。

```json
{
  "schema_version": 1,
  "media_type": "video",
  "state": "finalized",
  "original": {
    "path": "/.../captures/UFC-....mov",
    "container": "quicktime",
    "video_codec": "h264",
    "audio_codec": "aac",
    "pixel_width": 1280,
    "pixel_height": 720,
    "frame_rate": { "numerator": 100000, "denominator": 4167 }
  }
}
```

停止commandを呼んだ時点ではcontainerの確定は保証されない。`didFinishRecording`相当のdelegate完了を受け取るまで完成assetを公開しない。停止またはfinalize失敗時はcamera stateを`Failed`へ移し、`Stopping`のまま残さない。

保存後probeのfield、failure policy、diagnosticは[`CAPTURED_ASSET_CONTRACT.md`](CAPTURED_ASSET_CONTRACT.md)を正本とする。

## 権限とローカライズ

- `apps/camera/src-tauri/Info.plist`: 英語fallback
- `apps/camera/src-tauri/infoplist/en.lproj/InfoPlist.strings`
- `apps/camera/src-tauri/infoplist/ja.lproj/InfoPlist.strings`
- `apps/camera/src-tauri/infoplist/zh-Hans.lproj/InfoPlist.strings`
- `apps/camera/src-tauri/Entitlements.plist`: camera／audio input
- `tauri.conf.json > bundle.resources`: 各`lproj` directoryをbundleの`Contents/Resources`直下へ配置
- `tauri.conf.json > bundle.macOS.entitlements`: 署名時にEntitlementを適用

camera権限は撮影画面内の明示操作で要求する。マイク権限はVideo modeを選択した時に別途要求し、拒否／制限時は録画ボタンを無効化する。写真ライブラリへ直接保存しない限りPhotos権限は追加しない。

## 能力値に関する注意

`capabilities()`のresolution、FPS、manual ISO／shutter／focusは実機値へ移行した。撮影UIのresolutionとFPSは対応最大値ではなく、session開始後のactive formatを表示する。未接続のLens、Iris、WBは推測値を出さず`—`または`AUTO`、手動非対応のEI／Shutterは`AUTO`かつdisabledとする。

RAW、LOG、HDRは単純なdevice全体booleanでは不十分で、photo output pixel format、device format、color space、同時output構成の組合せに依存する。現時点ではfalseのままにし、UIはSDRと表示する。resolution／FPSはformat単位へ移行済みであり、次工程では同じ能力型へRAW／LOG／HDRとcolor spaceを追加する。

## 検証方法

```bash
cargo test --workspace
npm run build
npm run tauri build -- --debug --bundles app
```

bundle検証では次を確認する。

```bash
plutil -p "target/debug/bundle/macos/Universal Film Camera.app/Contents/Info.plist"
find "target/debug/bundle/macos/Universal Film Camera.app/Contents/Resources" -name InfoPlist.strings
```

2026-08-16の実機検証:

- [Done] 内蔵cameraでpermission許可後に実映像を表示
- [Done] 1100 × 760のright rail layoutでparameter stripとpreviewが非重複
- [Done] 880 × 650のbottom rail layoutへresizeし、previewが追従
- [Done] preview frameをWebView IPCへ渡さず、native layer内に保持
- [Done] Still modeのシャッターからJPEGを1枚保存
- [Done] 実ファイルをJPEG／1920 × 1080／Exif／sRGBとして検証
- [Done] microphone許可後に中央録画ボタンで音声付きMOVを開始／停止
- [Done] 録画中のStill／Video切替を無効化し、停止後にPreviewingへ復帰
- [Done] 完成MOVがH.264 1920 × 1080 + AAC 48 kHz monoであることを検証
- [Done] 停止後に`.incomplete`の残存ファイルが0件であることを検証
- [Done] WebView非依存の2秒間隔容量monitorと重複しないAVFoundation停止要求を実装
- [Done] 内蔵cameraのactive formatを1920 × 1080／30 FPSとしてUIへ反映
- [Done] manual shutter／ISO非対応を`AUTO`かつdisabledとしてUIへ反映
- [Done] 対応組合せから1280 × 720／24 FPSを選び、native previewと上部表示へ適用
- [Next] 外部cameraのhot plug／切替
- [Next] OS再起動後を含む拒否→System Settings→復帰
- [Next] 英語／简体中文OSでpermission promptの実表示
- [Next] iPhone実機で容量低下、Home遷移、session interruption、復帰後Finalizeを検証

継続する実機／実アプリ検証項目:

1. 初回だけcamera permission promptが表示される
2. 許可後にcamera名が撮影画面へ表示される
3. 拒否後の再起動で再promptせず、System Settingsへの案内になる
4. USB cameraの着脱が再取得時に一覧へ反映される
5. 日本語／简体中文のOSでpermission文言が対応言語になる

2026-08-20のformat切替実機検証では、内蔵cameraの初期1920 × 1080／30 FPSから1280 × 720／24 FPSを選択した。`InputPriority`を受理しないmacOS session構成を検出したためdevice直結fallbackを追加し、適用後の`activeFormat`、FPS parameter、format summaryがすべて1280 × 720／24 FPSへ同期することを確認した。Computer Useによる画面検証で初回実装の拒否エラーを捕捉し、このplatform差を実装とADRへ反映した。

2026-08-24の保存後validationでは、上記UI／active formatが720p／24 FPSでも、PhotoOutputとMovieFileOutputの同時接続中はMOVが1920 × 1080／約30 FPSへ戻ることを検出した。途中の明示output settingsでは720 × 720への再交渉も検出した。いずれもprobeが`pixel_dimensions`不一致として完成公開を拒否した。

Video開始前にPhotoOutputを外し、`AVCaptureSessionPreset1280x720`をcommit後にdevice active formatを最終適用する構成へ変更した。再検証MOVはH.264 1280 × 720、平均`100000/4167`（約23.998 FPS）、BT.709、AAC 16 kHz monoで合格した。アプリ再起動後の1280 × 720／24 FPS復元と、Video後にPhotoOutputを戻したJPEG 1280 × 720も合格した。audio sample rateが以前の48 kHzから16 kHzへ変わることを確認しており、audio formatを固定能力値として扱ってはいけない。

2026-08-18のスチル実機検証では132,810 bytesのJPEGを保存し、`file`と`sips`で1920 × 1080、Exif container、sRGB IEC61966-2.1を確認した。通常landscapeのためorientation tagはnilであり、回転角の明示設定とportrait検証は未完了。テスト画像の内容はdocsへ収録しない。

2026-08-20の動画実機検証では24,189,059 bytes、5.671秒のQuickTime MOVを保存した。`ffprobe`でvideo trackはH.264 1920 × 1080、audio trackはAAC 48 kHz monoを確認した。検証assetは`~/Library/Application Support/app.universalfilm.camera/captures/UFC-1787202319-142623000.mov`に残している。映像内容はdocsへ収録しない。

debug bundleはローカルlinker署名のままだとInfo.plist、Bundle ID、EntitlementがTCC identityへ結び付かない場合がある。検証時は生成した`.app`へ`Entitlements.plist`付きでadhoc再署名する。camera／microphoneの初回許可検証ではbundle内実行ファイルを親processから直接起動しない。macOS TCCが親processをresponsible applicationとして帰属させる場合があるため、`open -n ".../Universal Film Camera.app"`などLaunchServices経由で独立起動する。配布buildではDeveloper ID／App Store署名とnotarizationを正式に設定する。

## 参照

- [Apple: AVCaptureDevice](https://developer.apple.com/documentation/avfoundation/avcapturedevice)
- [Apple: AVCaptureDevice.DiscoverySession](https://developer.apple.com/documentation/avfoundation/avcapturedevice/discoverysession)
- [Tauri 2: macOS application bundle / Info.plist localization](https://v2.tauri.app/distribute/macos-application-bundle/)
- [objc2-av-foundation](https://docs.rs/objc2-av-foundation/0.3.2/objc2_av_foundation/)
