# Architecture Decision Log

## ADR-001: Tauri is the control plane, not the frame transport

Status: Accepted (2026-08-11)

Tauri 2はwindow、lifecycle、IPC、設定UIを担当する。連続するcamera frameをWebView IPCへ流さない。理由は、元仕様のzero-copy要件、4K60目標、メモリ帯域、GCとserialization遅延を守るため。

## ADR-002: Shared contracts precede platform backends

Status: Accepted (2026-08-11)

`media-core`、`camera-core`、`imaging-core`、`film-core`をOS実装より先に置く。各backendは共通のdevice、capability、session、state contractを満たす。OS固有機能はcapabilityとして公開し、共通APIの偽装値にしない。

## ADR-003: Still and video share a session but have separate capture operations

Status: Accepted (2026-08-11)

previewとdevice ownershipは共通化し、photo captureとrecording lifecycleは分ける。録画中のmode変更など不正な操作は`CameraController`で拒否する。

## ADR-004: ACEScg is the initial working space

Status: Accepted for MVP, Revisitable (2026-08-11)

共有Film処理の初期working spaceをscene-linear ACEScgとする。API enumは他のworking spaceを保持し、Engine自体をACES固定にはしない。入力変換でrange、primaries、transfer、matrix metadataを明示する。

## ADR-005: Apple is the first reference backend

Status: Accepted (2026-08-16)

現在の開発ホストがmacOSで、AVFoundationはpreview、photo、movie、audioを1つのcapture sessionで検証できるため。Apple縦切りの後にAndroidを実装し、desktop固有backendへ展開する。

## ADR-006: Film Engine is a specialized renderer inside the Imaging Pipeline

Status: Accepted (2026-08-11)

製品全体のモデルをFilmだけに限定せず、Camera Body/Exposure、Lens、Film/Digital Sensor、Chemical/RAW Development、Print/DI、Output Transform、Displayを記述する`imaging-core`を上位に置く。`film-core`はFilm Capture Medium、Chemical Development、Photochemical Printを実行する専門engineとして維持する。

## ADR-007: Pipeline edges are validated by signal domain

Status: Accepted (2026-08-11)

ノードの順番だけでなく、`scene_light`、`optical_image`、`film_latent_image`、`film_density`、`sensor_raw`、`scene_linear`、`display_linear`、`display_encoded`を型として持つ。接続domainが一致しないPipelineはレンダリング前に拒否する。特にDigital Sensor出力へChemical Developmentを直結するような誤構成を防ぐ。

## ADR-008: Physical and simulated characteristics retain provenance

Status: Accepted (2026-08-11)

全ノードに`Observed`、`Simulated`、`Transform`のroleを保存する。実カメラ／実レンズがすでに素材へ与えた特性と、後処理で追加した仮想機材の特性を区別し、二重適用と再現性喪失を防ぐ。

## ADR-009: Apple discovery and macOS preview are native Rust boundaries

Status: Accepted (2026-08-16)

権限状態、権限要求、device discoveryは`camera-apple`からTauriへmetadataとして公開する。macOSでは`AVCaptureSession`と`AVCaptureVideoPreviewLayer`も同crateが所有し、WKWebViewのnative NSViewへ直接配置する。WebViewへframeを運ばず、Web側とは状態、設定、viewport、完了結果、エラーだけを同期する。

sessionのblocking mutationは直列化し、NSView／CALayer mutationはmain threadへ限定する。WebViewをprivate APIで透過させる構成は配布互換性のため採用しない。preview内overlayは将来native layer／Metal compositorへ実装する。iOSでは同じcamera contractを保ちつつ、native plugin/viewの所有モデルを別途実装する。

## ADR-010: Native preview uses AppKit window geometry as coordinate truth

Status: Accepted (2026-08-16)

Tauri/WKWebViewでは`window.outerHeight - window.innerHeight`がdecorated windowでも0を返す場合がある。DOMの`preview-surface`はCSS pixelの矩形だけを通知し、macOS側が`NSWindow.contentLayoutRect`と`NSView.safeAreaInsets`からwindow chrome差分を計算して補正する。resizeはResizeObserverとwindow resize後のsettle passで同期する。OS固有座標補正をTypeScriptの固定値へ置かない。

## ADR-011: Initial still captures use app-managed atomic storage

Status: Accepted for MVP, Revisitable (2026-08-18)

最初のスチル縦切りは`AVCapturePhotoOutput`のdefault JPEGをapp data directoryの`captures`へ保存する。画像byte列をWebView IPCへ渡さず、Rust側で`.partial`へ書き、flush／sync後のrenameで完成assetだけを公開する。Photos libraryへ直接保存しないためPhotos権限は追加しない。ユーザー指定folder、Photos連携、JPEG／HEIF／RAW選択、オリジナルとImaging Pipeline処理済みassetの関連付けはMedia管理工程で再検討する。

## ADR-012: Movie assets become visible only after the final recording delegate

Status: Accepted for MVP, Revisitable (2026-08-20)

Appleの最初の動画縦切りは、preview／photoと同じ`AVCaptureSession`へmicrophone inputと`AVCaptureMovieFileOutput`を追加し、H.264/AACのQuickTime MOVをapp data directoryへ保存する。録画中は`.incomplete` directoryを使い、`stopRecording`呼出しだけでは完成扱いにしない。`AVCaptureFileOutputRecordingDelegate`の最終完了通知を受け、非空ファイルを確認した後だけ`captures`直下へrenameしてTauri IPCへ返す。

この方式は最短でphoto/video同格のnative縦切りを検証できる。一方、codec、bitrate、fragment化、Imaging Pipeline処理済みframe、厳密なaudio/video clock制御が必要になった段階では`AVAssetWriter`へ置換可能とする。初回権限検証はTauri bundleへcamera／audio-input Entitlementを署名適用し、TCCのresponsible applicationが製品bundleになるLaunchServices経由で起動する。

## ADR-013: Supported capabilities and the active capture format are separate contracts

Status: Accepted (2026-08-20)

`CameraCapabilities`はdeviceが対応できるresolution、frame rate、manual controlを表し、現在のsession設定を表さない。撮影画面の常時表示には、session開始後に`AVCaptureDevice.activeFormat`と`activeVideoMinFrameDuration`から取得した`PreviewStatus.active_format`を使う。対応最大値を現在値として表示しない。

能力モデルにまだ接続していない値は従来のデザインfixtureを残さず、`—`／`AUTO`／disabledで表現する。RAW、LOG、HDRはdevice全体の単純な真偽ではなくformat、color space、output構成の組合せとして次のschema改訂で扱う。

## ADR-014: Apple format selection prefers input priority and falls back to direct device configuration

Status: Accepted (2026-08-20)

resolution／FPSの選択肢は独立集合の直積ではなく、各`AVCaptureDeviceFormat`から得た対応組合せだけを提示する。適用時は録画中の変更を拒否し、対応するdevice formatとframe-rate rangeを再検証する。UIの整数24／30／60は、許容差内なら23.976／29.97／59.94などdeviceの実値へclampする。

sessionが`AVCaptureSessionPresetInputPriority`を受理する場合は同presetを使い、session presetによるactive formatの上書きを防ぐ。macOSのphoto＋movie output構成で同presetを受理しない場合は失敗にせず、session configuration transactionを開かずにdevice lock下で`activeFormat`とmin／max frame durationを直接設定する。実機では後者で1920 × 1080／30 FPSから1280 × 720／24 FPSへの変更を確認した。

## ADR-015: ACEScg is the normative rendering space; ACES2065-1 is an interchange space

Status: Accepted (2026-08-20)

Version 0.1で「ACES2065-1またはACEScg」としていた選択肢を廃止し、Version 0.2の標準内部計算色空間をscene-linear ACEScg（AP1）へ確定する。RGBA16FをPreview／Realtimeの最低精度、RGBA32FをReferenceの正本とする。ACES2065-1（AP0）はprofile交換、reference asset、archive用のinterchange spaceとして残す。

入力encoding、primaries、white point、transfer functionを暗黙に読み替えず、ACEScgへのinput transformとdisplay／encode用output transformを明示nodeとして記録する。custom working spaceは許可するが、profile ID、primaries、white point、transform versionをprojectへ保存しなければならない。

## ADR-016: Still and video share one finalized-asset lifecycle

Status: Accepted (2026-08-20)

StillとVideoはUI上だけでなく保存契約でも同格とする。両者は`Incomplete → Finalized | Failed`の共通asset lifecycleを使い、originalとImaging Pipeline処理済みderivativeを別resourceとして関連付ける。処理済みassetでoriginalを上書きしない。

Stillはfile syncとatomic rename、Videoはcontainer writerまたはplatform delegateの最終完了後にのみ`Finalized`へ移る。完成前のassetをMedia一覧やIPC成功結果へ公開してはいけない。derivativeは親resource、pipeline、profile version、engine version、seedを保持し、再現可能性を担保する。
