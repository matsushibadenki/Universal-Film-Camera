# CPU Reference Film Executor

更新日: 2026-08-23
実装: `crates/film-core/src/reference.rs`

## Status

- [Done] scene-linear ACEScg、linear transfer、RGBA32Fの`LinearImage`だけを入力として受理
- [Done] `VirtualExposureNode`でRGB `log10(lux·s)`へ変換
- [Done] Film ProfileのRGB sensitometryをlinearまたはshape-preserving monotonic cubicで評価
- [Done] `clamp | linear | reject` extrapolationをProfileどおりに適用
- [Done] straight alphaを演算対象外としてbit-preservingで伝播
- [Done] 非有限値、不正な入力寸法、負の計算density、範囲外露光を構造化errorにする
- [Done] normal reference developmentと最小matrix output transformを追加
- [Done] golden exposure sweepをfile artifactとして追加
- [Next] 校正済みColorChecker fixtureを追加
- [Later] CPU Referenceと同じfixtureを使うGPU conformance runner

## Responsibility boundary

`imaging-core`はProfile、node、SignalDomain、接続validationを所有する。実画素処理は専門rendererである`film-core`が所有し、`imaging-core`は`film-core`へ依存しない。これによりPipelineの記述層とrendererを循環依存させない。

入力はscene-linear ACEScg、linear transfer、RGBA32Fの`LinearImage`、出力は`FilmDensityImage`である。NV12、encoded RGB、RGBA16FはReference入口で拒否する。出力RGBは色値ではなくチャンネル別のlog10 optical densityなので、ACEScgを示す`FrameDescriptor`を流用しない。width、height、density RGBAだけを持つ別型としてFilm domainの境界を明示する。

## Evaluation order

```text
LinearImage (scene-linear ACEScg, straight alpha)
  → VirtualExposureNode
  → RGB log10(lux·s)
  → SensitometryEvaluator
  → RGB log10 optical density + unchanged alpha
  → FilmDensityImage
```

1 pixelごとにRGBだけを露光・curve評価へ渡す。alphaは露出、Film responseの対象外であり、そのまま出力する。非有限alphaは入力破損として拒否する。premultiplied inputのunpremultiplyはこのexecutorの責務ではなく、入力変換段で完了していなければならない。

## Interpolation contract

`linear`は隣接sample間の線形補間を行う。`monotonic_cubic`はPCHIP／Fritsch–Carlson方式の接線を事前計算する。単調区間のovershootを抑え、sample上の極値では接線を0にする。全RGBチャンネルは同じ露光軸を使うが、density値と接線は別々に評価する。

範囲外処理:

- `clamp`: 最初または最後のdensityを返す
- `linear`: 最初または最後の2 sampleのsecantを延長する
- `reject`: channel、入力露光、Profile範囲を含むerrorを返す

linear extrapolationが負または非有限のoptical densityを生成した場合は、物理的に無効なProfile／入力組合せとして拒否する。暗黙のzero clampは行わない。

## Determinism and current limitations

CPU Referenceは`f32`を使い、同一build／CPUで同じ入力を再実行した結果を比較基準とする。このexecutor単体の出力はFilm densityであり表示可能画像ではない。normal Chemical Developmentとscene-linear素材用matrix Output Transformは [`CPU_REFERENCE_FINISHING.md`](CPU_REFERENCE_FINISHING.md) へ実装したが、Film Density→Print→Display経路はまだ未実装である。

現在のunit fixtureは次を検証する。

- sample knot一致とmonotonic cubic midpoint
- linear interpolation
- clamp、linear、reject extrapolation
- ACEScg露光からRGB densityへのend-to-end変換
- transparent／半透明／opaque alphaの保持
- 負のscene-linear入力とpixel index付きerror

## Next acceptance target

次の工程では、Print Profile responseをFilm Density→Display Linearへ接続する。ColorCheckerはinput transformと照明条件を固定した後に追加し、未校正RGB fixtureを測色試験として扱わない。
