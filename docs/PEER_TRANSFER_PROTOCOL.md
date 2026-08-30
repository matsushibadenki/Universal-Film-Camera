# Nearby Peer Transfer Protocol

更新日: 2026-08-30

## 実装済み境界

`peer-transfer-core`はApple／Android／Windows／Linuxの発見APIやsocket実装から独立した共通契約である。

- [Done] visibility sessionごとのephemeral peer identity
- [Done] 期限と6桁確認codeを持つ明示的invitation
- [Done] protocol version、transport、最大chunk、resume能力の交渉
- [Done] BLEをcontrol用途に限定し、高速transportがない場合の転送拒否
- [Done] version 1 Transfer Manifest
- [Done] basename、100 GiB上限、16 KiB〜4 MiB chunk、SHA-256形式の検証
- [Done] AwaitingApproval → Negotiating → Transferring → Verifying → Finalized状態機械
- [Done] ACKの単調増加、宣言長超過拒否、cancel、hash不一致時Failed
- [Next] CapturedAsset original／processed／両方のmanifest表現
- [Next] `.incomplete`受信writer、chunk payload、resume ledger、実file SHA-256
- [Next] fsync／rename／Media manifestを一つのatomic finalize境界へ接続
- [Later] ephemeral key agreementとend-to-end authenticated encryption
- [Later] Apple／Android／Windows／Linux platform adapter

## セキュリティ不変条件

1. peer identityへBluetooth address、端末名、永続device IDを使用しない。
2. 招待の明示承認と確認code一致前にdata transportを開始しない。
3. 受信側は送信側のpathを信用せず、manifestはbasenameだけを許可する。
4. 宣言容量と保存先空き容量をwriter作成前に検証する。
5. ACKは連続して永続化済みのbyte位置だけを表す。
6. 受信完了通知だけでは公開せず、実fileの長さとSHA-256を検証する。
7. flush、hash、rename、Media manifest保存前のassetをFinalizedとして表示しない。

## Platform adapterの責務

Bluetooth LE、Bonjour、Nearby Connectionsなどはpeer discovery、invite delivery、transport候補通知を担当する。写真・動画本体は交渉済み高速transportで転送し、platform adapterが独自の成功状態を作ってはならない。最終状態は必ず`peer-transfer-core`とMedia lifecycleを通す。
