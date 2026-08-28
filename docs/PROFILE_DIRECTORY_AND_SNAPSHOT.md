# Profile Directory Loader and Render Snapshot

更新日: 2026-08-26
実装: `crates/imaging-core/src/profile_io.rs`

## Status

- [Done] directory以下の`.json` Profileを再帰的・辞書順で読み込むloader
- [Done] symlinkを追跡せず、重複IDや不正Profileで全体を失敗させるtransactional load
- [Done] 有効Pipeline nodeとProfile間参照から必要Profile closureを解決
- [Done] Pipeline、各Profile、snapshot全体のSHA-256を生成
- [Done] ID順の決定論的snapshotとmissing／kind mismatchの構造化error
- [Done] schema migration registryをdirectory loaderへ接続
- [Later] 実在する旧major Profileの明示migration
- [Done] CapturedAsset schema v2のderivative metadataへsnapshotとengine version／seedを保持
- [Done] asset manifestとMedia indexへsnapshotをatomic永続化
- [Later] Profile package署名とtrust policy

## Directory loading contract

`ProfileCatalog::load_directory(path)`は指定fileまたはdirectoryを読み込む。directoryは再帰走査し、拡張子が大文字小文字を問わず`.json`の通常fileだけを対象とする。symlinkはdirectory cycle、意図しないworkspace外読み込み、platform差を避けるため追跡しない。

対象pathは辞書順へsortしてからparseする。読み込み、JSON parse、共通Envelope、typed payload、重複IDのいずれかが失敗した場合、部分Catalogを返さない。errorはfile pathに加え、可能なら`$.data...`形式のProfile pathを持つ。JSON fileが1つもなければ`Empty`とする。

loaderはcurrent `schema_version = 1`を受理する。旧majorは名前付きstepがregistryへ明示登録されている場合だけ1 majorずつmigrationする。未知future majorを最新と推測して読み替えない。詳細は [`PROFILE_MIGRATION_REGISTRY.md`](PROFILE_MIGRATION_REGISTRY.md) を正本とする。

## Render snapshot closure

`ProfileCatalog::snapshot_for_pipeline(pipeline)`は次の順序で処理する。

```text
Pipeline validation
  → enabled nodeの直接Profile参照を収集
  → expected ProfileKindを検証
  → ProfileEnvelope.referencesを推移的に解決
  → selected ProfileをID順へ正規化
  → content hashとsnapshot hashを生成
```

disabled nodeとCatalog内の無関係Profileはsnapshotへ含めない。同じIDが異なるkindとして要求された場合、またはProfileの実kindが要求と違う場合はrender開始前に拒否する。Sourceの任意Profileはkindを限定しないが、そのProfile自身の参照は解決する。

現在のPipeline fieldとの対応:

| node | expected kind |
|---|---|
| Camera | `camera` |
| Lens | `lens` |
| Film／Digital Sensor | `film`／`digital_sensor` |
| Chemical／Digital RAW Development | `development` |
| Print／Digital Intermediate | `print` |
| Output Transform | `output_transform` |
| Display | `display` |

## Hash contract

Profileはtyped validation後の`ProfileEnvelope`を決定論的なJSONへserializeし、SHA-256を計算する。空白や元JSONのobject key順には依存しない。一方、same-major未知extensionはround-trip対象なのでhashにも含む。`$schema` annotationも現在はextensionとして含まれる。

`RenderProfileSnapshot`は次を持つ。

```text
schema_version
pipeline_id
pipeline_sha256
profiles[] { id, kind, profile_version, content_sha256 }
snapshot_sha256
```

`pipeline_sha256`はnode parameter、順序、enabled状態を含むPipeline全体の正規化内容hashである。`snapshot_sha256`はPipeline hashとID順Profile entriesをまとめたpayloadのhashであり、自分自身のhash fieldは計算対象に含めない。

同じ`profile_version`のまま内容が変更されても`content_sha256`と`snapshot_sha256`は変化する。この検出は運用ミスを可視化するが、同じversionで内容を書き換えることを許可するものではない。

## Security scope

SHA-256は内容識別と破損／変更検出に使う。作成者の真正性、配布元、license、信頼性を証明する電子署名ではない。remote packageを受け入れる段階では署名済みmanifestとtrust policyを別途追加する。

directory loaderはsymlinkを追わないが、file size、総Profile数、JSON nesting depthの独自上限はまだない。外部から受け取ったProfile packageを無制限に直接読み込ませてはいけない。

## Current verification

- bundled Profile 8件のrecursive loadと全参照解決
- Film Emulation Pipelineから4件だけを選ぶclosure
- snapshotのID sortと同一入力での完全一致
- version据え置きのProfile内容変更によるhash変化
- missing direct ProfileのPipeline JSON path付きerror

Camera Profile typed payloadとexampleはまだ未実装なので、Camera nodeを含むDigital／physical Film Pipelineの完全snapshotはそのProfile追加後に成立する。現在のsnapshot縦切りはCamera nodeを持たないFilm Emulation Pipelineを正本fixtureとする。
