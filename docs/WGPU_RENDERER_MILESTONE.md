# Rust／wgpu写真LUTレンダラー

更新日: 2026-10-15

## 今回の優先順位

写真LUTの既存保存経路を、全ウインドウで共有するRust所有のwgpu rendererへ接続した。
写真の待ち時間を計測でき、実画像・EXIF・CPU参照で開発環境内の検証ができるため、実機が必要なライブ映像のimportより先に着手した。
既存のCore Image経路は比較テストにだけ残す。

- 🟢 [Done] `crates/imaging-gpu`：wgpu 27.0.1／Metal、SDR sRGB LUT compute shader、CPU参照、conformance example。
- 🟢 [Done] `AppState.renderer: Arc<PhotoRenderer>`：Rust側の一つのownerがdevice／queue／pipeline／LUT／作業textureを管理。
- 🟢 [Done] LUT付きStillのdecode→wgpu→JPEG→既存probe／manifest finalize。LUTなしの撮影は従来経路。
- 🟢 [Done] red-fastest、clamp、有効なdomain、premultiplied alphaを定義。最大65³のfloat 3D textureを手動trilinear補間。float32で正規化できないdomainは、GPU上で黙って別の値にすることを避けるため拒否する。
- 🟢 [Done] 単一LUT cacheと単一作業image set。入力範囲・sampleが変わるとcontent hashで更新する。
- 🟢 [Done] JPEG品質0.95、sRGB、EXIF orientation保持、LUT renderer metadataを`wgpu-cube-v1`へ更新。
- 🟠 [Next] camera CVPixelBufferのwgpu native texture import、native surface、ライブLUT／peaking／zebra。
- 🟠 [Next] 撮影／monitoring設定全体をRust Document／Stateへ移す。既存WebViewのlocal preferenceまで移行済みとはしない。
- 🟠 [Next] 動画encoderへ処理textureを接続し、音声・timestampと同時に検証する。
- ⭕️ [Pending] iPhoneの48MPメモリ・発熱・連写・色・向き・OS suspensionの受け入れ。
- 🔴 [Later] desktop docking／floating panel、非iOS backendの追加開発、科学Film Pipeline全体のGPU化。

## 境界と所有権

```text
Tauri WebView（撮影command、状態表示）
  → Rust AppState（アプリ全体で一つ）
    → PhotoRenderer（decodeからencodeまで直列化）
      → ImageIO / Core Graphics：sRGB RGBA8 decode
      → imaging-gpu：tile upload → float LUT → finished tile readback
      → ImageIO：EXIFを保持したJPEG encode
    → CapturedAsset probe / manifest / Photos export
```

大容量画素はRust／GPU内に保持し、今回の経路からJavaScriptへは渡さない。
JPEG encoderへ渡す最終画素のreadbackは必要な処理として残す。
これは写真保存のGPU処理基盤であり、ライブ映像のzero-copyや動画LUT保存の完了を意味しない。
既存ライブpreviewにはRGB IPC／WebGL fallbackが残り、次工程で置換する。

GPUは最初に必要になるまで初期化せず、待機中のrender loop／timerは持たない。
同時要求はdecodeからencodeまで共有mutexで直列化し、複数の全解像度CPU画像が待機することを避ける。
撮影画面を閉じるcommandではworker上で進行中の処理を待ち、作業image setを解放する。
device／pipeline／直近LUTは次の撮影のために残す。OS suspensionでこの処理が完了することは実機確認待ち。
撮影画面を閉じた後にdecodeへ到達する遅延ジョブも、保存を完了してから作業imageを解放する。画面再開で再利用を有効に戻す。

## メモリと再利用

- 元画像は最大5000万画素／各辺16384pxまで。寸法・buffer length・GPU limitsを検証してから使用。
- GPU処理は最大2048×1024画素ずつ。LUTは近傍依存を持たないのでtileの重なりは不要。
- 最終tileが小さくても大きい作業textureを再利用する。転送extent・padded readback row・返却行数は実寸を使用。
- input／output／padded readbackの明示保持分は最大約24MiB、65³ float LUTは約4.2MiB。driver内部やCPU decode／encodeのメモリはこの値に含めない。
- CPU側の全解像度RGBAは一つのVecを加工し、全寸法のGPU textureや二つ目の完成RGBA Vecを保持しない。
- map／GPU waitはblocking worker内だけ。timeout／mapping errorでは作業image setを捨て、既存の未加工capture保持経路へ失敗を返す。
- GPUが使えない際に別の色処理へ黙ってfallbackしない。LUT付き撮影は明示的に失敗し、未加工captureをincompleteに残す。

## 診断

`get_renderer_status`は小さな状態だけを返す。呼び出してもGPUを起動しない。
初期化前はnull、処理中のtry-lock失敗はbusy error、処理後は以下を取得できる。

- `adapter`／`backend`
- `rendered_tiles`（写真枚数ではなくdispatch数）
- `lut_uploads`／`lut_cache_hits`
- `image_allocations`（input・output・readback一組の作成回数）
- `retained_bytes`（明示所有resourceの見積もり）
- `last_render_ms`（直近tileのCPU wall time。upload／GPU待ち／readbackを含み、GPU timestampではない）

metadataのLUT hashは従来通りpayloadの再現情報を保存する。GPU cache用hashとは用途を分ける。
既存manifestの`core-image-cube-v1`は書き換えず、新規撮影だけ新rendererを記録する。

## 再現コマンド

```sh
cargo test --workspace --offline
cargo run -p imaging-gpu --example lut_conformance --offline
cargo test -p universal-film-camera benchmark_photo_look --offline -- --ignored --nocapture
cargo clippy -p imaging-gpu --all-targets --offline -- -D warnings
./scripts/check_ios_rust.sh
```

macOSの通常のMetalアクセスが必要。制限された環境でadapterを初期化できない場合、実GPU試験を成功扱いにしない。
wgpu 27のMSRVに合わせworkspaceの宣言をRust 1.88へ更新した。今回のhost compilerはHomebrew Rust 1.99、iOS確認にはiOS targetを持つrustup stableを使用する。
wgpu 27の間接依存`block 0.1.6`にはRust 1.99のfuture-incompatibility warningが出る。現時点のbuild失敗ではないが、wgpu更新時の確認事項とする。

このホストはHomebrewとrustupが併存する。iOS確認では`RUSTC`／`RUSTDOC`をrustup stable配下の絶対pathへ指定してからcargoを実行する。`rustup run stable cargo`だけではHomebrew rustcが選ばれ、インストール済みのiOS coreを見つけられないケースを確認した。
`scripts/check_ios_rust.sh`はこの指定を自動化する。Xcode 27の標準SwiftPM `swiftbuild`とswift-rs 1.0.7の組合せではmacOS SDKを混在させて失敗したため、専用PATH内の`ios-swift-compat/swift`が`swift build`だけへ`--build-system native`を追加する。SDKやCargo registryのソースは変更しない。互換指定はこの検証コマンドの子プロセスだけに適用される。

## 検証・測定結果

Apple M4／Metalでの実GPU conformanceは5ケースすべて合格。恒等LUTとalphaは完全一致、非線形／swizzle／非対称domainの最大誤差は1/255以内。
奇数幅、65³最大cube、resize後の小画像とpadded buffer再利用、無効bufferの拒否も確認した。
10 dispatchでLUT upload 5／cache hit 5／image allocation 2、作業image解放後の明示保持は176 byte（2³ cubeとuniform）だった。

Rust workspace全111 test合格、手動benchmark 1件は通常testではignored。写真保存のtestにはJPEGの画素・sRGB profile・EXIF orientation 6・上書き拒否・未加工入力非変更・遅延完了時のcache解放を含む。
2051×1031のidentity tile試験は約211万画素すべて完全一致、4 tileでimage allocation 1／LUT upload 1／cache hit 3だった。
Web production buildと`imaging-gpu`の全target Clippy（`-D warnings`）も合格。
iOS arm64向けアプリ全体のcargo checkは、上記互換スクリプトを使いRust 1.93／Xcode 27で合格した。署名済みIPA再作成・端末install・実撮影は今回実施していない。
app全体の厳格Clippyは既存`camera-core`のstyle lint 5件（`obfuscated_if_else`／`collapsible_if`／`manual_contains`）で停止した。今回のGPU変更とは別の既存課題として残し、合格扱いにしない。

### 写真保存の交互A/B計測

2026-10-15、Apple M4、macOS 27.0.1、Rust 1.99 debug test。同時コンパイルを止めた後、各24回を交互に計測した。

| 方式 | 初回／process | warm中央値 | p95 | p99／最大 |
|---|---:|---:|---:|---:|
| 従来Core Image | 393.830ms | 245.872ms | 448.768ms | 508.056ms |
| Rust／wgpu | 201.658ms | 193.145ms | 300.631ms | 416.026ms |

この条件で中央値約21%、p95約33%短縮。初回は各process内の初回呼び出しであり、OS／driverのshader cacheを消した初回install条件ではない。
CPU decode・encode・I/Oも含むため、この差をGPU kernel単体の速度差とは解釈しない。

最初の基準測定（1254×1254、23 warm sample）はCore Image中央値949.616ms、p95 1346.850msだったが、依存の再コンパイルと重なっていたため最終比較の根拠には使わない。

benchmarkは同じPNGとLUTを使い、decode／処理／JPEG encode／file writeまでのwall timeを測る。
各方式24回のうち初回をcold、残り23回をwarmとし、実行順序を交互に変える。
p95／p99は小標本の観測値であり、とくにp99は実質最大値になる。iPhone・12MP／48MP・release buildへの外挿はしない。
