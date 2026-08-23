# Profile Schema Migration Registry

更新日: 2026-08-23
実装: `crates/imaging-core/src/profile_io.rs`

## Status

- [Done] major schemaごとの名前付きmigration step registry
- [Done] 1 majorずつ順番に適用し、各stepの出力versionを検証
- [Done] missing step、duplicate step、future schema、誤った出力versionの構造化error
- [Done] directory loaderからmigration適用履歴をfile／Profile ID単位で返す
- [Done] migration後にtyped validationし、その内容をrender snapshot hashへ使用
- [Later] 実在するlegacy schemaが確定した時点でbuilt-in migrationを追加

## Core rule

Migrationは推測ではなく、compiled codeとして明示登録した純関数だけを実行する。

```text
raw JSON Value
  → schema_versionを整数として読む
  → registered from N → N+1 migration
  → 出力schema_version == N+1を確認
  → 必要なら次step
  → ProfileEnvelope decode
  → common + typed payload validation
  → Catalog insert
  → render snapshot hash
```

stepはmajor versionを1つだけ進める。`v0 → v2`のようなskip migrationは登録できない。途中stepがない場合は`MissingStep`で停止し、近いschemaへ読み替えない。current versionより新しいProfileは`FutureSchemaVersion`として拒否する。

## API contract

```rust
fn migrate_v0_to_v1(value: serde_json::Value)
    -> Result<serde_json::Value, String>;

let mut registry = ProfileMigrationRegistry::default();
registry.register(0, "profile-v0-to-v1", migrate_v0_to_v1)?;

let load = ProfileCatalog::load_directory_with_registry(path, &registry)?;
let catalog = load.catalog;
let applied = load.migrations;
```

`ProfileMigrationRegistry::default()`はcurrent `schema_version = 1`をtargetにするstrict registryであり、built-in legacy stepを持たない。現在のprojectに公開済みv0 contractがないためである。実在しないlegacy形式を想像してproduction migrationへ入れてはいけない。

## Migration report

少なくとも1 stepを適用したfileだけ、次の情報を`ProfileDirectoryLoad.migrations`へ記録する。

```text
path
profile_id
applied[] {
  name
  from_schema_version
  to_schema_version
}
```

reportは診断・asset provenanceへ保存できるが、migration後Profile自体へ履歴fieldを自動注入しない。内容へ注入すると、migration処理の有無だけでrender hashが変わるためである。render hashは最終的に検証されたProfile内容を正本とする。

## Authoring requirements

新しいbuilt-in migrationを追加するときは以下を必須とする。

1. 旧Schemaと新Schemaの両方をrepositoryへ保存する
2. rename、unit変換、default挿入、削除fieldを文書化する
3. 情報を失う変換は自動実行せず、ユーザー判断または明示policyを要求する
4. migration前後のgolden JSON fixtureを追加する
5. 同じ入力からbit-identicalな正規化JSONを生成する
6. migrated Profileのtyped validationとsnapshot hashまでtestする

Migration functionはrepositoryに同梱されたtrusted codeとして扱う。外部Profile packageから実行コードやscriptを読み込んではならない。

## Current verification

合成test migrationは`legacy_profile_version`を`profile_version`へrenameし、schema 0から1へ進める。この形式はregistry機構のtest専用であり、公開済みProfile v0仕様ではない。

- explicit stepの適用順と履歴
- migration後のFilm typed validation
- missing stepの拒否
- duplicate source stepの拒否
- current v1 directoryでmigration reportが空であること

