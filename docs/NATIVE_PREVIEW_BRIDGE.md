# Native preview bridge — SDR LUT presentation

更新日: 2026-11-02

## 🟢 [Done] 接続済みの範囲

`AVCaptureVideoDataOutput`のdelegateが`CVPixelBuffer`をretainし、Rustのcapacity-one mailboxへ格納する。新しいframeで未取得frameを置き換え、待ち行列を増やさない。`take_latest()`で取り出したArc leaseは置換・mailbox clear後も有効。停止時にmailboxをclearする。GPU側consumerが処理中に保持するleaseは別であり、clearで強制解放しない。

`MetalFrameImporter`はrendererが指定したMetal deviceで`CVMetalTextureCache`を作る。BGRAはBGRA8Unorm、8-bit NV12はR8Unorm＋RG8Unormの2平面としてimportする。video rangeとfull rangeを区別するflagを保持する。HDR／10-bit／未対応形式はエラーにし、無断で8-bitとして解釈しない。実カメラのpixel format選択は今回変更していない。

`NativeTextureImporter`はwgpuのHALから取得した同一Metal deviceを使用する。CoreVideo textureのMTLTextureをHAL経由でwgpu textureへ登録するだけで、pixel bufferのCPU lock・RGB変換・GPU upload・JSへの転送は行わない。テストのreadbackは画素比較専用であり、production import APIにはreadback処理がない。

## 寿命と同期の契約

- source CVPixelBufferとCVMetalTexture wrapperを両方保持する。MTLTextureだけのretainを画像pool再利用の防止として扱わない。
- `NativeTextureImporter::import`はunsafe API。返された`ImportedNativeFrame`を、全GPU処理の完了まで生存させる。encoder作成完了／queue.submitだけでは解放不可。cloneしたviewからlease解放後にsubmitしてはいけない。
- imported planesはread-onlyとして使い、camera bufferへ書き込まない。iOS consumerは1frameずつsubmit／completion待ちとし、leaseをqueue completion callbackへ渡す。
- Metal cacheはrenderer所有threadで使用する。cacheにunsafe Send／Syncは追加していない。source frameのSend／Syncは、保持中のAVCapture画像をconsumerが変更しないという契約による。
- iOSのworkerを接続済み。描画・present要求後のmain-thread通知が成功するまで既存preview／canvasを維持する。実機表示受け入れは未実施。

## 🟢 [Done] 通常プレビューの不要CPU処理

通常画面は既存のAVCaptureVideoPreviewLayerが映像を表示する。しかし従来はWebView側の効果表示がOFFでも360pxのRGB画像を毎frame生成していた。

`includePreview=true`のpollが次のframeのCPU sampleを要求する。要求はcoalesceし、1要求で1frameだけ生成する。RGB sampleは次のpollまで保持し、間のframeで空配列に上書きしない。`includePreview=false`は要求を解除して保存RGBを解放する。session停止でも解放する。histogramとaudioの小さいtelemetryは維持する。

SDR LUT／ゼブラ／ピーキングのnative表示中はRGB IPCを使用しない。look OFFの表示補助単独も対象。Display P3／対応外input／起動待ち／GPU失敗時のWebView fallbackは引き続きRGB IPCを使用する。全モードのIPC廃止を完了したわけではない。

## 診断

`get_native_preview_status`は`frames_received`、`frames_replaced`、`cpu_rgb_generated`、`gpu_presented`、`gpu_ready`、optional `gpu_error`を返す。画像やpointerは返さない。clean／native LUT previewでRGB生成数が増えず、native LUT時はpresent数が進むことを実機確認する。`gpu_presented`はGPU完了後のpresent要求数で、実ディスプレイのscanout測定ではない。複数WebView間の表示補助demand統合は未実装（現在は単一capture window）。

`frames_replaced`は未取得mailbox frameの置換数で、GPU／画面のdrop率ではない。native worker無効時やGPU処理が遅い時に増える。

## 検証

- 合成BGRAのnative buffer→CoreVideo Metal texture→wgpu→GPU copy readbackが全16画素一致。
- NV12 video／full rangeが2平面・R8／RG8・正しい平面寸法でimportされる。
- lease解放後にsource Arcが解放される。GPUテストではqueue完了まで保持する。
- mailboxに100frameを投入し、1frameのみ取り出し可能・99置換・取り出し済みleaseはclear後も有効。
- delegateへ100frameを投入してCPU RGB生成0件、要求後1件、さらに20frameで追加生成0件、次要求で2件。間のframeでも生成済みRGBが保持され、histogramも継続する。

合成テストの結果を実カメラのFPS／発熱改善量へ換算しない。

2026-11-02検証結果: `npm run build`、`cargo test --workspace --offline`、`./scripts/check_ios_rust.sh`に合格。Metal GPUの入力testも含めて成功した。`cargo clippy -p imaging-gpu --no-deps --lib --offline -- -D warnings`は合格。依存workspaceもlintするstrict Clippyは既存camera-coreの5件のstyle lintで停止するため、その経路のcleanを主張しない。GPU testとSwift cache書き込みが必要なiOS checkは承認済みsandbox外実行。一度の権限レビューtimeoutは許可された再試行で解消した。

## 🟢 [Done] SDR LUTのGPU描画・表示接続

`apps/camera/src-tauri/src/native_preview.rs`がiOS専用workerを所有する。写真rendererと同じwgpu device／queueを共有し、LUT選択変更時にRust側のtableを読み、preview専用3D textureをkeyでcacheする。camera pixelsはimportしたtextureを直接sampleし、CPU readbackを行わない。worker未要求時はGPU初期化を遅延する。

`preview.wgsl`はBGRAとNV12を処理する。CoreVideoの709 primaries、601／709 YUV matrix、709／sRGB transfer、center／left chromaを検証し、不明値はfallbackする。NV12 video/full rangeとchroma位置を補正し、709 transferをlinear経由でsRGBへ変換してLUTを適用する。CAMetalLayerはsRGB color spaceを指定したBGRA8Unorm。Rec.709／sRGB monitoring時にこのSDR表示経路を使い、Display P3はfallbackする。任意のoutput profile／HDRを実装したものではない。

layerは既存のSafe Area内hostに重ね、bounds・scaleを既存resizeに同期する。orientation／mirrorはAVCapture monitor connectionが適用済みなので、shaderは二重変換せず中央aspect-fillだけ行う。geometry変更のimplicit animationを無効化した。回転時はmailboxをclearし、render中にorientationが変わればpresentを捨てる。レンズ切替は旧workerをjoinしてからhostをdetachする。

1frameずつGPUへsubmitし、leaseのreleaseはqueue completion callbackで行う。GPU待ちは2秒で失敗扱いとし、terminal errorはworkerを停止してfallbackする。pending leaseはcompletionまで保持する。iOS workerの再起動はpreview再開始時。import未対応・色metadata不明はframe単位でfallbackし、後の対応frameで回復可能。LUT読込失敗は同じrevisionで繰り返し読み込まず、revision変更後に再試行する。

native present要求後にmain threadでlayerを表示し、`native_lut_presented`でJSに通知する。JSは通知前にcanvasを隠さない。native ready時はbackendが`includePreview`を抑止し、RGB生成／base64 IPCを止める。Document revision／ゼブラ／ピーキング要求とpresent済み設定が一致しない間もfallbackを維持する。旧sessionのmonitor応答をJS generationで破棄する。

background時はJSがnative要求を解除する。要求が途絶えた場合も1秒で描画を休止する。UI visibility通知は集約し、worker終了後のqueued showを拒否する。初期化失敗・panic・GPU検証失敗ではreadyを落とし、既存のAVFoundation／WebView表示を維持する。

## 検証（表示接続の追加）

実Metal GPUでidentity／赤反転LUT、CPU LUT参照との一致、中央aspect-fill、NV12 full/video range、601／709 matrix、709中間調transferを検証。native CVPixelBuffer→Metal→wgpu→LUT描画→test-only readbackのBGRA fixtureも全画素一致。metadata欠落・HLG・P3拒否のテストを追加した。

UIはBrowser skill未提供のためPlaywright＋Chromeを使用（Browser plugin not available）。URL `http://127.0.0.1:1420/`、390×844／844×390。native応答模擬で「初回fallback→native readyでcanvasを隠す→peakingでfallback→peaking解除でnative→GPU失敗通知でcanvas復帰→look OFF」を検証した。ページidentity・非空・framework overlayなし・page errorなし。consoleは既存favicon 404のみ。証跡は`/private/tmp/camera-native-switch-mobile.png`と`/private/tmp/camera-native-switch-landscape.png`。これは物理CAMetalLayerの映像表示試験ではない。

追加でzebra切替、停止後に遅れて届く旧sessionのready応答の破棄、preview再開始を模擬検証した。Web build・workspace全体test・iOS arm64 compile・imaging-gpuのno-deps strict Clippyは合格。既存manual benchmark 1件はignoredのまま。実機へのインストール／CAMetalLayer実表示／速度計測は未実施。

## 🟢 [Done] nativeゼブラ（2026-11-02第1段階、下記ピーキング統合で対象を拡張）

`get_camera_monitor_snapshot`のoptional `nativeZebra`は表示要求のみを渡し、画素は渡さない。既存clientで省略した場合はfalse。Rust workerがAtomicBoolをframeごとに読み、LUTと同じfragment passで合成する。GPU完了後にpresent要求済みのゼブラ設定も保持し、最新要求と一致しない間は`native_lut_presented=false`としてfallback用RGB取得を維持する。ゼブラOFF／ONをLUT textureの再upload条件にはしない。

警告はLUT適用前のsRGB-encoded luminance（係数0.2126／0.7152／0.0722、閾値0.9216）で判定する。これはセンサーRAW飽和警告ではない。709入力は従来通りsRGBへ変換後に判定。映像と同一UV・aspect-fillを使い、警告だけ別の伸縮・回転を行わない。native render targetの12px周期で白／暗色の斜線を0.745でLUT後の画像へ合成、最終alphaは1。WebGL fallbackと周期の物理サイズは同一を保証しない。look ONかつpeaking OFFのSDR範囲に限定し、look OFFのゼブラ単独は既存fallbackのまま。

native ready中のJS pollは10Hz（状態・source histogram・audioのみ）、fallback処理中は従来の最大約30Hz。GPU描画はcamera-drivenであり、10Hzへ制限しない。ヒストグラムは依然source、LUT後scopeではない。

検証: 実Metalでpre-LUT閾値上下、黒LUTでも残る警告、暗部への誤警告なし、両stripe phase、中央cropの画素一致を確認。`cargo test -p imaging-gpu --offline`（5件）、Web build、iOS arm64 compile、imaging-gpu strict Clippyに合格。Browser skill未提供（Browser plugin not available）のためPlaywright＋既存Chromeを使用。ローカル`http://127.0.0.1:1420/`、390×844／844×390で実UIを操作し、LUT＋ゼブラのnative要求、ready時canvas非表示、GPU失敗時fallback、復帰、peaking併用時fallback、ゼブラ解除を応答模擬で確認。page identity／非空／framework overlayなし／page errorなし。mockでnative layerは存在しないため撮影面は空白であり、実機映像の表示証拠にはしない。証跡: `/private/tmp/camera-zebra-390.png`、`/private/tmp/camera-zebra-844.png`、一時test script `/private/tmp/camera-zebra-qa.cjs`。端末へのinstallは今回未実施。

## 🟢 [Done] nativeピーキング・look OFFの表示補助（2026-11-02第2段階）

optional `nativeFocus`は既存clientで省略時false。UIからは効果のboolのみを送り、色はRust所有Documentの`peaking_color`を使用する。`PeakingColor`の6色は既存WebViewと同じRGB値（cyan 0/225/255、red 255/48/48、green 70/255/105、yellow 255/224/32、magenta 255/64/224、white 255/255/255）。未定義色は推測せずエラーとする。

shaderは同じaspect-fillのUVを中心にsource texel間隔で8近傍をsampleし、pre-LUTのsRGB輝度（54/183/19）へSobelを適用、gradient長180超を色で上書きする。BGRAとNV12とも共通の色変換を通す。ゼブラの後にピーキングを合成し、重なる場所は輪郭色を優先する。textureはclamp-to-edgeでborder wrapを防止。旧360px fallbackとはsample解像度が違うため、線の太さ／感度の見た目の一致は保証しない。ピントの合焦保証ではなくcontrast edge補助である。iPhoneの実写でノイズ・細線の見え方と負荷を評価する必要がある。

look OFFでもピーキング／ゼブラがあればnative対象とする。workerはidentity LUTを保持し、shaderの`apply_lut=false`でLUT sampleをbypassする。無効な選択LUTの読込失敗でlook OFFの補助を妨げない。全効果OFFは従来AVCaptureVideoPreviewLayer。Display P3／未対応入力のfallbackは維持。`native_lut_presented`は互換名称のままだが表示補助のみのnative描画も含む。

GPU完了・present要求時にDocument revisionとfocus／zebraを記録し、command応答側で現在要求と照合する。色・look・LUT変更直後は旧revisionをready扱いせず、RGB fallbackを維持してから切り替える。複数windowのrequest arbitrationは別工程。

検証: 実Metalの6色、縦横step edge＋中央crop、BGRAとNV12の輪郭一致、平坦入力／borderで誤警告なし、黒LUT下の輪郭、look OFFのLUT bypassを確認。GPU unit testは6件成功。workspace全体test成功（既存manual benchmark 1件ignored）、Web build、iOS arm64 compile、GPU no-deps strict Clippy成功。Browser skill未提供（Browser plugin not available）のためPlaywright＋既存Chromeを使用し、`http://127.0.0.1:1420/`、390×844／844×390でnative応答を模擬する。撮影→補助menu→ゼブラ＋ピーキング→GPU失敗／復帰→環境設定でmagenta・look OFF→撮影復帰→補助解除を操作。実機映像／GPU FPSの証拠ではない。証跡は`/private/tmp/camera-peaking-390.png`、`/private/tmp/camera-peaking-844.png`。

## 🟠 [Next] 残るGPU移行

Display P3 output、LUT適用後histogram、HDR／10-bit対応。現在のhistogramは従来のsource telemetryで、native LUT後のGPU scopeではない。複数windowのcapture ownershipとrequest arbitrationも別途必要。動画保存へのLUT焼き込みはこのpreview接続とは別工程。

⭕️ [Pending] iPhone実機のFPS、遅延、発熱、レンズ切替、縦横回転、background復帰、効果ON/OFFの連続切替。今回端末へのインストールはしていない。
