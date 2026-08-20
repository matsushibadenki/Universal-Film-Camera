# Universal Film & Color Imaging Engine
## 汎用フィルム・カラーエミュレーション基盤 設計仕様書

> **2026-08-11 Architecture Extension:** 本仕様のFilm Engineは、より上位の`Universal Imaging Pipeline`に含まれる専門engineへ発展した。レンズ、Film/Digital Sensor、現像、プリント、ディスプレイまでの最新構成は [`IMAGING_PIPELINE_ARCHITECTURE.md`](IMAGING_PIPELINE_ARCHITECTURE.md) を参照すること。本書のFilm固有要件は引き続き有効である。

Version: 0.1  
Target: macOS / Windows / Linux / iOS / Android  
Core Language: Rust

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

内部基準色空間は、

```text
scene-linear
```

とする。

推奨基準：

```text
ACES2065-1
```

または内部計算用として、

```text
ACEScg
```

を利用できる設計とする。

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

標準処理順序：

```text
Camera / File / AI
        ↓
Decode
        ↓
Input Color Transform
        ↓
Scene Linear
        ↓
Exposure
        ↓
White Balance
        ↓
Film Negative
        ↓
Film Density
        ↓
Halation
        ↓
Grain
        ↓
Print Film
        ↓
Creative Grade
        ↓
Output Transform
        ↓
Display / Encode
```

ただしノード方式として順序変更可能にする。

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

Phase 1：

```text
film-core
film-profiles
scene-linear pipeline
RGB sensitometry
CPU renderer
```

Phase 2：

```text
wgpu
GPU Film Engine
3D LUT
ACES/OCIO
```

Phase 3：

```text
Halation
Grain
MTF
Print Film
```

Phase 4：

```text
macOS/iOS camera
Windows camera
Android camera
Linux camera
```

Phase 5：

```text
AI Video Context integration
FilmRecipe
CreativePreset
metadata
project reproducibility
```

Phase 6：

```text
Spectral Engine
Physical Film Model
Profile measurement tools
Film Profile Editor
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
