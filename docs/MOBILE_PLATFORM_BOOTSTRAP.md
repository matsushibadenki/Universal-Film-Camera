# Mobile Platform Bootstrap

更新日: 2026-08-31
対象: Tauri 2 / iOS / Android

## Status

- [Done] `src-tauri/gen/apple`のXcode projectを生成しrepository管理対象へ追加
- [Done] `src-tauri/gen/android`のAndroid Studio projectを生成しrepository管理対象へ追加
- [Done] iOS camera／microphone permission文言を英語、日本語、简体中文でresource接続
- [Done] Android `CAMERA`／`RECORD_AUDIO` permissionとcamera featureを宣言
- [Done] iOS arm64 simulator debug bundleを署名なしでbuild
- [Done] Android arm64 debug APKをJava 21でbuild
- [Done] iOSのnative preview hostをUIKit／WKWebView lifecycleへ接続
- [Done] iOSでPreview／Still／Video Tauri commandと共通asset finalize契約をcompile
- [Done] Development Teamを設定し、署名済みarm64実機IPAをbuild、接続iPhoneへinstall／launch
- [Next] iPhone実機でpermission、preview、Still、Video、orientationを検証
- [Done] Android CameraX adapterのpermission、device discovery、capability、native previewを実装
- [Done] Android CameraX ImageCapture／VideoCaptureを共通CapturedAsset契約へ接続しarm64 APKをbuild
- [Later] Android実機でStill／音声付きVideo、回転、background中断、保存metadataを検証（iOS優先方針のため保留）
- [Done] Android format選択をfallback禁止ResolutionSelector／Camera2 FPS requestへ接続してAPK build
- [Later] Android実機でformat別のPreview＋Still＋Video同時bind可否を検証（iOS優先方針のため保留）
- [Done] ADB install／診断snapshot scriptとAndroid実機受け入れmatrixを整備
- [Later] 実機permission、orientation、front-camera mirror、background復帰を検証

## Generated project policy

`apps/camera/src-tauri/gen`はmobile platform source、manifest、project設定を含むためrepository管理対象とする。各platformの`build`、`.gradle`、`jniLibs` symlink、local SDK pathなど、再生成可能またはmachine固有のartifactは配下の`.gitignore`で除外する。

Tauri CLIによる再初期化はplatform manifestと`project.yml`の手修正を上書きし得る。再実行後は必ず次を確認する。

- iOS `project.yml`に`AVFoundation.framework`と`CoreMedia.framework`がある
- iOS targetが`../../infoplist`をresourceとして含む
- iOS Info.plistにcamera／microphone usage descriptionがある
- AndroidManifestに`CAMERA`、`RECORD_AUDIO`、`android.hardware.camera.any`がある

## Build commands

### iOS simulator

```sh
npm exec tauri -- ios build --debug --target aarch64-sim --no-sign --ci
```

検証済み出力:

```text
apps/camera/src-tauri/gen/apple/build/arm64-sim/Universal Film Camera.app
```

Xcode targetへ`AVFoundation.framework`と`CoreMedia.framework`を明示的にlinkする。実機buildのDevelopment TeamはTauri設定の`bundle.iOS.developmentTeam`へ設定する。Apple ID、証明書ID、provisioning profile UUIDはrepositoryへ保存しない。

### iOS device

```sh
./scripts/build_ios_device.sh
xcrun devicectl list devices
./scripts/install_ios_device.sh <device-udid>
```

検証済み出力:

```text
apps/camera/src-tauri/gen/apple/build/arm64/Universal Film Camera.ipa
```

`build_ios_device.sh`は生成済みXcode productだけをcleanし、debugging export用の署名済みarm64 IPAを作る。`install_ios_device.sh`はUDIDを引数で受け取り、IPAを一時directoryへ展開してinstall／launchする。UDIDやApple IDをsourceへ固定しない。

### Android arm64 debug APK

```sh
JAVA_HOME=/path/to/jdk-21 npm exec tauri -- android build --debug --target aarch64 --apk --ci
```

この開発hostでは`/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home`を使用した。Android Studio同梱Java 25ではGradle 8.14.3が`Unsupported class file major version 69`で停止したため、JDK 21をbuild前提とする。ただし絶対pathはmachine固有なのでproject fileへ埋め込まない。

検証済み出力:

```text
apps/camera/src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

## Runtime boundary

mobile scaffoldとRust／WebView applicationのbuildは完了しているが、mobile camera runtimeは未完成である。

- iOS: `camera-apple`のAVFoundation discovery、native preview、Still、Video codeをcompile／link済み。`UIView` hostがTauriのWKWebViewへ`AVCaptureVideoPreviewLayer`をattachし、main threadでresize／detachする。保存はmacOSと同じIncomplete → probe → manifest → Finalized境界を使う。Simulatorはcamera入力を受け入れ条件にできないため、次は署名Teamを明示したiPhone実機検証とする。
- Android: Tauri mobile pluginとしてCameraX 1.4.2を組み込み、CAMERA／RECORD_AUDIO permission、front／back discovery、Camera2 stream size／AE FPS capability、`PreviewView`、`ImageCapture`、`VideoCapture<Recorder>`をRust commandへ接続した。StillはJPEG、Videoは音声付きMP4を`.incomplete`へ出力する。CameraX callback完了後にRustがin-process probe、CapturedAsset validation、atomic rename、manifest永続化を行う。Activity pause／destroyでは録画とproviderを閉じviewを除去する。選択formatはfallback禁止の解像度とCamera2 FPS requestとして実装済みであり、端末別runtime受け入れを実機で確定する。
- 共通: `CapturedAsset`、atomic manifest、Media indexはapp data directoryを使うためmobile buildへ含まれる。platform camera adapterも同じIncomplete → probe → Finalized境界を守る。

## iOS Simulator build and launch

Tauri CLI 2.11.xは古い`build/arm64-sim/*.app`が残っているとarchive後のrenameを`Directory not empty`で停止する。Xcode cleanに加え、このTauri生成済みSimulator `.app`一件だけを削除してからbuildするwrapperを正規手順とする。撮影media、app data、sourceは削除対象に含まれない。

```sh
./scripts/build_ios_simulator.sh
xcrun simctl install <simulator-udid> "apps/camera/src-tauri/gen/apple/build/arm64-sim/Universal Film Camera.app"
xcrun simctl launch <simulator-udid> app.universalfilm.camera
```

2026-08-31にiPhone 16e Simulatorへinstall／launchし、portrait狭幅でstatus bar、parameter strip、preview、scope、bottom rail、中央正円capture controlがSafe Area内に収まることを確認した。Simulatorには実camera入力がないため「カメラ信号なし」とpermission導線が表示される状態が正しい。

同SimulatorでHome操作によるbackground→iconからforeground復帰、portrait→landscape→portrait upside-down、landscape／portraitのMedia empty画面を確認した。最初のlandscapeではframe guideが透明なtimecodeへ重なったため、狭高さlandscapeだけtimecodeへ半透明背景を追加した。iPhone側の`UISupportedInterfaceOrientations`へPortraitUpsideDownも追加し、プロ撮影時の上下反転保持をOSへ明示した。

## Acceptance evidence

2026-08-31時点:

```text
iOS arm64 simulator debug bundle: passed
iPhone 16e Simulator install / launch / narrow portrait layout: passed
iOS arm64 signed device IPA: passed (TeamIdentifier 3WH28SSRZC)
Connected iPhone install / process launch: passed
iOS native Preview / Still / Video code: compiled and linked
Android aarch64 debug APK: passed with JDK 21
Android CameraX permission / discovery / capability / native Preview code: compiled
Android CameraX ImageCapture / VideoCapture / CapturedAsset bridge: compiled
Android strict resolution / FPS negotiation bridge: compiled
Android conformance harness: shell syntax passed; no authorized device connected

実機試験の正本と採取手順は[`ANDROID_CAMERA_CONFORMANCE.md`](ANDROID_CAMERA_CONFORMANCE.md)を参照する。
Camera / microphone permission declarations: present
iOS localized permission resources: en / ja / zh-Hans present
Mobile real-device camera runtime: installed and launched; permission / capture acceptance remains
```
