# Android Camera Conformance

更新日: 2026-08-28  
対象: Tauri 2 / CameraX 1.4.2 / Android API 24+

## 現在の状態

- [Done] arm64 debug APKのbuild
- [Done] Preview／JPEG Still／音声付きMP4 Videoのnative adapter
- [Done] fallback禁止の解像度指定とCamera2 AE FPS request
- [Done] CapturedAsset probe／validation／atomic manifest境界
- [Done] 接続確認、APK導入、端末診断採取script
- [Next] 物理Android端末で以下のmatrixを実行

2026-08-28時点でADBにauthorized deviceは接続されていない。したがって、この文書のruntime欄は未判定であり、build成功を実機合格として扱わない。

## 実行方法

接続確認:

```sh
./scripts/android_camera_conformance.sh doctor
```

authorized deviceを1台だけ接続し、debug APKを導入して起動する:

```sh
./scripts/android_camera_conformance.sh install
```

各試験の直後に診断を採取する:

```sh
./scripts/android_camera_conformance.sh snapshot
```

既定の出力先は`artifacts/android-camera-conformance`である。`getprop`、`dumpsys media.camera`、package状態、app-private files一覧、logcatを端末serial別に保存する。レポートには端末情報が含まれるため、公開issueへ添付する前にserialや個人情報を除去する。

## 必須試験matrix

各行はback cameraとfront cameraで実行する。UIへ表示された候補すべてを無条件に合格とせず、Preview＋ImageCapture＋VideoCaptureの同時bindに成功した組合せを記録する。

| Case | 操作 | 合格条件 | 状態 |
|---|---|---|---|
| Permission | 初回起動でCamera／Microphoneを許可、拒否、再許可 | UIが権限状態を正しく表示し、拒否時にcaptureを開始しない | [Next] |
| Preview | Camera画面を開き、Mediaへ移動して戻る | native previewがviewport内に表示され、Media文字を覆わず、復帰時に再開 | [Next] |
| Still | 各formatでJPEGを1枚撮影 | callback後にFinalized、JPEG probe、寸法一致、manifest作成、Incomplete残存なし | [Next] |
| Video | 各formatで5秒以上録画 | Finalize後にFinalized、MP4 video/audio、正duration、FPS一致、manifest作成 | [Next] |
| Rotation | 0／90／180／270度でStillとVideo | preview方向、保存orientation／rotation、front mirror契約が一致 | [Next] |
| Background | 録画中にHome、復帰 | 中断を成功扱いせず、破損assetをFinalizedとして公開しない | [Next] |
| Recovery | capture中にprocess終了、再起動 | orphanを自動削除せずFailed／Incompleteとして診断可能 | [Next] |
| Format rejection | 非対応または同時bind不能formatを適用 | 近似formatへ黙ってfallbackせず、以前のpreview設定へrollback | [Next] |

## Format受け入れ記録

端末ごとに次を追記する。FPSはUI要求値、保存MP4 probe値、許容差を別々に記録する。

| Device / OS | Lens | Requested | Preview bind | Still saved | Video saved | Saved FPS | Audio | Result |
|---|---|---:|---|---|---|---:|---|---|
| 未接続 | — | — | — | — | — | — | — | [Next] |

## 判定上の注意

- Camera2のstream sizeとAE FPS rangeは、三つのCameraX use caseを同時利用できる組合せを直接保証しない。
- `applyFormat`成功だけでは合格にしない。StillとVideoの保存後probeまで通す。
- VideoのFPSはcontainer timingから取得したrational rateを使い、表示整数だけで判定しない。
- background中断やCameraX Finalize errorをFinalizedへ昇格させない。
- 実機で合格したformatだけを将来のcapability filter／端末互換性fixtureへ採用する。
