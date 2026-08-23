# Scene-linear to Virtual Exposure Adapter

更新日: 2026-08-23  
対象実装: `imaging-core::VirtualExposureNode`  
入力: scene-linear ACEScg  
出力: RGB `log10(lux·s)` virtual exposure (`optical_image` domain)

## Status

- [Done] ACEScg RGBから仮想対数露光へ変換するtyped node
- [Done] 18% neutral gray基準、露出補正、black floor、負値方針の明示
- [Done] Film emulation Pipeline例とdomain／設定validation
- [Done] 基準gray、1 stop、black floor、負値、working spaceのunit test
- [Done] CPU Reference executorへRGB sensitometry evaluatorとともに接続
- [Next] 実測Profileごとに`reference_log_exposure`を校正する手順とfixtureを追加
- [Later] 分光放射量、film spectral sensitivity、光学系を扱うSpectral／Physical adapter

## Normative mapping

チャンネル`c`のscene-linear値を`C_c`とすると、version 1のRGB adapterは次式を使う。

```text
log10(H_c) = log10(H_ref)
           + log10(max(C_c, C_floor) / C_ref)
           + EV × log10(2)
```

- `H_c`: チャンネル別の仮想露光量、単位はlux·sの対数表現
- `C_ref`: `reference_scene_linear`。通常はneutral grayの`0.18`
- `log10(H_ref)`: `reference_log_exposure`。`C_ref`へ割り当てる校正済み絶対露光
- `C_floor`: `minimum_scene_linear`。対数の負無限大を避ける正の下限
- `EV`: `exposure_compensation_ev`。1 stop増加すると露光量は2倍になる

`C_ref = 0.18`はscene-linear信号の相対基準であり、単独では絶対的なlux·sを決定しない。`reference_log_exposure`はfilm、metering、scene calibrationまたは測定fixtureから別途決定し、Profile／Pipelineとともに保存しなければならない。サンプルPipelineの`-1.0`は計算確認用の合成値であり、市販filmの実測値ではない。

## Input policy

version 1はworking color spaceがACEScg（AP1 primaries、ACES white point D60）のPipelineだけを受理する。white pointはACEScgの定義へ固定されるためadapter固有parameterとして重複保存しない。別primaries、別white point、encoded RGBをACEScgとみなしてはならず、先に明示的なinput transformが必要である。

ゼロは必ず`C_floor`へclampする。負のscene-linear値はPipeline用途に応じて次のいずれかを明示する。

- `clamp_to_floor`: display-referred transformなどで生じた小さな負値をblack floorへ固定する
- `reject`: Reference fixtureや診断用途で負値を入力エラーとして扱う

NaN、無限大、不正な基準値は拒否する。暗黙の絶対値化、チャンネル間平均、auto exposureは行わない。

## Scientific scope

このnodeはRGB Film Emulation用の校正adapterであり、scene radiance、レンズ透過、入射角、photometric weighting、filmの分光感度を物理的に積分した結果ではない。RGB各チャンネルへlux·sという校正軸を割り当てる近似であるため、Spectral／Physical Modeの根拠として使用してはいけない。

Film sensitometryのx軸は`log10_lux_seconds`である。adapter出力を同じ軸へ接続することで、scene-linearを`scene_light`へ暗黙変換せず、校正の仮定を再現可能なparameterとして残す。

## Example and verification

- Pipeline: [`../examples/pipelines/film-emulation-reference.json`](../examples/pipelines/film-emulation-reference.json)
- Rust実装・test: `crates/imaging-core/src/lib.rs`

受け入れ条件:

1. `C_c = C_ref`、`EV = 0`で出力が`log10(H_ref)`になる
2. `C_c`を2倍、または`EV`を+1すると、ともに`log10(2)`増加する
3. zero／negative policyとblack floorが再現可能である
4. ACEScg以外のworking spaceや非有限値を実行前に拒否する

## References

- [ACEScg Specification](https://docs.acescentral.com/encodings/acescg/) — AP1 primariesを用いるlinear encoding
- [ACES Input Transform Capture Guide](https://docs.acescentral.com/system-components/input-transforms/capture-guide/) — neutral 18% grayの撮影基準
- [Kodak Basic Photographic Sensitometry Workbook](https://www.kodak.com/content/products-brochures/Film/Basic-Photographic-Sensitometry-Workbook.pdf) — lux-seconds、log exposure、1 stopの関係
