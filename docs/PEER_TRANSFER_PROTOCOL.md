# Nearby Peer Transfer Protocol

更新日: 2026-08-31

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
- [Done] CapturedAsset original／derivative／両方のAsset Transfer Manifest表現
- [Done] `.incomplete`受信writer、durable ACK、resume ledger、既存byte再hash
- [Done] 容量予約、managed path／symlink検査、実file SHA-256、atomic rename
- [Done] Original受信をMedia Incomplete／Failed／Finalizedへ接続
- [Done] JPEG `StripDeviceAndLocation` sanitizer。EXIF／XMP／IPTC／comment／未知APP segmentを除去して再hash
- [Done] sanitizerだけが生成できる`SanitizedJpeg`からOriginal transfer manifestを構築し、改変時は再照合で拒否
- [Done] Derivative resourceにparent resource ID／render snapshot／engine version／seedを含め、同じ親を持つ既存assetへ確定
- [Done] Original＋Derivative bundleのOriginal先行確定、source→local ID map、依存順序、重複／欠落／循環検査
- [Done] ChaCha20-Poly1305 encrypted chunk。transfer ID／offset／長さをAADへbinding
- [Done] durable受信prefix SHA-256を送信元fileと照合するresume checkpoint
- [Done] X25519 ephemeral DH、公開鍵とManifestを認証する6桁code、HKDF-SHA256 session secrets
- [Done] OS CSPRNG key生成、bounded binary framing、TCP stream adapter
- [Done] encrypted chunk／resume checkpoint／durable ACK wire message
- [Done] sender／receiver lifecycle、単一in-flight chunk、cancel、disconnect-resume、finalize接続
- [Done] Apple mDNS advertise／browse、P2P interface、実TCP listener、Tauri start／snapshot／stop commands
- [Done] Finalized Media再解決、Original実file hash、2分Invitation、peer選択、6桁code表示、local approval UI
- [Done] 64 KiB上限Handshake Offer／Approval wire frame、remote承認context検証、双方能力交渉、相互codec導出
- [Done] Apple listener accept／outbound connect、発見済みpeer key照合、incoming approval、secure transport／codec state保持
- [Done] Encrypted Original送信、受信durable ACK、IndexedOriginalReceive、hash／probe／Media確定、TransferFinalized返信
- [Later] 選択的`StripLocation`とMOV／MP4 metadata sanitizer。未実装policy指定は拒否
- [Done] Apple transfer task開始時のreceiver checkpoint交換と検証済みoffsetからの暗号化送信
- [Done] durable ACK progress snapshot、local cancel要求、wire Cancel、英語／日本語／简体中文UI
- [Done] Apple可視セッション内の同一Offer再handshake、checkpoint resume、明示retry UI
- [Done] failure reason分類とdisconnect／timeoutだけに限定したretry UI
- [Done] Invitation失効後の新規承認導線とmanaged partialの確認付きdiscard
- [Next] partial retention／一括管理、background／network切替
- [Next] iOS／Android background、network切替、timeoutの実機試験
- [Later] Apple／Android／Windows／Linux platform adapter

## セキュリティ不変条件

1. peer identityへBluetooth address、端末名、永続device IDを使用しない。
2. 招待の明示承認と確認code一致前にdata transportを開始しない。
3. 受信側は送信側のpathを信用せず、manifestはbasenameだけを許可する。
4. 宣言容量と保存先空き容量をwriter作成前に検証する。
5. ACKは連続して永続化済みのbyte位置だけを表す。
6. 受信完了通知だけでは公開せず、実fileの長さとSHA-256を検証する。
7. flush、hash、rename、Media manifest保存前のassetをFinalizedとして表示しない。
8. metadata除去はmanifest上の自己申告にせず、実byteを書き換えた出力を再hashする。
9. Derivativeは来歴がなくてもOriginalとして公開せず、宣言された親resourceが存在する場合だけ追加する。
10. bundle内の親参照は推測で書き換えず、確定済みresourceだけをsource→local ID mapへ登録する。
11. chunkはtransfer ID、offset、平文長、asset総長をauthenticated dataへ含め、別位置・別transferへ再利用できなくする。
12. resume offsetは受信側の自己申告だけで採用せず、送信元fileの同じprefix SHA-256と一致させる。
13. 6桁確認codeは任意のPINにせず、X25519共有秘密、双方の公開鍵、Invitation、Manifestから導出して画面間で比較する。
14. wire payload長はallocation前に上限検査し、未知message、末尾data、長さ不一致を拒否する。
15. senderはdurable ACK前に次chunkへ進まず、disconnect後は検証済みcheckpoint位置からだけ再開する。
16. Complete／Cancelled状態から送信を再開せず、Complete後のcancelも状態を書き換えない。
17. discovery TXT recordは公開情報だけを持ち、端末名、永続device ID、secret key、確認codeを広告しない。
18. 広告portは先にbind済みのlistenerから取得し、接続不能な推測portを公開しない。
19. local code承認だけではTransferringへ進めず、認証済みremote approvalまではNegotiatingに留める。
20. Handshake ApprovalはInvitation ID、transfer ID、確認code、offer sender keyと異なるapprover keyへbindingし、Offerと異なるcontextを拒否する。
21. control frameは64 KiBを超える宣言をpayload allocation前に拒否する。
22. incoming Offerは現在Bonjourで発見中のephemeral IDと公開鍵が完全一致するsenderだけを受理する。
23. outbound remote待機中にnative state lockを保持せず、cancel／stop後に戻った結果を古いsessionへ確定しない。
24. senderは最終DurableAckだけで成功扱いにせず、receiverのhash／probe／Media manifest確定後のTransferFinalizedを待つ。
25. transfer taskはreceiverのdurable checkpointを最初に交換し、sender側の同一prefix SHA-256が一致したoffsetからだけ暗号化chunk送信を開始する。
26. transport切断後の再接続は同じtransfer identityへ明示的に戻す。別transferのpartialを推測で再利用しない。

## Platform adapterの責務

Bluetooth LE、Bonjour、Nearby Connectionsなどはpeer discovery、invite delivery、transport候補通知を担当する。Appleの最初のadapterは`_ufcamera._tcp.local.`をadvertise／browseし、TXTのprotocol version、ephemeral ID、X25519 public key、任意labelだけを受理する。写真・動画本体は交渉済み高速transportで転送し、platform adapterが独自の成功状態を作ってはならない。最終状態は必ず`peer-transfer-core`とMedia lifecycleを通す。
