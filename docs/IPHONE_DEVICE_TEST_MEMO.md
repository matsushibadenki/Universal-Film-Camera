# iPhone実機テストメモ

更新日: 2026-11-02

対象はiOS版。Android／macOS等の新規受け入れと実機2台のNearby試験は保留する。この文書のチェック欄は手順であり、合格済みという意味ではない。

## まず確認すること

- 最新コードのbuildとinstallを行う。前回のIPAが残っていても最新版とは限らない。
- テスト用の被写体・音声を使用する。明暗差、細かい文字、直線がある被写体が便利。
- 最初は「撮影直後にiPhoneの写真へ送って削除」と一括移行を使用しない。環境設定で自動転送・削除をOFFにする。
- アプリを削除して入れ直さない。上書き更新を使い、既存の撮影データを維持する。
- 直近の確認端末はiPhone 13 Pro／iOS 27.2。ただし、今回追加したゼブラ／ピーキング統合版の実機表示・撮影・速度はまだ受け入れていない。

## 1. ビルド・上書きインストール

Macでプロジェクトを開き、iPhoneを接続・ロック解除する。Macとの信頼確認と開発用実行の許可が必要な場合は端末側で確認する。Xcodeの署名設定はTeam `3WH28SSRZC`を使用する。

```sh
cd /Users/Shared/Program/Tauri/Camera
xcrun devicectl list devices
./scripts/build_ios_device.sh
```

buildが成功したら、一覧で確認した対象端末のIDを指定する。次の`DEVICE_ID`は実際のIDに置き換える。

```sh
./scripts/install_ios_device.sh DEVICE_ID
```

install scriptは既存アプリをアンインストールせず、IPAを展開して上書きし、起動する。撮影データを削除する処理はない。build scriptが削除するのは生成済みbuild productのみ。

IPA: `apps/camera/src-tauri/gen/apple/build/arm64/Universal Film Camera.ipa`

build失敗時はinstallへ進まない。末尾の`code 65`だけでなく、その前にある最初の`error:`と該当箇所を保存する。iOS 27起動対策のscene manifestはbuild scriptがexport IPA内で検査する。手動で生成Info.plistだけを変更しない。

## 2. 最短の確認（約5〜10分）

- [ ] 起動して30秒程度待っても落ちない。カメラ映像が表示される。
- [ ] カメラ・マイク権限の要求が出たら、通常動作の試験では許可する。拒否試験は別に行う。
- [ ] 縦→横→縦で、被写体の上方向と映像の上方向が一致する。
- [ ] 時計・電波領域へ映像が出たり消えたりしない。メニューが映像の下へ潜らない。
- [ ] 背面カメラ自動が1項目だけある。ズーム操作で画面が固まらない。
- [ ] 写真1枚を撮り、メディアでサムネイルと原寸プレビューを開く。
- [ ] 動画を10秒程度録り、録画停止後にメディアへ完成済みとして出る。iPhoneの写真へ明示的に書き出して再生し、音声も確認する。
- [ ] 明確に色が変わるLUTを選び、撮影画面でlookをONにすると色が変わる。
- [ ] ピーキングとゼブラをON／OFFしても映像の画角・輪郭位置が飛ばない。
- [ ] ホーム画面へ戻り、アプリへ復帰しても映像が戻る。

## 3. 今回の重点：native表示補助

最初はSDR・1920×1080・30 FPS、monitor色空間はRec.709またはsRGBで試す。対応形式がない場合は選択可能なSDR形式を使い、その値を記録する。Display P3は現在WebView fallbackであり、native成功の判定に使わない。

| 操作 | 確認する結果 |
| --- | --- |
| look ON、補助OFF | LUTが反映され、映像が継続する |
| ピーキングON | 文字・物体の輪郭に色が付き、実映像の輪郭と重なる |
| 環境設定で6色を順に選択→撮影へ戻る | 選んだ色が反映され、古い色のままにならない |
| ゼブラON | 明るい部分に斜線。暗い部分を全面的に覆わない |
| LUT＋両補助ON | 映像・輪郭・斜線が同じ画角で動く |
| look OFF、補助ON | 色調だけ解除され、補助は残る |
| 全効果OFF | 普通のプレビューに戻る |
| 効果ONのまま回転・ズーム・前後カメラ切替 | 古いフレームの残留、二重回転、鏡像のずれがない |
| 設定画面へ移動→撮影へ戻る、ホームへ移動→復帰 | 黒画面・停止・旧効果の残留がない |

ピーキングはcontrast edge検出で、合焦の保証ではない。nativeはsource texelで検出し、従来の低解像度fallbackと線幅・感度が同一とは限らない。細かい模様や暗部で色が過剰に出ないかも記録する。ゼブラはLUT前の表示輝度判定で、RAWセンサー飽和警告ではない。

### native経路の診断（可能な場合）

Debug版のWebViewをMacのSafari Web Inspectorで検査できる場合、対象アプリの撮影画面を開いたままconsoleで実行する。設定画面へ移動するとpreviewは停止するため、その状態では測らない。

```js
const first = await window.__TAURI_INTERNALS__.invoke("get_native_preview_status");
await new Promise(resolve => setTimeout(resolve, 5000));
const second = await window.__TAURI_INTERNALS__.invoke("get_native_preview_status");
console.table([first, second]);
```

- `gpu_ready=true`、`gpu_error`なし、`gpu_presented`増加が目安。
- native histogramも`gpu_histogram_ready=true`、`gpu_histograms`増加が目安。LUTを切り替えると分布が変わり、ピーキング／ゼブラの色は分布に入らないことを確認する。安定した被写体と明確に色の変わるLUTを使う。
- 安定したnative表示中は`cpu_rgb_generated`が増えないことを確認。起動直後・設定変更中の少数増加はfallback準備で起こり得るため、安定後に測り直す。
- `native_lut_presented`／`allowNativeLut`という内部名は互換性のため残しており、LUT OFFの表示補助のみも含む。
- `gpu_presented`はpresent要求数で、実ディスプレイのFPSではない。`frames_replaced`も画面のdrop数ではない。
- P3／未対応入力でfallbackしても、それだけでは故障ではない。色空間と`gpu_error`を併記する。

## 4. 撮影・保存の回帰確認

- [ ] 縦／左右の横位置、背面／前面で写真を保存し、アプリ内とiPhoneの写真で向きが正しい。前面のpreview鏡像と保存画像の仕様差を、単なる位置ずれと混同しない。
- [ ] 各向きで動画を保存し、向き・音声・先頭／末尾の欠落を確認する。
- [ ] LUT ONの写真で色調が保存される。look OFFと比較できる同じ被写体を使う。
- [ ] 写真と動画の違いに注意：動画ファイルへのLUT焼き込みは未実装。previewにLUTが見えても、保存動画に焼き込まれるとは判定しない。
- [ ] FPS／フォーマット変更を適用し、警告が出ず、再起動後にも復元される。
- [ ] シャッター・EI・WB・LENS選択が動く。固定IRIS等、端末で物理変更できない項目は固定表示を確認する。
- [ ] メディアのサムネイル／詳細表示、スクロール、長押しメニューが動く。長押しでWeb画像用の別メニューが出ない。
- [ ] グリッド3種、ヒストグラム、音声レベルが動く。対応SDRのnative表示中はLUT後の分布を表示し、LUT切替に追従する。P3／未対応入力・起動待ち等のfallbackでは従来source分布へ戻る。nativeの集計はvisible cropの128×72近似sampleで、全画素の厳密分布ではない。
- [ ] シャッター音各種・外部音源を確認する。無音は端末の公開API capability制限に従う。非対応環境での消音成功を受け入れ条件にしない。

## 5. データ移動・削除の試験（テスト素材のみ）

既存の大切な写真で試さない。必ず通常の撮影・保存試験の後に実施する。

- [ ] テスト写真1枚を明示的にPhotosへ書き出し、iPhoneの写真で開けることを確認する。
- [ ] テスト写真1枚をアプリ内で削除し、サムネイルも消えることを確認する。
- [ ] 自動転送・削除をONにしてテスト写真／短い動画を撮る。Photos側に保存され、成功したものだけアプリ内から消えることを確認する。
- [ ] 権限拒否等で転送に失敗した場合、撮影済みの原本がアプリ内に残り、撮影失敗と転送失敗を区別して通知される。
- [ ] 終了後は自動転送・削除をOFFへ戻す。全件移行は別途バックアップ済みのテスト用ライブラリでのみ行う。

OSの権限設定変更や録画中のbackground移動は、テスト用録画を先に停止してから次のケースを始める。容量不足のために端末を意図的に満杯にする必要はない。安全に再現できなければPendingのままにする。

## 6. 速度・発熱と不具合の記録

同じSDR形式・同じ被写体・同じ照明で「全効果OFF」「LUTのみ」「LUT＋ピーキング＋ゼブラ」を各30〜60秒比較する。OS／端末／形式／色空間／LUT／電源接続／低電力モード／開始時の熱状態を記録する。見た目の滑らかさ、追従遅れ、発熱、暗転・停止の有無を記録し、数値計測していないFPSは推定値として断定しない。高温やOS警告が出たら試験を中止する。

```text
日時:
コードのcommit／作業差分・build日時:
端末／iOS:
写真 or 動画／解像度／FPS／レンズ:
monitor色空間／LUT／look／ピーキング色／ゼブラ:
操作手順:
期待結果:
実際の結果:
再現頻度:
native診断（前→後）:
画像／画面録画／ログの保管先:
結果: Pass / Fail / Pending
```

クラッシュ時は発生時刻・操作を残し、Xcodeの端末ログまたはiPhoneの解析データから対象アプリの最新ログを取得する。パスワード・署名秘密鍵・端末個人情報をdocsへ貼らない。スクリーンショットは被写体の個人情報を避ける。画面録画中の挙動は通常表示と負荷が違うため、その旨も記録する。

## 関連資料

- [ROADMAP](ROADMAP.md)：完了と実機Pendingの区別
- [NATIVE_PREVIEW_BRIDGE](NATIVE_PREVIEW_BRIDGE.md)：native経路の対象・制限
- [MOBILE_PLATFORM_BOOTSTRAP](MOBILE_PLATFORM_BOOTSTRAP.md)：署名／buildの背景
- [IMPLEMENTATION_HANDOFF](IMPLEMENTATION_HANDOFF.md)：変更履歴
