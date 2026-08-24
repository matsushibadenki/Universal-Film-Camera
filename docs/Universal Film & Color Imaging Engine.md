# Universal Film & Color Imaging Engine
## 汎用フィルム・カラーエミュレーション基盤 設計仕様書

> **2026-08-20 Version 0.2:** 本仕様の正本を`Universal Imaging Pipeline`へ拡張した。Film EngineはPipeline内の専門rendererである。レンズ、Film/Digital Sensor、現像、プリント、ディスプレイの詳細なsignal-domain規則と現行実装は [`IMAGING_PIPELINE_ARCHITECTURE.md`](IMAGING_PIPELINE_ARCHITECTURE.md) を併読する。本書と補助文書が矛盾する場合は、本書のVersion 0.2規範契約と実装済みRust型を優先する。

Version: 0.2  
Status: Active design specification  
Updated: 2026-08-23
Target: macOS / Windows / Linux / iOS / Android  
Core Language: Rust

Roadmap表記:

- `[Done]`: implemented in the current codebase
- `[Next]`: high-priority unfinished work
- `[Later]`: planned, but not the closest next step

---

## 1. 目的

本システムは、Kodak、Fujifilm等の写真・映画用フィルムの色再現特性をソフトウェア上で再現する、アプリケーション非依存の映像処理エンジンである。

本エンジンは最低でも以下の2種類のアプリケーションから共通利用できることを目的とする。

1. AI動画制作・映像制作コンテキスト管理ソフト
2. リアルタイム・ビデオカメラソフト

さらに将来的には、

- 動画編集ソフト
- RAW現像ソフト
- 写真アプリ
- DAW付属映像機能
- AI動画生成
- AI画像生成
- VFX
- バーチャルカメラ
- ストリーミング
- Webアプリ
- CLIバッチ変換
- 外部プラグイン

から同一エンジンを利用できる設計とする。

フィルム再現処理をUI、カメラAPI、動画コーデック等から完全に分離する。

---

# 2. 基本設計思想

システム全体を、

```text
Application
    ↓
Media / Camera Adapter
    ↓
Universal Image Representation
    ↓
Color Management
    ↓
Film Simulation Engine
    ↓
Creative Grade
    ↓
Display / Encode / AI Metadata
```

という構造にする。

最重要原則は、

```text
Camera ≠ Film Engine
Video Editor ≠ Film Engine
AI Software ≠ Film Engine
```

とすることである。

Film Engineは入力画像の出所を知らない。

入力が、

```text
iPhone Camera
Sony Camera
ARRI LogC
RED Log
ProRes
H.265
EXR
AI generated image
PNG
JPEG
RAW
```

のどれであっても、入力を共通内部色空間へ変換してから処理する。

---

# 3. 全体アーキテクチャ

推奨構成：

```text
film-platform/
│
├── crates/
│   │
│   ├── film-core/
│   ├── film-color/
│   ├── film-spectral/
│   ├── film-sensitometry/
│   ├── film-grain/
│   ├── film-halation/
│   ├── film-optics/
│   ├── film-gpu/
│   ├── film-profiles/
│   ├── film-ocio/
│   ├── media-core/
│   ├── camera-core/
│   ├── camera-apple/
│   ├── camera-windows/
│   ├── camera-android/
│   ├── camera-linux/
│   ├── video-codec/
│   └── film-ffi/
│
├── apps/
│   ├── camera/
│   ├── ai-video-context/
│   ├── desktop-editor/
│   └── cli/
│
├── profiles/
│   ├── kodak/
│   ├── fujifilm/
│   ├── custom/
│   └── synthetic/
│
├── shaders/
│   ├── exposure.wgsl
│   ├── color_transform.wgsl
│   ├── sensitometry.wgsl
│   ├── dye_density.wgsl
│   ├── halation.wgsl
│   ├── grain.wgsl
│   ├── optics.wgsl
│   └── output.wgsl
│
└── tests/
```

---

# 4. Rust Core

画像処理の中心部分はRustで実装する。

理由は、

- OS非依存
- メモリ安全性
- SIMD利用
- ネイティブ性能
- WebAssemblyへの展開
- C ABI公開
- GPU APIとの統合
- モバイル展開

を同時に行いやすいためである。

GPU抽象化には原則として`wgpu`を使用する。

wgpuはRustからMetal、Vulkan、D3D12等へ展開できるクロスプラットフォームGPU APIである。

概念構造：

```text
film-core
       │
       ├── CPU reference renderer
       │
       └── GPU renderer
               ↓
              wgpu
        ┌──────┼────────┐
      Metal  D3D12   Vulkan
```

CPU版を必ず持たせる。

GPU版だけにすると、

- テスト
- 数値検証
- サーバー
- 互換性
- 将来のバックエンド変更

が困難になるためである。

---

# 5. 内部色表現

内部処理を8bit RGBで行ってはいけない。

最低：

```text
RGBA16F
```

高精度モード：

```text
RGBA32F
```

を使用する。

Version 0.2の標準内部計算色空間は、

```text
scene-linear ACEScg (AP1 primaries)
```

とする。ACES2065-1（AP0）はprofile交換、reference asset、archive用のinterchange spaceとして使用できるが、renderer内部の既定値にはしない。

```text
Input encoding
→ explicit input transform
→ scene-linear ACEScg
→ Imaging Pipeline
→ explicit output/display transform
```

変換なしにprimaries、white point、transfer functionを読み替えてはいけない。custom working spaceを使用する場合はprofile ID、primaries、white point、変換versionをprojectへ保存する。

ACESは映画、テレビ等の制作工程全体を対象にした色管理規格であり、ACES 2はエンドツーエンドの色管理を改善した第2世代フレームワークである。

ただし、

```text
FilmEngine = ACES依存
```

にはしない。

内部APIでは、

```rust
enum WorkingColorSpace {
    Aces2065,
    AcesCg,
    LinearRec2020,
    LinearP3,
    Custom,
}
```

のように抽象化する。

---

# 6. カラーマネジメント

OpenColorIOとの互換性を持たせる。

OpenColorIOは映像/VFX向けのカラーマネジメント基盤で、ACESにも対応する。2026年のVFX Reference PlatformにはOCIO 2.5が採用されている。

処理：

```text
Input
 ↓
Input Color Space
 ↓
OCIO / Internal Transform
 ↓
Scene Linear Working Space
 ↓
Film Engine
 ↓
Output Transform
 ↓
Display
```

カメラ入力例：

```text
Apple Log
 ↓
Linear
 ↓
ACEScg
 ↓
Film
```

AI画像：

```text
sRGB
 ↓
linearization
 ↓
ACEScg
 ↓
Film
```

映画素材：

```text
ARRI LogC
 ↓
ACEScg
 ↓
Film
```

とする。

---

# 7. Film Profile

フィルムの特性はコードにハードコードしない。

外部データとして定義する。

例：

```text
profiles/kodak/vision3_500t_5219/
```

内容：

```text
profile.json
sensitometry.csv
spectral_sensitivity.csv
dye_density.csv
mtf.csv
grain.json
halation.json
metadata.json
```

---

# 8. FilmProfileデータモデル

基本モデル：

```rust
FilmProfile {
    id,
    manufacturer,
    family,
    stock_name,
    stock_code,

    film_type,

    nominal_iso,
    native_color_temperature,

    sensitometry,
    spectral_response,
    dye_density,

    grain,
    mtf,
    halation,

    processing,

    source_metadata
}
```

film_type：

```text
ColorNegative
ColorPositive
Slide
BlackAndWhite
Intermediate
Print
Synthetic
```

---

# 9. Sensitometry

最重要データの一つ。

フィルムの入力露光量と現像後濃度の関係を、

\[
D_c=f_c(\log E)
\]

として表す。

RGBについて、

\[
D_R=f_R(\log E)
\]

\[
D_G=f_G(\log E)
\]

\[
D_B=f_B(\log E)
\]

を持つ。

CSV：

```text
log_exposure,red,green,blue
-4.0,0.10,0.09,0.08
-3.9,0.11,0.10,0.09
...
```

数値計算は、

```text
Cubic spline
```

または、

```text
monotonic cubic interpolation
```

を使用する。

単純な多項式fitだけには依存しない。

フィルムのToeとShoulderを正確に保持するためである。

---

# 10. Spectral Sensitivity

可能なフィルムでは波長感度を記録する。

\[
S_R(\lambda)
\]

\[
S_G(\lambda)
\]

\[
S_B(\lambda)
\]

例：

```text
wavelength_nm,red,green,blue

380,...
390,...
400,...
...
700,...
```

これによって単純RGB変換より物理的に正しいフィルム応答を計算できる。

---

# 11. 二段階Film Engine

計算負荷と精度の要求が大きく異なるため、

```text
Fast Mode
Physical Mode
```

を持たせる。

## Fast Mode

リアルタイムカメラ向け。

```text
RGB
 ↓
3D color transform
 ↓
Sensitometric curve
 ↓
Color density transform
 ↓
Halation
 ↓
Grain
```

60fps / 120fpsを狙う。

## Physical Mode

オフラインレンダリング、AI映像制作、マスター生成向け。

```text
RGB
 ↓
spectral reconstruction
 ↓
film spectral sensitivity
 ↓
layer exposure
 ↓
density response
 ↓
dye spectral density
 ↓
print film
 ↓
scanner / display
```

として処理する。

---

# 12. Spectral Engine

Physical Modeでは、

```text
RGB → Spectrum
```

を近似する。

入力RGBだけから唯一のスペクトルを復元することは数学的にはできないため、これは推定問題として明示する。

インターフェース：

```rust
trait SpectralReconstructor {
    fn reconstruct(
        rgb: LinearRgb
    ) -> Spectrum;
}
```

将来的に、

```text
Smits
basis spectrum
PCA
neural spectral reconstruction
camera-specific spectral model
```

等を交換可能にする。

---

# 13. ネガフィルムモデル

カラーネガの場合、

```text
Scene Light
 ↓
Film exposure
 ↓
Negative dye density
```

を明示的に分ける。

ここでいきなり「最終画面色」を作らない。

```text
NegativeFilm
```

と、

```text
PrintFilm
```

を別モデルにする。

---

# 14. Print Film

映画フィルムについては、

```text
Camera Negative
 ↓
Print Film
```

という処理を独立して持つ。

これにより、

```text
Vision3 500T
+
2383 Print Film
```

のような組み合わせを表現できる。

FilmProfileとPrintProfileを分離する。

---

# 15. Grain Engine

グレインを静的画像テクスチャとして貼り付けない。

グレイン：

\[
G=F(x,y,t,L,C,stock)
\]

とする。

つまり、

- 座標
- 時間
- 輝度
- 色チャンネル
- フィルム

によって変化する。

重要：

```text
Grain != Additive RGB Noise
```

とする。

フレームごとに粒子状態を変化させる。

ただし完全ランダムにするとデジタルノイズに見えるので空間的・時間的相関を持たせる。

---

# 16. Grain Resolution Independence

グレインサイズを単純な「pixel size」で指定してはいけない。

基準を、

```text
film physical size
```

にする。

例えば、

```text
8 mm
16 mm
35 mm
65 mm
```

で粒状感が変わる設計にする。

入力解像度が、

```text
1080p
4K
8K
```

に変わってもフィルム粒子の物理サイズが不自然に変わらないようにする。

---

# 17. Halation Engine

Halationは、

```text
Threshold
→ wavelength-dependent diffusion
→ re-exposure
```

としてモデル化する。

単なる、

```text
highlight blur + red
```

にはしない。

最低でも、

```text
Red diffusion radius
Green diffusion radius
Blue diffusion radius
```

を独立させる。

Physical Modeではスペクトル依存PSFへ拡張できる設計にする。

---

# 18. Optical Model

Film Engineとは別モジュールとして、

```text
film-optics
```

を設ける。

処理：

```text
Lens softness
Bloom
Diffraction
Chromatic aberration
Vignetting
Diffusion filter
Gate weave
Focus breathing
Lens distortion
```

FilmStockとは独立させる。

これによって、

```text
Kodak Vision3
+
Cooke Lens
+
Black Pro-Mist
```

のような構成が可能になる。

---

# 19. Image Pipeline

Version 0.2では、物理撮影と既存素材へのemulationを区別する。正規の物理撮影順序は次とする。

```text
Scene Light
→ Camera Body / Exposure
→ Lens
→ Capture Medium (Film | Digital Sensor)
→ Development (Chemical | Digital RAW)
→ Print / Digital Intermediate / Output Transform
→ Display
```

既存のデジタル画像、動画、AI生成画像へFilmを適用する場合は、入力encodingからscene-linear ACEScgへ明示変換した後、`scene_linear → virtual_exposure` adapterを通してFilm emulation subgraphへ接続する。scene-linearをscene lightやfilm exposureへ暗黙に読み替えてはいけない。

RGB adapterはチャンネル`c`について`log10(H_c) = log10(H_ref) + log10(max(C_c, C_floor) / C_ref) + EV × log10(2)`を使用する。`C_ref`は通常0.18だが、18% grayだけでは絶対的なlux·sは決まらない。`log10(H_ref)`をfilm／metering／測定fixtureごとの校正値として保存し、black floorと負値方針も明示する。これはRGB emulation用の近似であり、分光放射量を積分するSpectral／Physical Modeの代替ではない。詳細契約は [`VIRTUAL_EXPOSURE_ADAPTER.md`](VIRTUAL_EXPOSURE_ADAPTER.md) を正本とする。

各nodeは`SignalDomain`を宣言し、隣接nodeのoutputとinputが一致しないPipelineは実行前に拒否する。現在の正規domainは`scene_light`、`optical_image`、`film_latent_image`、`film_density`、`sensor_raw`、`scene_linear`、`display_linear`、`display_encoded`である。

Film固有の標準処理順序は、正規Pipeline内のFilm capture／development／print subgraphとして維持する。

```text
Virtual Exposure
→ Film Negative Response
→ Dye Density
→ Halation / Grain / MTF
→ Print Film
→ Display Linear
```

---

# 20. Node Graph

内部では処理をNode Graphとして表す。

```text
Input
 ↓
Exposure
 ↓
White Balance
 ↓
Film
 ↓
Halation
 ↓
Grain
 ↓
Print
 ↓
Output
```

ノード：

```rust
trait ImageNode {
    fn prepare(...);
    fn process(...);
}
```

これによりAI動画制作ソフト側では、

```text
AI Image
 ↓
Film
 ↓
Grade
```

だけ使用することもできる。

---

# 21. Camera Abstraction

Camera APIをfilm-coreへ入れてはいけない。

```rust
trait CameraBackend
```

で抽象化する。

概念：

```rust
trait CameraBackend {

    fn devices() -> Vec<CameraDevice>;

    fn open(
        device: CameraDevice,
        config: CameraConfig
    ) -> CameraStream;

}
```

出力：

```rust
VideoFrame
```

に統一する。

---

# 22. OS別Camera Backend

Apple：

```text
AVFoundation
```

を利用する。

AVFoundationはmacOS/iOSでカメラ、マイク、外部キャプチャデバイス等を扱える。

Android：

```text
Camera2 NDK
```

を中心にネイティブ層を構成する。Android NDKではCamera2系のネイティブCamera APIが公開されている。

Windows：

```text
Media Foundation
```

を使用する。Microsoftは新規映像キャプチャ実装についてMedia Foundation系APIを提供している。

Linux：

```text
V4L2
```

を利用する。V4L2はLinuxの標準的な映像キャプチャ・カメラ制御APIである。

---

# 23. VideoFrame

全プラットフォームで、

```rust
struct VideoFrame {
    timestamp,
    duration,

    width,
    height,

    pixel_format,
    color_space,
    transfer_function,
    color_primaries,

    gpu_texture,
    cpu_buffer,

    camera_metadata,
}
```

に変換する。

重要なのは、

```text
GPU textureを可能な限りCPUに戻さない
```

ことである。

---

# 24. Zero Copy Pipeline

リアルタイムCamera Modeでは、

```text
Camera
 ↓
GPU Texture
 ↓
Film GPU
 ↓
Preview
```

を目標とする。

禁止：

```text
Camera
 ↓
CPU RAM
 ↓
GPU
 ↓
CPU
 ↓
GPU
```

可能な限り、

```text
GPU → GPU → GPU
```

とする。

---

# 25. Camera Controls

統一CameraConfig：

```text
resolution
fps

ISO
shutter
white balance
focus

exposure compensation

HDR
LOG

lens

stabilization
```

OSによって利用できない項目はCapabilityとして判定する。

```rust
CameraCapabilities
```

を必ず取得してからUIを生成する。

---

# 26. Video Encoding

エンコード層はFilm Engineとは分離する。

共通インターフェース：

```rust
trait VideoEncoder
```

対応候補：

```text
H.264
H.265/HEVC
AV1
ProRes
Image Sequence
EXR
PNG
```

コンテナ：

```text
MOV
MP4
MKV
```

FFmpeg連携を可能にする。

FFmpegのlibavformatは音声・映像・字幕ストリームのmux/demuxを提供する。

ただしApple等では、

```text
VideoToolbox
AVFoundation
```

によるネイティブHardware Encoderを優先できるようにする。

---

# 27. AI動画制作との統合

AI動画制作側ではFilm Engineを「映像加工」だけに使用しない。

Film Profile自体を、

```text
Creative Context
```

として利用する。

例えばプロジェクトコンテキスト：

```json
{
    "cinematography": {
        "camera": "ARRI Alexa 35",
        "lens": "Cooke S4",
        "film_emulation": "Kodak Vision3 500T",
        "print_stock": "Kodak 2383",
        "exposure": -0.3,
        "white_balance": 3200
    }
}
```

とする。

---

# 28. AI Context Model

AI映像生成では、

```text
Film Look
```

だけでは足りない。

以下を独立したコンテキストにする。

```text
Project
 ├── Story
 ├── Scene
 ├── Shot
 ├── Camera
 ├── Lens
 ├── Lighting
 ├── Film
 ├── Processing
 ├── Color Grade
 └── Output
```

---

# 29. Shot Context

各ショット：

```rust
ShotContext {
    shot_id,

    scene_id,

    camera,
    lens,

    focal_length,

    aperture,
    shutter,

    iso,

    white_balance,

    lighting,

    film_profile,

    print_profile,

    grade,

    references
}
```

を持つ。

---

# 30. AI生成と物理Film Engineの役割分離

AIへ、

```text
Kodak Vision3 500T look
```

とだけ指示して完成画像を生成させることを標準方式にしない。

推奨：

```text
AI generation
 ↓
scene-referred / neutral image
 ↓
Film Engine
 ↓
Final look
```

とする。

理由：

同一作品中のショット間でFilm特性を固定できるためである。

AIモデルを変更しても、

```text
Film Look
```

を共通化できる。

---

# 31. Film Profile Metadata

AIが理解できる意味情報も保持する。

例：

```json
{
    "id": "kodak-vision3-500t-5219",

    "semantic": {
        "contrast": "medium-low",
        "grain": "medium",
        "highlight_rolloff": "soft",
        "saturation": "moderate",
        "skin_tone": "natural",
        "temperature": "tungsten"
    }
}
```

ただし、

```text
semantic
```

と、

```text
physical measurement
```

を混ぜない。

---

# 32. Source Provenance

非常に重要。

フィルムデータそれぞれに、

```text
source
source_type
source_url
manufacturer
document_name
document_version
measurement_method
digitized_by
digitized_date
confidence
```

を記録する。

例えば：

```json
{
    "source_type": "manufacturer_datasheet",
    "manufacturer": "Kodak",
    "confidence": "high"
}
```

とする。

メーカー実測値と推定値を明確に区別する。

---

# 33. Measurement Quality

各データ点に、

```text
official
measured
digitized
estimated
synthetic
```

を設定できるようにする。

これにより、

```text
Kodak公式Characteristic Curve

＋

独自測定Halation
```

のようなProfileを安全に扱える。

---

# 34. Versioning

FilmProfileは変更可能なため、

```text
profileVersion
engineVersion
```

を映像ファイル・Projectに記録する。

例えば：

```json
{
    "filmProfile": "kodak-vision3-500t",
    "filmProfileVersion": "1.3.2",
    "engineVersion": "0.8.1"
}
```

とする。

10年後でも同じ映像を再レンダリングできることを目標とする。

---

# 35. Deterministic Rendering

AI動画制作には特に重要。

Grain等の乱数処理に、

```text
seed
```

を設定する。

```rust
FilmRenderConfig {
    seed: u64
}
```

同じ、

```text
frame
profile
parameters
seed
```

なら完全に同じ画像を生成することを原則とする。

---

# 36. FilmRecipe

ユーザー設定をFilmProfile自体へ書き込まない。

FilmProfile：

```text
物理フィルム
```

FilmRecipe：

```text
ユーザーの撮影/現像設定
```

として分離する。

例：

```json
{
    "profile": "kodak-vision3-500t",

    "exposure": 0.7,

    "development": {
        "push": 1
    },

    "halation": 0.8,
    "grain": 1.2
}
```

---

# 37. Creative Preset

さらに上位に、

```text
CreativePreset
```

を置く。

```text
Profile
 ↓
Recipe
 ↓
Preset
```

とする。

例えば、

```text
FilmProfile
Kodak Vision3 500T

FilmRecipe
+1 push

CreativePreset
1970s Tokyo Night
```

という関係にする。

---

# 38. API

基本API：

```text
FilmEngine.create()
FilmEngine.loadProfile()
FilmEngine.setRecipe()

FilmEngine.processFrame()

FilmEngine.processImage()
FilmEngine.processVideo()

FilmEngine.renderPreview()
```

カメラ：

```text
Camera.open()
Camera.start()

frame
 ↓
FilmEngine.processFrame(frame)
```

---

# 39. FFI

Rust以外から利用できるように、

```text
C ABI
```

を必須とする。

これにより、

```text
Swift
Kotlin
C++
Python
JavaScript
Dart
Unity
Unreal Engine
```

へ展開できる。

---

# 40. Web / Tauri

デスクトップのAI動画制作ソフトについては、

```text
Tauri 2
+
React/Web UI
+
Rust Core
```

を利用できる。

ただしリアルタイム映像は、

```text
WebView
```

へ画像配列を送る設計にはしない。

重い映像処理はRust/GPU側で完結させる。

UIには、

```text
parameters
metadata
thumbnail
state
```

だけを渡す。

---

# 41. Camera App UI

Camera Appは、

```text
Native Camera Preview
+
Rust Film Engine
```

を中心にする。

UI技術をFilm Engineから分離することで、

```text
SwiftUI
Kotlin UI
React/Tauri
Native Rust UI
```

のどれでも利用可能にする。

---

# 42. Performance Target

最低目標：

```text
1920×1080
60 fps
Film emulation realtime
```

標準目標：

```text
3840×2160
60 fps
```

高性能端末：

```text
4K 120 fps
```

を狙う。

---

# 43. Quality Levels

```text
Preview
Realtime
High
Reference
```

の4段階とする。

Preview：

```text
low resolution
simplified grain
simplified halation
```

Realtime：

```text
full resolution
GPU approximation
```

High：

```text
high-quality spatial processing
```

Reference：

```text
spectral mode
maximum precision
CPU/GPU
```

---

# 44. GPU Shader Pipeline

推奨：

```text
Shader 01
decode / normalization

Shader 02
input color transform

Shader 03
exposure

Shader 04
film sensitometry

Shader 05
dye transform

Shader 06
halation

Shader 07
grain

Shader 08
print film

Shader 09
creative grading

Shader 10
output transform
```

将来的には複数処理を1 compute shaderに融合してメモリ帯域を削減する。

---

# 45. LUTの位置付け

LUTは使用するが、

```text
Film Simulation = LUT
```

とはしない。

LUTは高速近似キャッシュとして利用する。

例えばPhysical Modelから、

```text
65 × 65 × 65
```

3D LUTを生成する。

Realtime Mode：

```text
Physical model
 ↓
LUT baking
 ↓
GPU LUT
```

とできる。

---

# 46. Film LUT Compiler

独立機能として、

```text
Film LUT Compiler
```

を持つ。

入力：

```text
FilmProfile
FilmRecipe
```

出力：

```text
.cube
.spi3d
3D texture
internal binary LUT
```

これにより外部ソフトでも利用できる。

---

# 47. テスト

最重要テスト：

### Numerical Test

CPUとGPUの差：

```text
ΔE
```

で評価する。

### Color Checker

標準カラーチャート入力に対する結果を保存する。

### Exposure Sweep

```text
-10 EV
...
+10 EV
```

のグレースケールテスト。

### Spectral Test

単波長入力：

```text
380–780 nm
```

を評価。

### Temporal Test

動画Grain、Halationの時間変動を検証。

---

# 48. Reference Renderer

CPUによる、

```text
film-reference
```

を作る。

高速性を要求しない。

ここを、

```text
ground truth
```

としてGPU実装を比較する。

GPU最適化による画質変化を検出するために必須とする。

---

# 49. Film Profile Editor

将来的にGUI Profile Editorを作る。

```text
Characteristic Curve
Spectral Sensitivity
Dye Density
MTF
Grain
Halation
```

を可視化する。

メーカー資料PDFからデジタイズしたデータも編集できるようにする。

---

# 50. Profile Database

Film Engine本体とProfile DBは分離する。

```text
film-engine
```

と、

```text
film-database
```

を別リポジトリにしてもよい。

これにより新しいフィルムを、

```text
engine updateなし
```

で追加できる。

---

# 51. ブランド依存性

コード上では、

```text
Kodak
Fujifilm
```

を特別扱いしない。

すべて：

```text
FilmProfile
```

とする。

したがって、

```text
Kodak
Fujifilm
Agfa
Ilford
Cinestill
ORWO
Ferrania
Lomography
Custom
Synthetic
```

を同じ形式で扱える。

---

# 52. Synthetic Film

存在しないフィルムも作成可能にする。

例えば、

```text
Kodak-like highlight response

+

Fujifilm-like color response

+

65mm grain
```

といったものを作る。

これを、

```text
SyntheticProfile
```

として明確に実在フィルムと区別する。

---

# 53. AIとの将来的統合

FilmProfileをAIへ直接渡せるようにする。

AI入力：

```text
FilmProfile
Lighting
Lens
Camera
Scene
```

から、

```text
recommended exposure
white balance
lighting
camera settings
prompt context
```

を生成できる。

さらに、

```text
Reference Image
 ↓
Profile Estimator AI
 ↓
Estimated FilmRecipe
```

も実装可能とする。

ただしAI推定値は、

```text
estimated
```

としてメーカー実測データとは区別する。

---

# 54. AI動画プロジェクトの再現性

プロジェクトファイルには、

```text
AI model
AI model version
seed
prompt
camera
lens
film profile
film profile version
film recipe
grade
engine version
```

を保存する。

これによって、

```text
映像そのもの
+
映像がどのように作られたか
```

を同時に保存する。

これはAI映像制作コンテキスト管理ソフトの中心概念にできる。

---

# 55. 最終システム

最終的な構造：

```text
                       UNIVERSAL VIDEO CORE
                               │
             ┌─────────────────┼─────────────────┐
             │                 │                 │
           Camera            Files              AI
             │                 │                 │
             └────────────┬────┴─────────────────┘
                          ↓
                    Scene Linear
                          ↓
                ┌───────────────────┐
                │   COLOR ENGINE    │
                └───────────────────┘
                          ↓
                ┌───────────────────┐
                │    FILM ENGINE    │
                │                   │
                │ Sensitometry      │
                │ Spectrum          │
                │ Dye Density       │
                │ Grain             │
                │ Halation          │
                │ MTF               │
                │ Print Film        │
                └───────────────────┘
                          ↓
                   Creative Grade
                          ↓
                     ACES / OCIO
                          ↓
          ┌───────────────┼───────────────┐
          ↓               ↓               ↓
       Display          Encoder        AI Context
```

---

# 56. 推奨実装順序

Version 0.2では実装順序を依存関係と現在地へ合わせる。Phase番号は完了順を強制せず、各項目のstatusを正本とする。

Phase 1 — 共通契約：

```text
[Done] media-core frame/color metadata boundary
[Done] imaging-core signal domains and Film/Digital pipeline validation
[Done] camera-core state/capability/session boundary
[Done] common profile metadata, JSON Schema, loader, and Catalog reference validation
[Done] scene_linear → virtual_exposure adapter and calibrated mapping fixture
```

Phase 2 — Reference renderer：

```text
[Done] film-core renderer boundary and FilmRecipe type
[Done] scene-linear exposure and PCHIP RGB sensitometry CPU executor
[Done] deterministic interpolation, extrapolation, negative-input, and alpha unit fixtures
[Done] golden exposure sweep and minimal matrix output transform fixture
[Done] explicit synthetic print response and display encoding
[Later] measured print response and calibrated ColorChecker fixture after dataset selection
[Later] spectral reference renderer
```

Phase 3 — Camera vertical slice：

```text
[Done] macOS AVFoundation native preview
[Done] JPEG still capture and H.264/AAC MOV recording
[Done] supported resolution/FPS enumeration and active format selection
[Next] still/video orientation, metadata, selected-format persistence
[Next] iOS/Android Tauri mobile initialization
[Later] Windows Media Foundation and Linux camera backend
```

Phase 4 — GPU and color management：

```text
[Next] wgpu scheduler and texture lifetime model
[Next] ACEScg input/output transforms and OCIO adapter
[Later] GPU Film Engine, LUT compiler, shader fusion
```

Phase 5 — Film detail：

```text
[Later] Halation
[Later] Grain
[Later] MTF and optical model
[Later] Print Film
```

Phase 6 — Tools and ecosystem：

```text
[Later] AI Video Context integration
[Later] CreativePreset and project reproducibility
[Later] Profile measurement tools and Film Profile Editor
[Later] plugin/FFI/CLI distribution
```

---

# 57. 最も重要な設計判断

本プロジェクトでは、

```text
「Kodak風フィルター」
```

を作るのではなく、

```text
Film Imaging Model
```

を作る。

つまり、

```text
Input RGB
→ LUT
→ Film Look
```

ではなく、

```text
Scene
→ Exposure
→ Film Response
→ Density
→ Dye
→ Grain
→ Optical Effects
→ Print
→ Display
```

をソフトウェア上の抽象モデルとして表現する。

同時にPhysical Modelそのものをリアルタイム処理へ強制せず、

```text
Physical Reference Model
        ↓
Fast Approximation
        ↓
GPU
```

という二層構造にする。

これにより、

```text
科学的再現性
映像制作上の自由度
リアルタイム性能
クロスプラットフォーム性
AIとの統合性
```

を同時に維持する。

この「Film Engine」を一つ作れば、

```text
Camera App
AI Video Production App
Video Editor
Photo Editor
VFX
Streaming
AI Generation
CLI
Plugin
```

のすべてから同じカラーサイエンスを共有できる。

本プロジェクトにおける最重要資産はUIではなく、

```text
Film Profile Database
+
Reference Film Model
+
GPU Film Renderer
```

の3点とする。

---

# 58. Version 0.2 規範ルール

本章以降は実装間の互換性を決める規範契約である。

- **MUST / 必須**: 満たさない実装は互換実装ではない
- **SHOULD / 推奨**: 明確なplatform制約がある場合だけ逸脱でき、理由を記録する
- **MAY / 任意**: 能力値で有無を公開したうえで実装を選べる

すべての公開データは`schema_version`をMUSTで持つ。未知のmajor schemaは拒否し、未知の追加fieldは同一major内では保持または無視できる。読み込み後に保存し直すeditorは、理解できないfieldを失わないことをSHOULDとする。

外部profile、pipeline、asset metadataの識別子は空でないUTF-8文字列とする。同一collection内で一意でなければならない。時刻はUTCのRFC 3339、durationとtimestampは単位をfield名または型で明示する。

JSONには`NaN`、正負のinfinity、暗黙の単位を保存してはいけない。数値が不明な場合は、schemaが許可する`null`を使用する。`0`を「不明」の代用にしてはいけない。

# 59. 数値・単位・色の共通契約

## 59.1 数値

| 値 | 標準単位／表現 | 規則 |
|---|---|---|
| exposure | EV、stops | `f32`またはJSON number。加算値 |
| shutter | seconds | 0より大きい有限値 |
| focal length | millimetres | 0より大きい有限値 |
| aperture | f-number | 0より大きい有限値。T-stopは別field |
| focus distance | metres | 0より大きい値。未知は`null` |
| wavelength | nanometres | profile内で単調増加 |
| density | log10 optical density | 測定条件とbaseをmetadataへ記録 |
| frame rate | rational number | integer表示と実値を分離 |
| timestamp | integer ticks + time base | 浮動小数秒だけを正本にしない |
| luminance | cd/m² (`nits`) | 0以上の有限値 |
| temperature | kelvin | 0より大きい整数 |

Curve sampleはx軸を単調増加にし、重複点を拒否する。Sensitometryの既定補間はmonotonic cubicとし、sample範囲外は端点の傾きを無制限に延長せず、profileが宣言した`clamp | linear | reject`のいずれかに従う。

## 59.2 色

Realtime／High／Reference rendererの標準working spaceはscene-linear ACEScgとする。RGBA16FはPreview／Realtimeの最低精度、RGBA32FはReferenceの正本とする。

- transfer functionを除去してからmatrix／chromatic adaptationを行う
- input primaries、white point、transfer、range、matrix coefficientsをmetadataとして保持する
- alphaは既定でstraight alphaとし、色変換、露出、Film responseの対象外とする
- premultiplied inputは処理前に安全にunpremultiplyし、出力要件に応じて再適用する
- negative scene-linear RGBはReferenceでは保持する。display output時のclamp／gamut mappingは明示nodeで行う
- HDRからSDR、SDRからHDRを暗黙に変換しない

custom color spaceには、primaries、white point、transfer function、matrixまたは外部transform ID、transform versionをMUSTで記録する。

# 60. Profile共通契約

Camera、Lens、Sensor、Film、Development、Print、Displayの全profileは次の共通envelopeを持つ。

```json
{
  "schema_version": 1,
  "profile_version": "1.0.0",
  "id": "org.example.profile-id",
  "kind": "film",
  "manufacturer": "Example",
  "model": "Example 500T",
  "license": "SPDX identifier or explicit license reference",
  "created_at": "2026-08-20T00:00:00Z",
  "provenance": {
    "quality": "official",
    "source_type": "manufacturer_datasheet",
    "source_reference": "document or dataset identifier",
    "measurement_method": null,
    "measured_by": null
  },
  "data": {}
}
```

`kind`は`camera | lens | digital_sensor | film | development | print | display | output_transform | synthetic`のいずれかとする。`profile_version`はsemantic versioning形式とし、測定値、処理結果、default解釈が変わる更新はminor以上、互換性を壊すschema変更はmajorを上げる。

`quality`は`official | measured | digitized | estimated | synthetic`のいずれかとする。異なるqualityのdataを混ぜる場合はfieldまたはdataset単位でprovenanceを上書きできなければならない。出典不明の測定値を`official`として扱ってはいけない。

共通Profile loaderはschema version、ID、kind、version、license、timestamp、provenance、参照profileの存在とkindを検証する。kind別loaderは有限数、単位、curve順序、補間／外挿規則を追加検証する。validation errorはJSON pathと理由を含める。

Film Profile v1では、`film_type`、`nominal_exposure_index`、`native_color_temperature_kelvin`、`sensitometry`を必須とする。Sensitometryは`log10_lux_seconds → log10_optical_density`、最低2 sample、strictな露光軸単調増加、非負のRGB densityをMUSTとする。JSON Schemaの正本は`docs/schemas/film-profile-v1.schema.json`である。

Lens Profile v1は焦点距離、F-number、最短撮影距離、image circleを明示単位で持つ。Digital Sensor Profile v1はactive pixels、物理寸法、CFA、bit depth、black／white level、ISO範囲、任意の360–830 nm分光感度を持つ。正本は`docs/schemas/lens-profile-v1.schema.json`と`docs/schemas/digital-sensor-profile-v1.schema.json`である。

# 61. Still／Video共通Asset Contract

スチルと動画は同じ`CapturedAsset` lifecycleを使い、UI上も同格に扱う。

```rust
struct CapturedAsset {
    schema_version: u32,
    id: String,
    media_type: MediaType,       // Still | Video
    state: AssetState,           // Incomplete | Finalized | Failed
    original: MediaResource,
    derivatives: Vec<MediaResource>,
    capture: CaptureMetadata,
    pipeline: Option<PipelineReference>,
    created_at_utc: String,
}
```

`original`はcamera／encoderが生成した変更前のasset、`derivatives`はImaging Pipeline処理、thumbnail、proxy、exportを表す。derivativeは親resource ID、pipeline ID、profile version、engine version、seedをMUSTで保持する。処理済みassetでoriginalを上書きしてはいけない。

## 61.1 Still

Still resourceは最低限、pixel width／height、encoded format、bit depth、orientation、embedded color descriptionを持つ。対応候補はJPEG、HEIF、RAW/DNG、PNG、EXRとし、実際の利用可否はcamera/output capabilityから生成する。

EXIF／XMP、capture timestamp、exposure time、ISO/EI、focal length、aperture、white balance、device IDは取得できる場合に保持する。取得できない値を推測して記録してはいけない。rotation済みpixelとorientation tagを二重適用しない。

## 61.2 Video

Video resourceはcontainer、video codec、audio codec、pixel dimensions、rational frame rate、variable-frame-rate可否、time base、duration、color metadata、audio channel layoutを持つ。各frameのtimestampは単調非減少、durationは正とする。audio/videoのclock sourceとstart offsetを記録する。

録画停止要求だけでassetを`Finalized`にしてはいけない。container writerまたはplatform delegateの完了、非空file、stream metadataの読出し後にのみ公開する。録画中のformat、color space、camera mode変更は、backendがseamless reconfigurationを明示対応しない限り拒否する。

## 61.3 保存

Still／Videoとも一時名またはincomplete directoryへ書き、flush／finalize成功後に完成assetへ原子的に移す。失敗assetはMedia一覧へ完成品として表示しない。保存先、空き容量、sandbox／library権限、cleanup policyはapplication layerが所有し、Film Engineへ持ち込まない。

# 62. Camera lifecycleとエラー契約

共通camera stateは最低限、`Idle`、`Authorizing`、`Previewing`、`Capturing`、`Recording`、`Stopping`、`Failed`を区別する。permission denied、device unavailable、device busy、format unsupported、storage full、encoder failure、device disconnectedを同じ「cameraなし」に潰してはいけない。

Camera capabilityはdevice全体の集合と、選択可能なformat単位の組合せを分離する。現在値はsession開始後のactive formatから取得する。最大対応値を現在値として表示してはいけない。

Preview frame、native texture、audio sampleをWebView IPCへ連続送信してはいけない。IPCは設定、状態、metadata、thumbnail、完了eventに限定する。session mutationはbackendごとのserial queueまたは同等の排他境界で直列化する。

# 63. 性能予算と受け入れ基準

最初の共通baselineは1080p60 Preview／Realtimeとし、4K60は標準目標、4K120は対応deviceのstretch goalとする。

| 指標 | 1080p60 baseline | 4K60 target |
|---|---:|---:|
| frame interval | 16.67 ms | 16.67 ms |
| Imaging Pipeline GPU p95 | 8.0 ms以下 | 12.0 ms以下 |
| capture-to-preview p95 | 50 ms以下 | 75 ms以下 |
| sustained dropped frames | 0.5%未満 | 1.0%未満 |
| full-frame CPU readback | 0回／frame | 0回／frame |
| preview queue depth | 2 frames以下 | 2 frames以下 |

測定は最低60秒の連続runで行い、最初の2秒をwarm-upとして除外する。device、OS、resolution、FPS、pixel format、quality level、thermal stateを記録する。未達platformは能力値を下げ、対応済みと表示してはいけない。

Reference rendererは性能基準の対象外だが、同じ入力、profile、recipe、seed、engine versionに対して決定的でなければならない。

# 64. 数値・画像・時間方向の適合試験

## 64.1 必須fixture

- neutral ramp: -10 EVから+10 EV
- ColorChecker reference under declared illuminant
- wavelength sweep: profile対応範囲
- saturated and negative scene-linear RGB
- alpha edge and transparent pixel
- odd dimensions and row stride
- still orientation cases 1–8
- constant／variable frame rate timestamp sequence
- fixed-seed grain sequence

## 64.2 合否

CPU Referenceと同一precisionの再実行はbit exactをMUSTとする。異なるCPU architectureではRGBA32Fの各channelについて絶対誤差`1e-6`または相対誤差`1e-5`以内をbaselineとする。

GPU Realtimeは、Referenceから生成したdisplay-referred fixtureに対して次を満たすことを初期基準とする。

- neutral ramp: code-value誤差 最大2/1023以下
- ColorChecker: CIEDE2000 平均1.0以下、最大3.0以下
- alpha: 最大絶対誤差`1e-5`以下
- timestamp: 並べ替え、重複生成、負durationなし
- fixed seed: 同一backend／engine versionでframe hash一致

Spectral／Physical Modeの閾値は測定datasetを確定した後にprofile class別に追加する。閾値未定の機能を「科学的再現済み」と表現してはいけない。

Still acceptanceではdecode可能、寸法、orientation、embedded color description、metadata round-trip、partial残存なしを確認する。Video acceptanceではcontainer decode、video/audio stream、duration、timestamp単調性、A/V start offset、停止後のfinalize、incomplete残存なしを確認する。

# 65. Version 0.2 Completion Roadmap

- [Done] Film Engineを上位Imaging Pipelineの専門rendererとして位置付け
- [Done] Camera → Lens → Film/Digital Sensor → Development → Print/Output → Displayのsignal-domain modelを実装
- [Done] Film／Digital pipeline JSON例と接続validation testを実装
- [Done] macOSでnative preview、JPEG still、音声付きMOV、resolution／FPS選択を実装
- [Done] ACEScgを標準内部計算space、ACES2065-1をinterchange用途として規定
- [Done] 数値、単位、missing value、Profile envelope、Still／Video asset lifecycleを規定
- [Done] 1080p60／4K60の性能予算と初期conformance thresholdを規定
- [Done] Profile共通JSON Schemaとloaderを実装し、validation errorにJSON pathを追加
- [Done] Film kindのdata Schema、typed payload、sensitometry curve validatorを実装
- [Done] Lens／Digital Sensor kindのSchema、typed payload、物理値validatorを実装
- [Done] `scene_linear → virtual_exposure` adapterを数式、基準露光、black floor、負値方針込みで実装
- [Done] CPU Reference executorへvirtual exposure、linear／PCHIP RGB sensitometry、alpha保持を接続
- [Done] Development／Print／Display／Output Transform kindのtyped payloadとSchemaを実装
- [Done] directory loader、Profile closure解決、content hash付きrender snapshotを実装
- [Done] explicit major-step Profile schema migration registryを実装
- [Done] CPU Reference executorへnormal developmentと最小matrix output transformを接続
- [Done] explicit synthetic Print responseをFilm Density→Display Linearへ接続
- [Done] Still／Video共通`CapturedAsset`と保存後JPEG／QuickTime probeを実装
- [Done] device別format永続化、1280×720／24 FPSのStill／Video実機validation
- [Later] 測定dataset確定後にmeasured Print responseとColorChecker fixtureを追加
- [Done] UI姿勢からPreview／Photo／Movie connectionへのrotation同期とpreview／capture mirror分離
- [Next] iOS実機でportrait／upside-down／front-camera mirror caseを検証
- [Next] CapturedAsset derivativeへrender snapshotと再現情報を保存
- [Later] wgpu renderer、OCIO adapter、LUT compiler、GPU conformance runner
- [Later] Spectral Engine、profile measurement、Film Profile Editor
