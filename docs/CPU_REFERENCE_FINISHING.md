# CPU Reference Development and Output Transform

更新日: 2026-08-23
実装: `crates/film-core/src/finishing.rs`

## Status

- [Done] reference Chemical Developmentのnormal-process contrast適用
- [Done] ACEScg AP1／D60からDisplay Profile primaries／whiteへのRGB変換
- [Done] Bradford chromatic adaptation
- [Done] Rec.709、sRGB、PQ、HLG encoding関数
- [Done] straight alpha保持とdisplay出力時の明示的RGB clamp
- [Done] 外部JSON golden exposure sweep fixture
- [Done] explicit synthetic Print responseによるFilm Density→Display Linear executor
- [Done] Display Linear→Display Encoded executor
- [Next] measured push/pull response curve
- [Later] 正式なACES Output Transform／OCIO integration

## Reference Development scope

Film Profileのsensitometryはreference development後のdensity測定curveとして扱う。`CpuReferenceDevelopmentExecutor`はChemical Development Profileの`contrast_scale`をnormal process adjustmentとして適用する。

schema v1はpush／pullごとの測定response curveを持たない。したがってProfileの許容range内でも、0 stop以外を推測して処理せず`UnsupportedPushPull`で拒否する。rangeはUI／将来Profile能力の記述であり、renderer実装済み範囲とは別である。

Digital RAW DevelopmentはこのFilm density executorの対象外である。RAW decode、white balance、camera input transformはSensorRaw→SceneLinearの別rendererとして実装する。

## Matrix output transform

`CpuReferenceOutputExecutor`は次の条件だけを受理する。

- input: scene-linear ACEScg、RGBA32F、straight alpha
- method: `matrix_tone_curve`
- tone mapping: `none`
- Output TransformとDisplay Profileのprimaries／white、transfer、peak luminanceが一致

変換順序:

```text
ACEScg AP1 / D60
  → RGB-to-XYZ matrix
  → Bradford D60-to-display-white adaptation
  → XYZ-to-display-RGB matrix
  → explicit display gamut clamp [0, 1]
  → declared transfer function
  → DisplayEncodedImage
```

display primaries／whiteからmatrixを実行時に導出する。matrixの逆行列が特異、Profileが不正、pixelが非有限なら処理を拒否する。alphaはmatrix、clamp、transferの対象外としてそのまま保持する。

PQ／HLG関数はencoding数式の縦切りであり、absolute scene luminance mappingやsystem gammaを含む完全なHDR mastering pipelineではない。HDR出力を製品品質と表現するには別の測定fixtureが必要である。

## Synthetic print response

測定済みprint curveがないProfileへ汎用photochemical responseを推測しない。最初の縦切りは`response_model = inverse_density_preview_v1`を明示したProfileだけを受理し、チャンネルごとに次式を使う。

```text
effective_density = max(
  (negative_density - base_density) × contrast_scale
  - exposure_offset_ev × log10(2),
  0
)

display_linear = 1 - 10^(-effective_density)
```

これはnegative densityが増えるほど出力が明るくなり、printer exposureを増やすほど暗くなる単調なpreview contractである。dye coupling、print sensitometry、printer light spectrum、flareを再現する物理modelではない。`measured_curve`と`digital_transform`は対応rendererができるまで拒否する。

`CpuReferenceDisplayEncoder`がDisplay Linear RGBを`[0, 1]`へ明示clampし、Display Profileのtransferでencodeする。alphaはPrint responseとDisplay encodingの対象外として保持する。

## Unsupported claims

現在の`matrix_tone_curve`は正式なACES RRT／ODTではない。`aces_odt`または`ocio` method、`aces | perceptual` tone mappingは実行せず明示的に拒否する。synthetic Output Transform Profileの名称とprovenanceもこの範囲に合わせている。

Chemical Developmentの温度／時間、push／pull、dye interactionを物理simulationしていない。Profile条件を保持していることと、その条件からimage responseを計算できることを混同しない。

## Golden fixture

[`../examples/fixtures/cpu-reference-film-density-v1.json`](../examples/fixtures/cpu-reference-film-density-v1.json) は次を固定する。

- ACEScg neutral scene-linear値5段
- virtual exposure calibration
- synthetic Film Profile ID
- synthetic Development Profile ID
- expected RGB optical density
- expected synthetic Print display-linear RGB
- transparentからopaqueまでのstraight alpha

fixtureはsynthetic conformance artifactであり、実film測定datasetではない。CPU testは各density channelを絶対誤差`1e-5`未満、alphaをexactで比較する。
