# Development, Print, Display, and Output Transform Profiles

更新日: 2026-08-23
Schema version: 1
実装: `crates/imaging-core`

## Status

- [Done] Chemical／Digital RAW Development typed ProfileとJSON Schema
- [Done] Photochemical／Digital Intermediate／Paper Print typed ProfileとJSON Schema
- [Done] Display primaries、white point、transfer、black／peak luminance Profile
- [Done] ACEScg入力のOutput Transform Profileとencoding／transfer整合検証
- [Done] synthetic example、JSON path付きnegative test、Pipeline参照IDの統一
- [Done] directory loaderとrender snapshotでPipeline参照を一括解決
- [Done] CPU Referenceへnormal Development／matrix Output Transformの最小演算を接続
- [Done] explicit `inverse_density_preview_v1` responseをFilm Density→Display Linearへ接続
- [Later] measured process curves、ICC／OCIO／ACES transform package

## Development Profile v1

`development_type`は`chemical | digital_raw`である。両者に`process_name`、zero stopを含む`push_pull_stops`範囲、正の`contrast_scale`を要求する。

Chemicalは正のreference temperature（°C）とtime（seconds）を必須とし、working color spaceを持たない。Digital RAWはchemical条件を持たず、出力working color spaceを必須とする。`custom` color spaceは識別情報が不足するためv1 payloadでは拒否する。

このProfileは工程条件のtyped foundationであり、時間／温度からdensity curveを物理予測するrendererはまだ未実装である。合成ECN-2例の数値をメーカー標準や実測値として扱ってはいけない。

## Print Profile v1

Printは`print_type`とSignalDomainを組で検証する。

| print type | input | output | base density |
|---|---|---|---|
| `photochemical` | `film_density` | `display_linear` | required |
| `paper` | `film_density` | `display_linear` | required |
| `digital_intermediate` | `scene_linear` | `display_linear` | forbidden |

`base_density`は非負のRGB optical densityである。`contrast_scale`は正、`exposure_offset_ev`は有限値とする。v1はPipelineの現行linear node契約に合わせ、出力を`display_linear`へ固定する。

`response_model`は、合成previewの`inverse_density_preview_v1`、将来の`measured_curve`、Digital Intermediate用`digital_transform`を区別する。現在rendererが実装するのは最初の合成modelだけであり、測定curveを暗黙に代替しない。

## Display Profile v1

DisplayはCIE 1931 xyのRGB primariesとwhite point、display transfer、peak／black luminance、surround、technologyを保持する。xyは有限、`0 ≤ x ≤ 1`、`0 < y ≤ 1`、`x + y ≤ 1`を要求する。blackは0以上かつpeak未満でなければならない。

Display profileはencoded displayの特性なので、v1では`linear`と`log` transferを拒否する。例のRec.709 chromaticityと100 nitはsynthetic conformance fixtureであり、接続中の実モニターを測定した値ではない。

## Output Transform Profile v1

v1入力は規範working spaceのscene-linear ACEScgへ固定する。出力encodingとtransfer functionは次の組合せを要求する。

| encoding | transfer |
|---|---|
| `rec709` | `rec709` |
| `srgb` / `display_p3` | `srgb` |
| `rec2020_pq` | `pq` |
| `rec2020_hlg` | `hlg` |
| `custom` | explicit custom profile ID |

methodは`aces_odt | matrix_tone_curve | ocio`、tone mappingは`none | aces | perceptual`から選ぶ。合成例は`matrix_tone_curve + none`であり、Display Profileから行列を導出する。正式なACES transform実装を内包しない。

## Artifacts

Schemas:

- `docs/schemas/development-profile-v1.schema.json`
- `docs/schemas/print-profile-v1.schema.json`
- `docs/schemas/display-profile-v1.schema.json`
- `docs/schemas/output-transform-profile-v1.schema.json`

Examples:

- `examples/profiles/synthetic-ecn2-development.json`
- `examples/profiles/synthetic-neutral-raw-development.json`
- `examples/profiles/synthetic-theatrical-print.json`
- `examples/profiles/reference-rec709-display.json`
- `examples/profiles/aces-rec709-output-transform.json`

## Handoff constraints

1. Profileの存在だけでrendererが実装済みとは表現しない
2. Pipeline実行前にID、kind、profile version、内容hashをsnapshotへ固定する
3. Output Transformの表示結果を検証するときはDisplay Profileも同時に固定する
4. manufacturer公称値、実測値、digitized資料、推定値、synthetic値をprovenanceで混同しない
