# Profile Schema and Loader

更新日: 2026-08-22  
Schema: `profile-common-v1`  
実装: `crates/imaging-core`

## 現在地

- [Done] Camera／Lens／Digital Sensor／Film／Development／Print／Display／Output Transform／Synthetic共通envelopeをRust型へ実装
- [Done] JSON Schema Draft 2020-12を`docs/schemas/profile-common-v1.schema.json`へ追加
- [Done] `schema_version = 1`、semantic version、RFC 3339 timestamp、必須文字列、object dataをloaderで検証
- [Done] validation errorへ`$.profile_version`などのJSON pathと理由を付与
- [Done] 未知の同一major fieldを`extensions`へ保持し、deserialize／serializeで失わない
- [Done] Profile Catalogで重複ID、参照先不在、参照先kind不一致を検出
- [Done] 合成color negative fixtureを追加し、Schema JSON、loader、round-tripをunit testへ接続
- [Done] Film専用Schemaと`FilmProfileData` typed payloadを追加
- [Done] Film sensitometryの単位、sample順序、非負density、補間／外挿enumを検証
- [Done] Lens Profile v1 Schema、typed payload、物理範囲／anamorphic条件を検証
- [Done] Digital Sensor Profile v1 Schema、typed payload、CFA／code range／ISO／分光感度を検証
- [Next] Development／Print／Display／Output Transformの`data` Schemaとtyped payloadを追加
- [Next] file／directory loaderとschema migration registryを追加
- [Next] pipelineが参照する全profileをCatalogで解決し、render開始前にsnapshot化
- [Later] 署名済みProfile package、remote registry、license policy enforcement

## ファイル

- `docs/schemas/profile-common-v1.schema.json`: 共通JSON Schema
- `docs/schemas/film-profile-v1.schema.json`: Film Profile専用JSON Schema
- `docs/schemas/lens-profile-v1.schema.json`: Lens Profile専用JSON Schema
- `docs/schemas/digital-sensor-profile-v1.schema.json`: Digital Sensor Profile専用JSON Schema
- `examples/profiles/synthetic-color-negative-500.json`: 合成fixture
- `crates/imaging-core/src/lib.rs`: `ProfileEnvelope`、`ProfileCatalog`、validator

## 共通Envelope

全profileは次を必須とする。

```text
schema_version
profile_version
id
kind
manufacturer
model
license
created_at
provenance
data
```

`references`は任意で、別profileのIDと期待するkindを記録する。参照解決は単体profileのparse時ではなく、必要なprofileをCatalogへ登録した後に行う。これにより読み込み順へ依存せず、循環検出などの将来拡張もCatalog側へ集約できる。

## Loader contract

```rust
let profile = ProfileEnvelope::from_json(json)?;

let mut catalog = ProfileCatalog::default();
catalog.insert(profile)?;
catalog.validate_references()?;
```

`from_json`は構文解析後に共通validationを実行する。`insert`も再検証するため、deserialize後に書き換えられた不正profileはCatalogへ入らない。同じIDの上書きは暗黙に行わず拒否する。

共通Envelopeの`data`はJSON objectとして保持する。`kind = film`では追加で`FilmProfileData`へdecodeし、専用validatorを必ず通す。Lens、Sensorなど未実装kindのpayloadをrendererが任意のJSONとして信用して実行してはいけない。

Film v1は`film_type`、nominal exposure index、native color temperature、sensitometryを必須とする。Sensitometryのx単位は`log10_lux_seconds`、y単位は`log10_optical_density`へ固定し、最低2 sample、露光軸のstrictな単調増加、RGB densityの非負値を検証する。補間は`monotonic_cubic | linear`、範囲外は`clamp | linear | reject`から明示選択する。

Lens v1はlens type、mount、焦点距離範囲、F-number範囲、最短撮影距離、image circleを必須とする。範囲は正の有限値かつ`min <= max`とする。Anamorphic lensだけが1より大きいsqueeze ratioを持てる。

Digital Sensor v1はactive pixel寸法、物理寸法、native bit depth、CFA、black／white level、base ISO、ISO範囲を必須とする。white levelはblack levelより大きくnative bit-depth内、base ISOは宣言範囲内とする。分光感度は省略可能だが、存在する場合は360–830 nm、最低2 sample、strictな波長増加、非負RGB responseを要求する。

## Extension policy

Schemaは同一major versionの追加fieldを許可する。Rust loaderは未知fieldを`extensions`として保持し、editorが読み込み／保存しただけで将来fieldを消さない。

未知の`schema_version`は拒否する。未知fieldの保持は、未知major schemaを理解したことにはならない。

## Error examples

```text
$.schema_version: unsupported profile schema version 2; expected 1
$.profile_version: must be a semantic version
$.references[0].profile_id: a profile cannot reference itself
profiles[org.example.development].references[0].profile_id: referenced profile not found
```

UIへ表示するときはpathと技術的reasonを開発者logへ残し、英語・日本語・简体中文のユーザー向け要約へ変換する。元JSONの全文をIPC errorやproduction logへ出してはいけない。

## 互換性

- minor／patch更新: 共通Envelopeの既存fieldを壊さず、追加fieldを許可
- major更新: migrationを明示実装するまでloaderが拒否
- profile version: Profile内容の版。Schema versionとは別管理
- engine version: CapturedAsset／derivative側で記録し、Profileへ埋め込まない

Profileの`id + profile_version`と内容hashをrender snapshotへ保存する設計を次工程で追加する。同じversion文字列で測定値を書き換える運用は禁止する。
