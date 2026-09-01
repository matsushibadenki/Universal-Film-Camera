# Shutter Sound Import Staging

このディレクトリは、後でアプリへ取り込むシャッター音原本の配置場所です。

- 対応予定形式: `.wav`、`.caf`、`.m4a`、`.mp3`
- 推奨原本: WAVまたはCAF、48 kHz、16/24-bit、monoまたはstereo
- 1ファイルあたりの現在の取込上限: 5 MiB
- ファイル名: 半角英数字、ハイフン、アンダースコアを推奨
- 音源の出典、作者、ライセンス、編集履歴を同名のメモに残してください

ここに置いただけではアプリへ自動登録されません。ファイル追加後に、形式検査、端末試聴、アプリへの同梱を別作業として行います。音量正規化やfadeは原音を変更するため、必要性と目標値を決めてから派生assetへ適用し、原本は保持します。

## Integrated catalog (2026-09-01)

次の5ファイルはViteの同梱assetとして環境設定 > 撮影 > シャッター音へ登録済み。表示名は提供時の識別名を維持する。

| ID | File | Format | Duration | Peak | SHA-256 |
|---|---|---|---:|---:|---|
| `c137` | `C137.wav` | 48 kHz Float32 stereo | 0.380 s | 0.930588 | `0b12a7e75c1bc100862a964ec5db7caf07e9d473a60264d789d65de999799662` |
| `f2` | `F2.wav` | 48 kHz Float32 stereo | 2.400 s | 0.879218 | `9c810ff27b24c969f401bd65f816b6d721b487d8487ed306f0c028179cb6c844` |
| `h503` | `H503.wav` | 48 kHz Float32 stereo | 3.378 s | 0.630393 | `e8938f2c08f90935d94ac674d4f33e6fa7b01087321562bc6a487222cdb95f5c` |
| `m4` | `M4.wav` | 48 kHz Float32 stereo | 2.130 s | 0.909660 | `9bcbfcccb3945a961559833c3fcaf143f294093a252ad679853aa00333812422` |
| `small` | `small.wav` | 48 kHz Float32 stereo | 0.380 s | 0.008785 | `a1b2bf10b48e9e4d4d36b17ee87a30f1c9beeb43f51f854ca82a936da37839b8` |

`small.wav`は他4件よりpeakが約40 dB低い。名称から意図を推測せず、現時点では原音のまま再生する。

### Distribution metadata

- [Next] 提供者、出典、作者、ライセンス、商用配布可否を記録する。
- [Next] iPhone speakerで5音を試聴し、必要なら原本を保持したまま派生assetのgain／fade／trim値を決める。
