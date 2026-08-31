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

## ADR-017: Profile envelopes preserve unknown fields and resolve references in a catalog

Status: Accepted (2026-08-22)

Profile共通envelopeの`schema_version = 1`をJSON SchemaとRust型で固定する。同一major内の追加fieldは許可し、Rust loaderは未知fieldを`extensions`へ保持してround-tripで失わない。未知major schemaは互換と推測せず拒否する。

Profile間参照は埋め込みobjectではなくstable IDと期待kindで記録する。単体parseでは読み込み順が未確定なため参照先を要求せず、必要profileを`ProfileCatalog`へ登録した後に存在とkindを一括検証する。重複ID、自己参照、参照先不在、kind不一致を暗黙に補正しない。

## ADR-018: Film sensitometry uses explicit units and a strictly increasing exposure axis

Status: Accepted (2026-08-22)

Film Profile schema version 1のsensitometryは、x軸を`log10_lux_seconds`、y軸を`log10_optical_density`へ固定する。sampleは最低2点、`log_exposure`をstrictな単調増加、RGB densityを0以上の有限値とする。重複または逆行する露光sampleを自動sortして入力ミスを隠してはいけない。

補間方法は`monotonic_cubic | linear`、範囲外は`clamp | linear | reject`をProfileへ保存する。rendererごとの暗黙defaultに依存せず、CPU ReferenceとGPU rendererが同じcurve contractを共有する。

## ADR-019: Nearby sharing separates discovery from asset transport

Status: Accepted as a roadmap direction (2026-08-22)

ユーザー間の近距離共有では、Bluetooth／BLEをpeer発見、招待、capability negotiation、本人確認へ使用し、写真・動画本体はWi-Fi Direct、peer-to-peer Wi-Fi、同一local networkなど利用可能な高速経路へ切り替える。大容量assetをBLEだけで送り切ることを共通要件にしない。

Rustの`peer-transfer-core`がversioned Asset Manifest、chunk、cancel／resume、content hash、暗号化、transfer stateを所有し、Apple／Android／Windows／Linuxは発見とtransportのplatform adapterを持つ。双方の明示承認と短い確認コードを要求し、本名、Bluetooth address、永続device IDをadvertiseしない。EXIF位置情報とdevice metadataは送信前に共有範囲を選べるようにする。

受信assetは既存の`Incomplete → Finalized | Failed` lifecycleへ統合し、hash検証とflush完了前にMediaへ完成品として公開しない。実装開始条件とplatform別候補は [`ROADMAP.md`](ROADMAP.md) のMilestone 6を正本とする。

## ADR-020: Lens and digital sensor profiles expose physical ranges, not UI labels

Status: Accepted (2026-08-22)

Lens Profile v1は焦点距離とF-numberを正の数値範囲として保持し、prime lensも`min = max`で表現する。UI向けの「標準」「望遠」などの名称を物理contractにしない。Anamorphic squeezeはanamorphic lensだけに許可し、spherical lensへ暗黙defaultを設定しない。

Digital Sensor Profile v1はactive pixel寸法と物理寸法、native bit depth、CFA、black／white level、base ISOとISO範囲を分離する。white levelはnative code range内、base ISOは宣言範囲内でなければならない。分光感度は任意だが、存在する場合は360–830 nmのstrictな昇順sampleと非負responseを要求し、波長点を自動sortしない。

## ADR-021: Virtual exposure is an explicit calibrated RGB adapter

Status: Accepted (2026-08-23)

既存素材のscene-linear ACEScgは、`virtual_exposure` nodeでのみFilm sensitometry用のRGB `log10(lux·s)`へ接続する。version 1は18% neutral grayを通常の相対anchorとし、`reference_log_exposure`、EV補正、正のblack floor、負値のclamp／reject方針をPipelineへ明示保存する。18% grayから絶対露光を推測しない。

このadapterはRGB Film Emulation用の校正近似であり、scene radiance、レンズ透過、photometric weighting、film spectral sensitivityの物理積分を表すものではない。Spectral／Physical Modeは別adapterとして実装する。ACEScg以外のworking space、非有限値、不正な基準値は実行前に拒否する。数式と受け入れ条件は [`VIRTUAL_EXPOSURE_ADAPTER.md`](VIRTUAL_EXPOSURE_ADAPTER.md) を正本とする。

## ADR-022: CPU Film rendering belongs to film-core and returns a density-domain type

Status: Accepted (2026-08-23)

Pipeline記述、Profile、SignalDomainを所有する`imaging-core`はrendererへ依存させない。CPU画素処理は専門rendererの`film-core`へ置き、`film-core → imaging-core`の一方向依存とする。

scene-linear入力には既存の`LinearImage`を使うが、sensitometry後のRGBは色値ではなくlog10 optical densityであるため、ACEScg `FrameDescriptor`を持つ同じ型へ上書きしない。width、height、density RGBAを持つ`FilmDensityImage`として返す。straight alphaはFilm演算の対象外として保持する。補間、範囲外処理、error条件は [`CPU_REFERENCE_FILM_EXECUTOR.md`](CPU_REFERENCE_FILM_EXECUTOR.md) を正本とする。

## ADR-023: Finishing profiles encode domain and display constraints before rendering

Status: Accepted (2026-08-23)

Development、Print、Display、Output Transformは共通Envelopeの任意JSONとして扱わず、schema version 1のtyped payloadとしてrender前に検証する。ChemicalとDigital RAWの必須条件、Print種別とSignalDomain、Display xy／transfer／luminance、Output encodingとtransferの組合せを不変条件にする。

Profile artifactの存在はrenderer完成を意味しない。現在のexamplesはすべてsynthetic provenanceであり、正式なECN-2、ACES ODT、測定済みRec.709 monitorの再現とは表現しない。詳細contractと未実装範囲は [`FINISHING_PROFILES.md`](FINISHING_PROFILES.md) を正本とする。

## ADR-024: Render snapshots hash the enabled Pipeline and selected Profile closure

Status: Accepted (2026-08-23)

render snapshotはCatalog全体ではなく、有効Pipeline nodeが直接参照するProfileと、その`references`の推移closureだけを含む。disabled nodeと無関係なProfileを含めず、ID順へ正規化する。Pipeline、各Profile、snapshot payloadをSHA-256で個別に固定し、Profile version据え置きの内容変更も検出する。

hashは検証済みRust型の決定論的JSONから計算し、元fileの空白やobject key順には依存しない。未知same-major extensionは再現対象なのでhashへ含む。SHA-256を作成者署名やtrustの代替には使わない。loaderとhashの詳細は [`PROFILE_DIRECTORY_AND_SNAPSHOT.md`](PROFILE_DIRECTORY_AND_SNAPSHOT.md) を正本とする。

## ADR-025: Minimal CPU output is a matrix transform, not a synthetic ACES ODT

Status: Accepted (2026-08-23)

最初のCPU Output Transformは`matrix_tone_curve`かつtone mapping `none`に限定する。ACEScg AP1／D60からDisplay Profileのprimaries／whiteへRGB–XYZ行列とBradford adaptationで変換し、display gamut clamp後に宣言transferを適用する。straight alphaは演算対象外とする。

`aces_odt`、`ocio`、ACES／perceptual tone mappingは、正式transformまたは依存runtimeが接続されるまで拒否する。実装していないlookを合成近似で同じ名称にしない。Chemical Developmentも測定push／pull curveがないためnormal 0 stopだけを処理する。詳細とgolden fixtureは [`CPU_REFERENCE_FINISHING.md`](CPU_REFERENCE_FINISHING.md) を正本とする。

## ADR-026: The first Print renderer is explicitly synthetic and versioned

Status: Accepted (2026-08-23)

測定済みprint sensitometryがない状態で、一般的なphotochemical responseを暗黙に推測しない。最初のFilm Density→Display Linear縦切りは`inverse_density_preview_v1`というversioned synthetic response modelをProfileへ必須記録し、base density、contrast、printer exposureから単調なpreview値を生成する。

`measured_curve`と`digital_transform`は別modelとして予約し、対応dataとrendererができるまで実行を拒否する。合成modelの出力を実print film、paper、printer lightの測定再現と表現してはいけない。数式と適用範囲は [`CPU_REFERENCE_FINISHING.md`](CPU_REFERENCE_FINISHING.md) を正本とする。

## ADR-027: Profile migrations are explicit trusted major-version steps

Status: Accepted (2026-08-23)

Profile migrationは`from N → N+1`ごとの名前付きcompiled functionだけを実行する。missing step、duplicate source step、future schema、宣言と異なる出力versionは拒否し、近いSchemaやfield名を推測しない。migration後に共通Envelopeとtyped payloadを再検証し、その最終内容をsnapshot hashへ使う。

現在公開済みのlegacy Profile schemaはないためbuilt-in migrationは0件とする。合成v0→v1 stepはregistry test専用であり、製品仕様として受理しない。旧Schemaが実在した時点でbefore／after fixtureと情報損失policyを伴って追加する。詳細は [`PROFILE_MIGRATION_REGISTRY.md`](PROFILE_MIGRATION_REGISTRY.md) を正本とする。

## ADR-028: A capture is finalized only after an in-process media probe

Status: Accepted (2026-08-24)

AVFoundation delegate完了とfile非空だけでは、UIで選択したformatが保存trackへ反映されたことを保証できない。Still／Videoは`.incomplete`へ保存し、camera-coreがJPEGまたはQuickTime／ISO BMFFを直接probeして、寸法、container、codec、FPS、duration、audio、orientation／rotation、color metadataを`CapturedAsset`へ格納する。必須check不一致のresourceは完成directoryへrenameしない。

macOSではPhotoOutputとMovieFileOutputの同時接続によるformat再交渉を避けるため、Video開始時にPhotoOutputを外し、標準session presetをcommitしてからdevice active formatを最終適用する。Stillへ戻るとPhotoOutputを再接続し同じformatを再適用する。外部`ffprobe`や`mdls`はcross-checkだけに使い、製品の成功判定へ依存させない。詳細は[`CAPTURED_ASSET_CONTRACT.md`](CAPTURED_ASSET_CONTRACT.md)を正本とする。

## ADR-029: Preview mirroring is independent from captured-media mirroring

Status: Accepted (2026-08-24)

画面姿勢は共通`CaptureOrientation`で0／90／180／270度だけを受理し、Preview、Photo、MovieのAVFoundation connectionへ同じrotationを適用する。AVFoundationがStillではEXIF、VideoではQuickTime track matrix、Previewではlayer transformを生成するため、保存後にpixelを再回転しない。

front cameraのpreviewは操作感のためmirrorを許可する一方、Photo／Movieは他のImaging Pipelineや編集ソフトとの相互運用を優先して既定で非mirrorにする。このため`preview_mirrored`と`capture_mirrored`を独立fieldとして保持する。録画途中のorientation変更は拒否し、停止後に最新のUI姿勢を同期する。portrait／upside-down／front-cameraの最終受け入れはiOS実機で行う。

## ADR-030: Derivatives form an append-only reproducible resource graph

Status: Accepted (2026-08-26)

`CapturedAsset` schema version 2ではoriginalにも明示的なresource IDを付け、各derivativeは既存originalまたは先に追加済みderivativeをparentとして参照する。未来のresourceをparentにできない追加順制約により循環を防ぎ、resource IDとpathの重複を拒否してoriginalを上書きしない。

各derivativeはPipeline IDだけでなく、Pipelineと必要Profile closureのSHA-256を含む完全な`RenderProfileSnapshot`、engine version、seedを保持する。これらは再現条件の識別契約であり、Profile packageの真正性を証明する署名ではない。型・validation・JSON round-tripに加え、atomic manifestとMedia indexへの永続化まで実装済みである。

## ADR-031: A capture succeeds only after its media manifest is durable

Status: Accepted (2026-08-27)

Finalized mediaはresourceだけでなく、CapturedAsset schema v2全体を含むversioned manifestと一組で公開する。mediaを完成pathへrenameした後、manifestを`.partial`へflush／syncしてrenameする。manifest保存が失敗した場合はmediaを`.incomplete`へ戻し、capture commandを失敗させる。

Media indexはmanifestのFinalized／Failed recordと`.incomplete`のresourceを統合する。壊れたmanifestを黙って除外するとlibraryが正常に見えてしまうため、parse、schema、record整合性errorはindex全体の診断として返す。cleanupとcrash orphan reconciliationは自動削除せず、別の確認可能なUI milestoneで実装する。

## ADR-032: Recovery cleanup is explicit and orphan reconciliation is non-destructive

Status: Accepted (2026-08-27)

Media読込み時にcaptures root直下をreconcileし、対応manifestのない既知拡張子の通常ファイルを`Failed` recordへ変換する。crash orphanは完成品か破損物かを自動判定できないため、reconciliationではresourceを削除しない。

cleanupは`Failed`／`Incomplete`に限定し、詳細表示とは別の確認dialogを要求する。Rust commandも`Finalized`を拒否し、安全なrecord ID、canonical path containment、通常ファイルであることを再検証する。UIの非表示だけを安全境界にせず、外部path、directory、完成assetを削除できない契約とする。

## ADR-033: Mobile scaffolds are versioned and native camera frames stay outside WebView IPC

Status: Accepted (2026-08-28)

Tauriが生成するiOS／Android projectはpermission、framework link、native adapterを所有するsource artifactなので`src-tauri/gen`をrepository管理対象とする。build output、local SDK path、generated JNI symlinkは各platformの`.gitignore`で除外する。

iOSはAVFoundation preview layer、AndroidはCameraX Surfaceをnative viewとしてWebViewと合成する。連続frameをTauri IPCへ渡さず、共有Rust層へはcamera control、capability、metadata、完成resourceだけを渡す。scaffoldとdebug artifactのbuild成功はnative camera runtimeの完成を意味しない。

## ADR-034: iOS preview is a retained UIView host attached on the Tauri main thread

Status: Accepted (2026-08-28)

iOSではTauriのWKWebViewを`UIView`として扱い、撮影viewportと同じframeを持つ専用host viewへ`AVCaptureVideoPreviewLayer`を追加する。host pointerはRust側でretainし、resizeとdetachもTauriのmain-thread closure内だけで実行する。WebViewへ連続pixelを転送せず、macOSと同じcontrol IPCとCapturedAsset保存境界を維持する。

Simulator buildはUIKit／AVFoundationの型・link検証に使うが、camera device、permission、orientation、mirror、音声付きVideoの受け入れ判定には使わない。これらは署名済みiPhone実機で判定する。

## ADR-035: Android CameraX is owned by a Tauri mobile plugin and PreviewView

Status: Accepted (2026-08-28)

Android camera runtimeはKotlinのTauri mobile pluginが所有する。Camera permission、CameraManager discovery／capability、ProcessCameraProvider、PreviewViewをplugin内に閉じ、Rust commandはJSON control contractだけを中継する。PreviewViewはCSS viewportをdisplay densityでnative pixelへ変換した位置へ重ね、pause／destroy／Media遷移で必ずunbindして除去する。

複数のlogical cameraを最初のfront／back endpointへ正規化する初期契約とする。物理camera切替、concurrent camera、extension modeは実機capability方針確定後に拡張する。Still／VideoはPreview完成と混同せず、CameraX ImageCapture／VideoCaptureのfinalize callbackを共通CapturedAsset境界へ接続してから`[Done]`とする。

## ADR-036: CameraX completion and CapturedAsset finalization are separate boundaries

Status: Accepted (2026-08-28)

Android native pluginはPreview、ImageCapture、VideoCaptureを同じProcessCameraProvider lifecycleへbindする。Stillは`OnImageSavedCallback`、Videoは`VideoRecordEvent.Finalize`が成功するまでRustへ完成通知を返さない。preview停止やActivity pauseで録画が中断された場合は成功として扱わない。

CameraX callbackはcontainer writer完了の境界であり、製品asset完成の境界ではない。出力はまず`.incomplete`へ置き、Rustのin-process JPEG／ISO BMFF probe、CapturedAsset validation、rename、atomic manifest保存がすべて成功した時だけFinalizedとする。format未選択時はCameraXが交渉した実出力からcapture metadataを確定する。明示選択時の扱いはADR-037で追加規定する。

## ADR-037: Android format requests fail explicitly instead of silently falling back

Status: Accepted (2026-08-28)

UIで選択した解像度はCameraX `ResolutionSelector`の`FALLBACK_RULE_NONE`としてPreview、ImageCapture、VideoCaptureへ同時指定し、FPSはCamera2 interopのAE target rangeとして指定する。use-case再bindに失敗した場合は以前の構成へrollbackし、近い解像度へ暗黙に置換しない。

native撮影完了結果は要求formatもRustへ返す。CapturedAsset validationはprobeした保存寸法・FPSを要求値と照合し、不一致をFinalizedにしない。Camera2 capabilityのsizeとFPS rangeから作る候補が三use-case同時bind可能とは限らないため、code build成功と端末別format conformanceを分離し、実機で確認した組合せだけを将来のcapability一覧へ残す。

## ADR-038: Recovery reinspection never invents missing capture intent

Status: Accepted (2026-08-30)

Failed／Incomplete resourceの再検査は、既存のJPEG／ISO BMFF probeを再実行して診断を更新する非破壊操作とする。元のselected format、device、orientation intentが完全には残っていないresourceについて、現在の画素寸法やFPSを期待値として捏造しFinalizedへ昇格してはいけない。

probe成功時もFailed状態を維持し、構造的に読めることとcapture contractを満たすことを区別する。利用者はresourceを残したままStill／Videoを再撮影でき、削除は従来どおり別の確認付きcleanupだけが行う。

## ADR-039: Output presets advertise only enforced native writer configurations

Status: Accepted (2026-08-30)

出力presetは将来候補の一覧ではなく、現在のnative writerと保存後probeが実際に保証する構成だけを返す。初期値はStillのJPEGとVideoのH.264／AACであり、AppleはQuickTime、AndroidはMP4 containerとする。RAW、HEVC、LOG、bitrate選択はnative設定と保存後検証が接続されるまで表示しない。

残容量はcapture directoryと同じfilesystemのavailable blocksを`statvfs`で取得し、total capacityと区別する。概算撮影可能量はJPEG 8 MiB／枚、Video 120 MiB／分のnominal planning値であり、保証値ではない。実測bitrateがCapturedAssetへ記録される段階で、端末／format別rolling estimateへ置換する。

## ADR-040: Capture storage preflight is enforced by the backend

Status: Accepted (2026-08-30)

Still撮影は8 MiB、Video録画開始は120 MiBのnominal出力量に加え、256 MiBのfilesystem安全予約を残せる場合だけ許可する。UIは同じ判定を表示して中央capture controlを無効化するが、権威はApple／Android共通のRust command直前検査に置く。これにより表示後の容量変化やUI迂回でも開始を拒否できる。

この値は完成ファイル容量の保証ではなく開始前guardである。録画中の容量減少は別の連続監視で検出し、containerを壊さず停止・Finalizeできる実装が成立するまで完了扱いにしない。容量APIを未実装のplatformでは誤って常時停止しないようpreflightを許可し、platform固有実装を追加する。

## ADR-041: Foreground storage auto-stop reuses the manual finalize path

Status: Accepted (2026-08-30)

foreground録画中はWebViewから2秒間隔で同じstorage status commandを呼び、Videoの安全閾値を下回った時点で手動停止と同じ`stop_video_recording`を一度だけ実行する。容量監視専用の強制終了経路は作らず、native writer停止、probe、CapturedAsset validation、rename、manifest保存を共通化する。容量取得の一時的失敗は録画破棄の理由にせず、次回pollで再試行する。

このmonitorはforeground UXの防御であり、OSがWebView timerをpauseするbackground状態の安全保証ではない。次工程ではApple／Android native lifecycleがfilesystem閾値を監視し、同じ停止・Finalize契約へ通知する。native側が完成するまでbackground録画の容量保護を`[Done]`と表現しない。

## ADR-042: Android native storage stop retains CameraX Finalize for Rust recovery

Status: Accepted (2026-08-30)

AndroidはCameraX pluginのmain looperで2秒間隔に保存先`usableSpace`を確認し、Rustが渡すVideo概算120 MiB＋安全予約256 MiBを下回ると`Recording.stop()`を一度だけ呼ぶ。WebView timerやIPC応答待ちには依存しない。Activity `onPause`／`onDestroy`も録画中は即時`close()`せず、同じstop→Finalizeを優先する。

native側で先にFinalizeした場合、その成功結果または診断をplugin内へ保持する。復帰後の`stopVideo`は保持結果を返し、RustのPendingMovie、probe、CapturedAsset validation、atomic rename、manifest保存を従来どおり完了する。native container保全とapplication asset確定を別境界として扱い、途中状態をFinalized Mediaとして公開しない。

## ADR-043: Apple storage monitor issues an idempotent AVFoundation stop request

Status: Accepted (2026-08-30)

Apple録画中はRust native runtimeがPendingMovie ID、CameraState、capture filesystemを2秒間隔で確認する。Videoの安全閾値を下回るとAVFoundation sessionへ停止要求を送る。各MovieRecordingはatomic stop flagを持ち、容量monitorと利用者の手動停止が競合しても`AVCaptureMovieFileOutput.stopRecording()`を一度だけ呼ぶ。

停止要求だけでCapturedAssetを完成扱いにしない。既存delegateのFinished通知をreceiverが保持し、Rustのstop commandが非空確認、probe、validation、rename、manifest保存を完了する。これはWebView timer停止から独立したprocess-resident保護であり、iOSがapp process全体をsuspendする条件やAVCaptureSession interruptionの保証ではない。それらは実機lifecycle試験と明示的interruption observerが必要である。

## ADR-044: Peer transfer protocol cannot publish before contiguous byte and hash verification

Status: Accepted (2026-08-30)

`peer-transfer-core`はOS固有の発見／transportから独立した状態機械とする。peer identityはvisibility sessionごとのephemeral IDとし、Bluetooth address、端末名、永続device IDをprotocol identityにしない。招待は期限と双方で比較する6桁確認codeを持つ。BLEはcontrol／discovery用途に限定し、共通のlocal network、peer-to-peer Wi-Fi、Wi-Fi Directがない場合はasset転送へ進めない。

受信前にmanifest version、basename、byte上限、chunk範囲、SHA-256表現を拒否可能にする。ACKは連続受信byteだけを単調増加で表し、宣言長を越えられない。全byte受信後もVerifyingを経由し、宣言byte数とSHA-256が一致した場合だけFinalizedへ遷移する。実file writerとMedia Incomplete lifecycleへの接続は別工程だが、この不変条件をplatform adapterで迂回してはいけない。

## ADR-045: Receive ACK means bytes and resume ledger are durable

Status: Accepted (2026-08-30)

受信writerは送信側のpathを使用せず、検証済みtransfer IDからapp管理下の`.incomplete/peer-transfer`へpartとresume ledgerを作る。chunkは現在の永続offsetと完全一致する連続dataだけを受理し、partへ`sync_data`した後でledgerを一時fileからrenameする。ACKはこの両方が成功した位置だけを返す。

再開時はmanifest、ledger offset、part file長が一致しなければ推測修復せず拒否し、保存済みbyteを再読込してSHA-256状態を復元する。全byte受信後に`sync_all`、実file hash、宣言hashを比較し、一致した場合だけ完成basenameへrenameする。hash不一致はpartを保持してFailedとし、完成先を作らない。Media manifestとのatomic公開境界は次工程で接続する。

## ADR-046: Received originals preserve capture intent; derivatives are not disguised as originals

Status: Accepted (2026-08-30)

Asset Transfer ManifestはOriginal、指定Derivative、Original＋指定Derivativeを明示的に区別する。Original受信は送信元CaptureMetadataを保持し、実file probe結果を元の選択寸法／FPSと照合してからローカルCapturedAssetへする。開始時はMedia Incomplete、hash／probe／validation失敗はFailed、Media manifest保存成功後だけFinalizedとする。

Derivativeだけを受信した場合に、来歴を捨ててCapturedAsset Originalへ見せかけてはいけない。Derivative bundleはparent resource ID、render snapshot、engine version、seedを保持できる確定処理が完成するまでOriginal adapterへ通さない。また`StripLocation`／`StripDeviceAndLocation`は実byteを変換して再hashするsanitizerが必要であり、現行source builderは未実装の除去を宣言せず明示的に拒否する。

## ADR-047: JPEG privacy policy is proven by a byte rewrite, not a manifest claim

Status: Accepted (2026-08-30)

`StripDeviceAndLocation`は元fileのmanifest fieldだけを変更せず、別のJPEGへ実byteを書き直す。EXIF／XMP／IPTC／comment／未知APP segmentを除去し、ICC profile、Adobe color interpretation、標準JFIF／JFXXは保持する。scan開始後のpixel entropyは変更せず、出力をflushしてSHA-256を再計算する。転送manifestはこの新しいbyte列の長さとhashを参照しなければならない。

sanitizer出力はprivate pathを持つ不透明な`SanitizedJpeg`として渡す。Strip policy用builderはこの型を要求し、JPEGを再probeして元Originalと画素寸法を照合したうえで、現在のbyte長とSHA-256をsanitizer reportと再照合する。通常のCapturedAsset builderはStrip指定を拒否し続けるため、未変換の任意fileを除去済みとして生成する公開経路を設けない。

`StripLocation`はEXIF GPS IFDだけを消す選択的TIFF再構築が必要である。pointerやoffsetを部分的に壊す危険があるため、実装完了までは明示的に拒否する。MOV／MP4 metadataもcontainer固有の再構築を別工程とし、JPEG sanitizerで対応済みと扱わない。

## ADR-048: A received derivative may only extend an existing verified parent

Status: Accepted (2026-08-30)

Derivative用`TransferResource`はpurposeに加えて`DerivativeProvenance`全体を運ぶ。受信側はprovenanceが欠落しているresource、親resource IDがFinalized asset内に存在しないresource、親とmedia typeが異なるresourceをIncomplete writer開始前に拒否する。content hash確定後も実fileをprobeし、既存`CapturedAsset::add_derivative`によるsnapshot hash、engine version、親子関係の検証を通してから親asset manifestを更新する。

Derivativeだけを新規CapturedAsset Originalとして作らない。Original＋Derivative bundleでsource側resource IDと受信側local resource IDが異なる場合は暗黙に書き換えず、bundle coordinatorが明示的な対応表と依存順序を管理するまで未対応とする。

## ADR-049: Bundle resource identity is mapped only after each dependency is finalized

Status: Accepted (2026-08-30)

Original＋Derivative bundleはOriginalを先に受信・検証・Media確定し、その完成結果からsource Original resource IDとlocal Original resource IDの対応を登録する。Derivativeのprovenance parentはこの確定済みmapだけを使ってlocal IDへ変換する。受信予定、Incomplete、Failed resourceをmapへ追加してはならない。

coordinatorはresource IDとtransfer IDの重複、selectionとの不一致、media type不一致、存在しない親、循環依存を開始前に拒否する。各resourceは個別のInvitation承認済み`TransferSession`を通り、coordinator自体は本人確認、transport交渉、content hash検証を省略する権限を持たない。

## ADR-050: Chunk confidentiality and resume continuity are authenticated separately

Status: Accepted (2026-08-30)

asset chunkはChaCha20-Poly1305で暗号化し、protocol version、transfer ID、offset、平文長、asset総長をAADへ含める。これによりciphertextの改ざんだけでなく、正しいchunkを別offset、別transfer、別manifestへ移す操作も認証失敗にする。nonceはsession固有prefixとkeyに加え、transfer ID、offset、平文hashから導出し、同一offsetを異なる内容で再暗号化した場合のnonce再利用を避ける。session keyはmemory上でzeroize対象にする。

resumeはAEAD frameの認証だけでは保存済みprefixの同一性を証明できない。受信側はdurable ACK位置とprefix SHA-256をcheckpointとして提示し、送信側が元fileの同じprefixを再hashして一致した場合だけ再開する。sessionがresume対応を交渉していない場合はcheckpointを拒否する。

この決定はkey agreementを定義しない。ephemeral public keyと6桁確認codeをsession key導出へbindingし、実transportへframeを接続するまではE2E暗号化の完成条件を満たさない。

## ADR-051: The six-digit comparison authenticates the ephemeral handshake transcript

Status: Accepted (2026-08-31)

双方はplatform CSPRNG由来のsession限定X25519 key pairを交換する。6桁確認codeはInvitationで任意生成せず、X25519 shared secret、双方のpublic key、Invitation identity、transfer ID、asset hash、byte長から導出する。利用者が二画面のcode一致を確認することで、public keyまたはManifestを差し替える中間者を検知する。

確認済みcodeとInvitation identityをsalt、X25519 shared secretをinput key material、sort済みpublic keyとManifest identityをcontextとしてHKDF-SHA256からchunk keyとnonce prefixを導出する。all-zero secret、自己public key、不一致codeを拒否する。Rust共通層へ汎用乱数generatorは置かず、各platformのCSPRNG adapterを次工程で接続する。

## ADR-052: Local-network framing is bounded before allocation and independent of discovery

Status: Accepted (2026-08-31)

session secretの通常生成は`getrandom`を通じてOS CSPRNGへ委譲する。共通層は乱数algorithmやseedを自作せず、取得したsecretをX25519 key pair生成後もzeroize対象として扱う。

実data transportはdiscovery APIから独立したbounded binary stream protocolとする。EncryptedChunk、ResumeCheckpoint、DurableAckを明示的なmessage kindで表し、共通headerのpayload長を最大chunk＋固定overhead以下と確認してからallocationする。transfer ID、宣言平文長、ciphertext長、hash文字列、payload完全消費を検査し、未知kindや余剰dataを許容しない。

最初のadapterは`TcpStream`を使用するが、接続先発見やlisten policyをcoreへ含めない。Bonjour、Nearby Connections、Wi-Fi Directなどは検証済みstreamを渡す責務を持ち、暗号化・framing・ACK意味論を独自実装しない。

## ADR-053: One encrypted chunk may be in flight until its durable ACK

Status: Accepted (2026-08-31)

最初のtransport lifecycleはstop-and-waitとし、senderは1 chunkを送った後、その正確なend offsetのDurableAckを受けるまで次chunkへ進まない。throughputよりもresume位置、nonce context、filesystem durabilityの対応が明確であることを優先する。windowed transferはACK rangeと再送nonce規則を別versionで定義してから導入する。

disconnectは成功や通常cancelとして扱わず、senderを`PeerDisconnected`へ移す。resume対応を交渉済みで、receiver checkpointとsender source prefix hashが一致した場合だけTransferringへ戻す。Cancelled／Completeからの送信再開とComplete後のcancelは禁止する。receiverがCompleteになってもMedia完成ではなく、従来の全file SHA-256、atomic rename、Media manifest確定を通過するまで公開しない。

## ADR-054: Apple discovery advertises a bound endpoint and ephemeral public identity only

Status: Accepted (2026-08-31)

Appleの最初の近距離発見はBonjour互換DNS-SD service `_ufcamera._tcp.local.`とする。advertise前にTCP listenerをbindし、OSが確保した実portだけをSRV recordへ載せる。Apple P2P interfaceを有効にするが、到達経路の選択はresolved addressへ接続するtransport adapterに任せる。

TXT recordへ載せるのはprotocol version、session限定X25519 public key、そのpublic key由来ephemeral ID、任意の利用者labelだけである。端末名、Bluetooth address、永続device ID、secret、確認code、asset情報を公開しない。発見したTXTはuntrusted inputとして形式とversionを検証し、最終的なpeer認証は引き続きtranscript由来6桁codeの二画面比較で行う。

discovery stateはTauri commandからstart／snapshot／stopするpoll modelとする。native daemon callbackからWebViewへ無制限eventをpushせず、resolved peerをID順snapshotとして返す。application終了時はadvertise、browse、listener、ephemeral keyを同じstate ownerが破棄する。

## ADR-055: Nearby visibility is scoped to its dedicated foreground screen

Status: Accepted (2026-08-31)

最初のNearby UXでは専用画面を開いた明示操作でだけadvertise／browseを開始し、撮影画面へ戻る操作で停止する。Nearby表示中はnative camera previewも停止し、camera hardware、local network permission、発見状態を同時に隠れて動かさない。application unloadでも停止を要求し、native state ownerのDropを最終防衛線とする。

発見結果は1.5秒pollのsnapshotで表示し、peerの存在を接続・本人確認・転送承認と同義にしない。UIはephemeral IDと到達候補だけを示し、asset選択、Invitation、6桁code一致、双方の承認が実装されるまでpeer cardへ転送開始操作を設けない。

## ADR-056: Local code approval remains Negotiating until authenticated remote approval

Status: Accepted (2026-08-31)

送信準備はMedia indexのFinalized entryをIDで再解決し、実fileからTransfer Manifestを生成する。UIが渡したpath、byte長、hash、完成状態を信頼しない。Invitation IDはOS CSPRNG由来とし、2分で失効する。6桁codeは既存ADR-051のhandshake transcriptから導出し、独立したUI乱数を使わない。

利用者が「コード一致・承認」を押した時点ではlocal `TransferSession`をAwaitingApprovalからNegotiatingへ進めるだけとする。発見広告だけでは相手が同じManifestを見たことも承認したことも証明できない。remote approvalを認証済みcontrol channelから受け取り、双方のtranscript一致を確認する前にAgreedSessionSecrets、EncryptedChunkCodec、Transferring stateを作らない。

## ADR-057: Mutual approval and capability negotiation share one bounded control transcript

Status: Accepted (2026-08-31)

Handshake OfferはInvitation、Transfer Manifest、sender public key、sender capabilityを1つのcontrol messageとして運ぶ。ApprovalはInvitation ID、transfer ID、同一確認code、別のapprover public key、approver capability、明示approved flagを返す。一部fieldだけを別sessionから再利用できないよう、暗号codec生成前に全contextを照合する。

control messageも既存UFC1 binary envelopeを使うが、asset chunkと同じ4 MiB上限を許さず64 KiBへ制限する。headerのkindとlengthからallocation前に上限を適用する。双方のlocal sessionが承認済みNegotiatingでなければ相互handshakeをcompleteせず、capability negotiationとX25519／HKDFが両方成功した結果としてのみTransferring sessionとcodecを生成する。

## ADR-058: Incoming connections must correspond to a currently discovered ephemeral key

Status: Accepted (2026-08-31)

Apple listenerが受けた任意TCP connectionをInvitationとしてUIへ表示しない。最初のmessageはbounded Handshake Offerでなければならず、sender ephemeral IDとX25519 public keyが現在のBonjour discovery snapshotの同一peerへ一致することを要求する。さらに受信側local keyで6桁codeを再導出し、Offerのcodeと一致してから利用者へ提示する。

outbound接続は複数resolved addressを順に試すが、各addressを5秒で制限する。remote approval待機はInvitation期限を含む125秒以内とし、待機中にglobal Nearby mutexを保持しない。戻り時にはapproval identityとsession stateを再検査し、待機中にcancel／stopされた古い結果からkeyやsecure sessionを復活させない。

## ADR-059: Sender completion requires receiver Media finalization evidence

Status: Accepted (2026-08-31)

最後のDurableAckは全byteがreceiver filesystemへ永続化されたことだけを意味し、SHA-256、media probe、CapturedAsset validation、Media manifest保存の成功を意味しない。receiverはこれらをすべて完了した後にだけtransfer IDとManifest SHA-256を持つTransferFinalizedを返す。senderは両fieldを照合するまで利用者へ完了を表示しない。

受信OriginalはHandshake OfferのTransferResourceとCaptureMetadataからIndexedOriginalReceiveを作り、送信元pathを使用しない。既存partialが存在する場合、resume checkpoint交換前にoffset 0のchunkを受け入れず明示停止する。安全なresumeが未接続であることを、暗黙の再送やpartial削除で隠さない。

## ADR-060: The application icon represents the imaging system, not one medium

Status: Accepted (2026-08-31)

アプリアイコンはフィルム孔や具体的なカメラ筐体を主題にせず、光学絞り、デジタルセンサー、動画の進行方向を1つの幾何学マークへ統合する。これによりStill／VideoおよびFilm／Digitalを同格に扱うUniversal Imaging Pipelineの製品境界と一致させる。

graphite／gunmetalを基調に状態表示のcyanだけをaccentとする。platform別bitmapを直接修正せず、`assets/app-icon.svg`を正本としてTauri CLIからICNS、ICO、PNG、iOS AppIcon、Android launcher assetを再生成する。初期生成案は設計資料として保存するが、launcherで使うのは16–32 pxでの判別性を優先して整理したvector版とする。

## ADR-061: Receiver checkpoint precedes every Apple asset chunk stream

Status: Accepted (2026-08-31)

Appleのsecure transfer taskではreceiverが`IndexedOriginalReceive::create_or_resume`を通してledgerとpart fileを検証した後、最初のwire messageとしてdurable `ResumeCheckpoint`を送る。fresh transferでもoffset 0と空prefixのSHA-256を送るため、senderはreceiver readinessを推測しない。

senderは元file全体がManifest hashと一致することを再検査し、さらにcheckpoint位置までのprefix SHA-256を照合してから`EncryptedTransferSender`をそのoffsetで開く。不一致、別transfer ID、総byte超過、resume非交渉sessionはchunkを1 byteも送る前に拒否する。これで既存partialのoffset 0上書きは解消するが、socket切断後のpeer再発見、新しいephemeral handshake、同一transfer identityへの復帰は別の再接続state machineとして未完了である。

## ADR-062: Transfer progress means receiver-durable bytes

Status: Accepted (2026-08-31)

Nearby UIのprogressはsenderがsocketへ書いたbyte数ではなく、receiverがfilesystemへ同期しDurableAckを返した連続offsetだけを表示する。native transfer taskとsnapshotの間は共有atomic stateを使い、TCP待機中にglobal Nearby mutexを保持しない。表示はbyte数と総量、割合を併記し、最終100%はreceiverのMedia Finalized証明後だけ確定する。

cancelはUIだけを閉じず、共有cancel flagをtaskがchunk境界で検出して同じtransfer IDのwire Cancelを相手へ送る。重複cancelを無効化し、receiverは別transfer IDのCancelを拒否する。blocking socket receive中の即時interrupt、timeout／disconnect後の再試行は今後の非同期task／再接続state machineで扱う。

## ADR-063: Reconnect reuses only the approved visibility-session transcript

Status: Accepted (2026-08-31)

X25519 ephemeral key pairの寿命を単一TCP socketではなく、利用者がNearby専用画面を開いている一時可視セッションとする。keyは検出停止、画面離脱、application終了で破棄し、永続device identityとして保存しない。同じ可視セッション内では、承認済みInvitation、Offer、transfer ID、Manifestが完全一致する切断再接続に限り、同じtranscriptからcodecを再導出できる。

senderは接続中断後もPrepared Approvalとsource pathを保持し、明示retryで新しいTCP接続を作る。receiverは現在発見中の同じephemeral ID／public keyから届いた完全一致Offerだけを自動再承認する。双方とも新しい`TransferSession`をNegotiatingから作り直し、secure transport確立後はreceiverのdurable checkpointを最初に交換する。別Offer、別transfer ID、別peer key、期限切れInvitationは再接続として扱わない。

同じkey／Manifest／offsetで再暗号化されるchunkは同じplaintextとnonce contextを持つ。checkpoint prefixが一致しない内容を同じoffsetで送らず、nonceはplaintext hashにもbindingされている。Invitation失効後、Nearby画面を閉じた後、mobile backgroundやnetwork interface変更をまたぐ再開は新しい承認フローとして今後定義する。

## ADR-064: Retry eligibility is derived from a closed failure taxonomy

Status: Accepted (2026-08-31)

Nearby native stateは失敗をDisconnected、Timeout、Integrity、Storage、InvitationExpired、Cancelled、Protocolの閉じた分類としてsnapshotへ公開する。内部error文字列は診断用に保持するが、UIの判断や翻訳に使用しない。接続中断とtimeoutだけを同じ承認済みtranscriptからretry可能とし、Invitationが有効でcancel要求がない場合に限る。

hash／prefix／AEAD／Manifest不一致を含むIntegrity、容量不足、期限切れ、利用者cancel、予期しないprotocol errorは自動retryしない。特にIntegrity failureをnetwork failureとして再送すると、破損partialや異なるsourceを正常なresumeとして扱う危険がある。UIは英語、日本語、简体中文の固定文言で、未公開、容量確保、新規確認codeなど必要な次動作を示す。partialの物理削除は別の明示discard操作として実装し、この分類だけで自動削除しない。

## ADR-065: Partial discard is explicit, receiver-only, and manifest-bound

Status: Accepted (2026-08-31)

Nearbyの受信partialは失敗分類だけで自動削除しない。受信側UIが失敗状態で、secure taskが停止済みの場合だけ確認dialogからdiscardできる。送信側には削除対象がないため同操作を出さず、既存Approvalを閉じて新しい確認codeの準備へ戻す。

native discardはUI由来pathを受け取らず、Prepared Approvalのtransfer IDからmanaged `.incomplete/peer-transfer/{id}.part`とledgerを組み立てる。ID形式、canonical managed directory、symlink、ledger schema、ledger内transfer ID、Media Indexが非Finalizedであることを検証する。削除対象はpart、resume ledger、対応するIncomplete／Failed Media manifestだけで、Original送信元とFinalized Mediaを削除しない。

discard確定は復元不能なため、「復旧用に保持」を既定の離脱操作として残し、「途中データを破棄」を別buttonにする。削除後はPrepared Approvalと診断errorを消去する。保持期限や一括cleanupは別policyとして定義し、この操作を自動実行しない。
