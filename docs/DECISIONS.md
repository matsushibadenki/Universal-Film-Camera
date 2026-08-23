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
