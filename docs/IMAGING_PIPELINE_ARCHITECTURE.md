# Universal Imaging Pipeline Architecture

Version: 0.2  
更新日: 2026-08-23
実装: `crates/imaging-core`

## 目的

本プロジェクトの上位概念を`Film Engine`から`Imaging Pipeline`へ拡張する。Film Engineは廃止せず、フィルム感光、色素濃度、粒状性、ハレーション、プリントフィルムを再現する専門レンダラーとしてPipeline内に保持する。

Imaging Pipelineは、実際の機材とシミュレーションの両方について、次の全工程を再現可能なデータとして記述する。

```text
Scene
  → Camera Body / Exposure
  → Lens
  → Capture Medium (Film | Digital Sensor)
  → Development (Chemical | Digital RAW)
  → Print / Digital Intermediate / Output Transform
  → Display
```

物理的な光路ではレンズが撮像媒体より先にある。このため正規の物理モデルは`SceneLight → Camera Body/Exposure → Lens → Film/Digital Sensor`とする。Camera nodeは本体profile、シャッター時間、露出補正を持ち、実際のDigital SensorはLens後のCapture Mediumとして表す。一方、すでにデジタルカメラで撮影された素材へ仮想レンズやフィルムを適用する場合は、入力をscene-linearへ再構成した後の`Emulation` Pipelineとして区別する。

## Film Engineとの関係

```text
Imaging Pipeline（工程、接続、provenance、profile参照）
├── Lens Renderer
├── Digital Sensor Renderer
├── Film Engine
│   ├── Sensitometry / Dye Density
│   ├── Chemical Development
│   ├── Grain / Halation / MTF
│   └── Print Film
├── Digital RAW Developer
├── Color Management / Output Transform
└── Display Model
```

`imaging-core`は記述と検証のみを担当し、画素処理を実装しない。`film-core`、将来の`lens-core`、`sensor-core`、`display-core`とCPU/GPU rendererが各ノードを実行する。

数値単位、標準working space、Profile envelope、Still／Video asset lifecycle、性能予算、適合試験の規範契約は [`Universal Film & Color Imaging Engine.md`](Universal%20Film%20%26%20Color%20Imaging%20Engine.md) Version 0.2の58–65章を正本とする。

## 信号領域

単なるノード名の並びでは、Filmの後へDigital RAW現像を接続するような誤りを検出できない。各ノードは入力と出力の`SignalDomain`を持つ。

| Domain | 意味 | 代表的な値 |
|---|---|---|
| `scene_light` | シーンからレンズへ入る光 | spectral radiance / scene RGB |
| `optical_image` | レンズを通過し像面へ到達した光 | irradiance、PSF適用後 |
| `film_latent_image` | 露光済み未現像フィルム | layer exposure / latent response |
| `film_density` | 現像後のフィルム濃度 | RGB dye density / silver density |
| `sensor_raw` | デジタルセンサー出力 | Bayer/X-Trans、black level、gain |
| `scene_linear` | 現像・正規化済みscene-linear画像 | ACEScg等 |
| `display_linear` | display rendering後の線形出力 | display primaries、linear light |
| `display_encoded` | 実際に表示／エンコードできる信号 | sRGB、Rec.709、PQ、HLG |

接続時に前ノードの出力domainと次ノードの入力domainが一致しなければ`PipelineError::DomainMismatch`になる。

## ノードの由来

各ノードには`NodeRole`を保存する。

- `observed`: 実機、実レンズ、実センサー、実測displayなど、入力素材にすでに作用している特性
- `simulated`: 仮想レンズ、仮想フィルム、仮想現像、creative printなど、レンダラーが加える特性
- `transform`: RAW現像、色空間変換、output transformなどの信号変換

これにより「iPhoneの実センサーで撮影した映像に仮想35mmレンズと仮想ネガを適用した」といった履歴を、すべて実機で撮影した場合と区別できる。

## 正規Pipeline

### フィルム撮影

```text
SceneLight
 → Camera Body / Shutter
 → Lens
 → Film Capture Medium
 → FilmLatentImage
 → Chemical Development
 → FilmDensity
 → Photochemical Print
 → DisplayLinear
 → Display
 → DisplayEncoded
```

### デジタル撮影

```text
SceneLight
 → Camera Body / Shutter
 → Lens
 → Digital Sensor Capture Medium
 → SensorRaw
 → Digital RAW Development
 → SceneLinear
 → Output Transform
 → DisplayLinear
 → Display
 → DisplayEncoded
```

### デジタル素材へのFilm Emulation

実カメラのレンズ／センサー特性をsource metadataとして保持し、RAW decode/input transformで一度`scene_linear`へ戻す。その後にFilm Engine用のemulation subgraphを実行する。実装済みの`virtual_exposure` nodeがscene-linear ACEScgを校正済みRGB `log10(lux·s)`へ対応付け、`optical_image` domainとしてFilm captureへ渡す。scene-linearを`scene_light`へ暗黙に読み替えてはいけない。数式、black floor、負値方針、科学的な適用範囲は [`VIRTUAL_EXPOSURE_ADAPTER.md`](VIRTUAL_EXPOSURE_ADAPTER.md) を正本とする。

## Profile設計

ノードにメーカー名や固定係数をハードコードせず、`profile_id`で外部profileを参照する。

```text
profiles/
├── cameras/       # camera body、metering、shutter、provenance
├── lenses/        # focal length、T-stop、distortion、MTF、PSF、flare
├── sensors/       # CFA、spectral response、noise、gain、black/white level
├── films/         # sensitometry、spectral response、dye density、grain
├── development/   # ECN-2/C-41/E-6/RAW developer、temperature、push/pull
├── prints/        # print film、paper、DI、tone reproduction
├── displays/      # primaries、EOTF、peak black/white、surround
└── pipelines/     # 上記profileを結ぶversioned recipe
```

Profile共通metadataには最低限、`id`、`schema_version`、`profile_version`、`kind`、`manufacturer`、`model`、`license`、`created_at`、`provenance`、`data`を持たせる。測定sourceとqualityは`provenance`へ格納し、実測、メーカー公称、推定、合成を混同しない。正本は [`PROFILE_SCHEMA_AND_LOADER.md`](PROFILE_SCHEMA_AND_LOADER.md) とJSON Schemaである。

## Pipelineの不変条件

現在の`schema_version = 1`では次を検証する。

1. 有効ノードが1つ以上ある
2. 最初の有効ノードが`source`である
3. sourceは1つだけである
4. 全node IDが空でなく一意である
5. 有効ノード間のSignalDomainが一致する
6. 最終domainが`display_encoded`である

将来DAG化する場合も、各edgeで同じdomain検証を行う。音声、depth、motion vector、camera metadataは画像信号と別portにし、RGB domainへ混在させない。

## 実装ロードマップ

- [Done] `imaging-core` crate、Camera/Exposure node、node role、signal domain、film/digital分岐を定義
- [Done] 完全なFilm chainとDigital chainのvalidation testを追加
- [Done] Film/Digitalのversioned JSON Pipeline例とdeserialize検証を追加
- [Done] Tauriからschema versionと対応domainを取得できるcommandを追加
- [Done] ACEScgを標準内部計算space、ACES2065-1をinterchange spaceとして規定
- [Done] Profile／Asset／性能／conformanceのVersion 0.2規範契約を追加
- [Done] Profile共通metadata、JSON Schema、Rust loader、Catalog参照検証を実装
- [Done] Film Profile data Schema／typed payloadとsensitometry curve検証を実装
- [Done] Lens／Digital Sensor Profile Schema、typed payload、物理範囲／分光感度検証を実装
- [Done] `scene_linear → virtual_exposure` adapter、Film emulation Pipeline例、数式fixtureを実装
- [Done] CPU Reference executorでvirtual exposureとRGB sensitometryを実画素bufferへ接続
- [Done] Development／Print／Display／Output Transformのtyped payload、Schema、synthetic例を実装
- [Done] directory loader、Profile closure解決、content hash付きrender snapshotを実装
- [Done] CPU Referenceへnormal Development／matrix output transformとgolden fixtureを追加
- [Done] explicit synthetic Print responseをFilm Density→Display Linearへ接続
- [Done] explicit major-step schema migration registryを追加
- [Later] measured Print dataset確定後にresponseを追加
- [Later] Lens、Sensor、Development、Print、Displayを個別crateへ分離
- [Later] linear sequenceをtyped-port DAGへ拡張
- [Later] wgpu schedulerでnode fusion、texture lifetime、zero-copyを最適化

## 命名方針

- 製品全体: `Universal Imaging Pipeline`
- カメラアプリ: 当面`Universal Film Camera`を維持するが、正式名称決定時に再検討
- `film-core`: Film/Print固有処理
- `imaging-core`: Pipelineの記述、接続検証、versioning
- `media-core`: 画像／動画frameと色メタデータ
- `camera-core`: OS cameraのdevice、capability、session、capture lifecycle
