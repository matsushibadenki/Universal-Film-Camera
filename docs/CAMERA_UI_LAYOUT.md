# Professional Camera UI Layout

更新日: 2026-08-27
対象: プロ向けスチル／動画カメラ  
トーン: 技術的・暗色・撮影画面優先

## 参考画像から採用した設計DNA

参考画像はBlackmagic Camera系のランドスケープ撮影UIである。固有ブランド、アイコン、写真を複製せず、以下の情報構造だけを採用した。

- previewを画面の主要面積として確保する
- Lens、FPS、Shutter、Iris、EI/ISO、White Balanceを上部へ常時表示する
- 現在調整中のparameterだけをaccent surfaceで強調する
- 録画／撮影操作を他のtoolから視覚的・空間的に分離する
- focus assist、zebra、guide、scopeをpreview近傍から直接切り替える
- histogramとaudio meterをpreviewへ重ね、素材を隠しすぎない
- timecodeはpreview上部中央へ置き、録画状態を位置変化なしで示す

## 本プロジェクト固有の変更

スチルと動画を同格にする。動画だけを主画面にせず、右rail／下railの最上位にPhotoとVideoを同じ寸法で配置する。シャッターボタンは同一位置を維持し、次の形状変化だけでmodeを示す。

- Photo: 白い円
- Video idle: 赤い円
- Video recording: 赤い角丸四角

Imaging PipelineはCamera、Lens、Capture Medium、Development、Print/Output、Displayを扱うため、右railの主要destinationとしてCamera画面から直接移動できる位置に置く。

## Media Catalogue

- [Done] 撮影画面と同じright／bottom railからMediaを開閉
- [Done] Finalized／Incomplete／Failedを明示する三言語state filter
- [Done] native preview layerがMedia文字を覆わないsession停止／復帰境界
- [Done] 320／375／414／768／1100pxで横overflow、見出し、操作labelを検証
- [Done] asset詳細dialog、確認付きcleanup、orphan reconciliation
- [Next] Failed／Incompleteの再検査とcapture再試行導線

Mediaは写真作品を演出するgalleryではなく、撮影素材の状態を判別するtechnical Catalogueとする。thumbnailが生成されるまでは架空画像を使わず、media種別、状態、filename、時刻、解像度、duration、asset ID、診断理由を表示する。

`AVCaptureVideoPreviewLayer`はWebViewより前面に配置されるため、Mediaへ遷移する前にnative previewを停止する。録画中または停止失敗時は遷移しない。撮影画面へ戻るとdevice discoveryからpreviewを再開する。

## Responsive layout

```text
Landscape / desktop
┌──────────────────────────────────────────┬────────┐
│ Camera · Lens · FPS · Shutter · EI · WB │ Photo  │
├──────────────────────────────────────────┤ Video  │
│                                          │ tools  │
│              Native Preview              │        │
│ histogram                    audio meter  │ shutter│
│                                          │ pages  │
└──────────────────────────────────────────┴────────┘

Portrait / narrow
┌──────────────────────────────────────────┐
│ Lens · FPS · Shutter                     │
│ Iris · EI · WB                           │
├──────────────────────────────────────────┤
│                                          │
│              Native Preview              │
│                                          │
├──────────────────────────────────────────┤
│ Photo · Video · tools · shutter · pages │
└──────────────────────────────────────────┘
```

- 全画面をSafe Area内に収める
- 左右paddingは最低16px
- rootのhorizontal overflowは禁止する
- 320 / 375 / 414 / 768px幅でclickable labelを折り返さない
- touch targetは最低44px、coarse pointerでは48px
- 960px以上でright rail、それ未満ではbottom rail
- 横向きで高さが不足する場合は説明文とrail labelを省略し、previewを優先する
- 横位置の上部情報バーはブランド表示を持たず、Lens、FPS、Shutter、Iris、EI、WB、解像度へ全幅を割り当てる
- 横位置の右railは上段をPhoto／Video、中央を正円capture、下段をmonitor／destination入口とし、capture周囲を操作禁止帯として空ける
- 横位置でmonitor／destination menuを開いた場合は上段を2×2 icon面へ置換し、native preview上へpopupを出さない

## 操作状態

すべてのbuttonはdefault、hover、focus-visible、active、disabled、loading、error、successを表現できるCSS contractを持つ。focus ringは即時表示し、hoverはfine pointerでのみ有効にする。

macOSではAVFoundation native preview、JPEG still capture、音声付きMOV recordingを接続済みである。未許可、deviceなし、起動失敗時はpreview中央に明示的な`NO CAMERA SIGNAL`状態を表示する。Video modeはmicrophone許可後だけ録画を有効化し、録画中はmode切替を無効化する。

native previewは公開APIでWKWebView上の`NSView`に配置するため、Web contentより前面に出る。現在はpreview内のHTML guide／timecode／histogram／audio meterを映像表示中だけ隠し、誤って表示済みと見せない。parameter stripとtool railはpreview外に保ち、引き続きWeb UIで操作する。次工程でoverlayをnative layerまたはMetal compositorへ移す。

monitor tools menuもnative preview上へ重ねない。bottom railではmenuを独立した上段として展開してpreviewを縮め、phone landscapeのright railでは上段を2×2 menuへ置換する。中央の正円capture controlとは別grid rowにし、menu、capture、下段navigationの間に空白を残す。開閉時はResizeObserverに加えてnative preview frame同期を明示実行する。native `NSView`はCSS `z-index`で前後関係を変更できないため、HTML popupをpreview領域へ戻してはいけない。

## Import staging directories

後から提供される素材は、アプリへ同梱する前に次のproject内directoryへ配置する。

- LUT原本: `assets/luts/`
- シャッター音原本: `assets/shutter-sounds/`

環境設定のシャッター音一覧は能力駆動とする。`shutter_sound_suppression_supported === true`のときだけ「無音」とその免責説明を同時に表示し、それ以外は両方をDOM上で非表示・選択不可にする。地域名の独自判定は持たない。

各directoryの`README.md`を受入条件の正本とする。配置だけではruntime catalogへ自動登録せず、形式検証、権利確認、音量／色変換検証を通してから組み込む。

2026-09-01、`assets/shutter-sounds`へ提供された`C137`、`F2`、`H503`、`M4`、`small`の5 WAVを形式検査し、環境設定 > 撮影 > シャッター音の「追加音源」groupへ登録した。Viteがhash付きbundle assetへ変換し、選択時とStill撮影時は同じ事前生成`HTMLAudioElement`を先頭から再生する。原本のgain、長さ、波形は変更しない。出典・作者・ライセンスとiPhone実機の音量受け入れは`assets/shutter-sounds/README.md`の`[Next]`である。

同梱後は390×844／844×390のBrowser QAで5項目の表示、選択、再読み込み保持、horizontal overflow 0、console error／warning 0を確認した。全103 test合格後に署名済みarm64 IPAを再生成し、2026-09-01 20:57にiPhone 13へinstall／launchした。

実機の「写真撮影に失敗しました：収録できません」報告を受け、Still captureではWebKit操作音を先に鳴らさない。AVCapturePhotoOutput、JPEG atomic write、asset finalizeの成功後にだけ選択音を再生し、process-wide AVAudioSessionとphoto captureの競合を避ける。Native NSErrorは今後localized descriptionに加えてdomain／codeを返す。修正版は全103 test合格後、2026-09-01 21:03にiPhone 13へinstall／launchした。

## Roadmap

- [Done] プロ向けdark camera layoutと撮影優先のpreview面を実装
- [Done] Photo/Video同格mode switch、photo feedback、24fps timecodeを実装
- [Done] parameter quick-adjust、focus/zebra/guide/scope toggleを実装
- [Done] 英語、日本語、简体中文の主要labelを維持
- [Done] macOS AVFoundation native previewを`preview-surface`へ接続
- [Done] 1100 × 760／880 × 650でnative previewのresizeとtitle bar補正を実機確認
- [Done] シャッター／録画ボタンをbottom railの水平中央、right railの垂直中央に正円で固定
- [Done] monitor tools menuをnative preview外へ配置し、中央capture controlとの重なりを解消
- [Done] Still modeの中央シャッターをJPEG実撮影へ接続
- [Done] Video modeの中央録画ボタンを音声付きMOVの開始／停止へ接続
- [Done] 録画中timecode、停止形状、mode lock、保存完了feedbackをnative stateと同期
- [Done] active resolution／FPSとmanual exposure可否をCameraCapabilities／PreviewStatusから生成
- [Done] 未接続のLens／Iris／WBと手動非対応のEI／Shutterから架空の固定値を除去
- [Done] parameter stripを72%半透明・4px blurへ調整し、露出値の可読性を保ちながら背景映像の視認性を向上
- [Done] iOS native previewをSafe Areaへ強制クリップし、回転直後も時計・電波・電池領域を黒背景に固定
- [Done] LENS選択を背面／前面／外部camera群へ整理し、「背面カメラ自動」を既定値、物理レンズを固定modeとして併記
- [Done] 背面カメラ自動ではAVFoundation仮想deviceの表示倍率（0.5×を含む）を繊細な連続zoom sliderへ接続
- [Done] format／FPS選択panelを対応組合せから生成し、sessionへ適用
- [Done] format panelを英語・日本語・简体中文へ翻訳し、44px以上の選択controlを確保
- [Done] format選択のdevice別永続化
- [Done] still／video別output preset、実filesystem残容量、概算枚数／録画分数表示
- [Done] 容量不足の三言語表示と中央capture controlの開始前無効化
- [Next] waveform、vectorscope、false color、peakingのGPU rendererを接続
- [Later] UI customization、button remapping、external monitor layout

## Verification record

2026-08-11にViteの実画面を次のviewportで確認した。

| Viewport | Root horizontal scroll | Minimum visible target | Clickable label wrap | Scope overlap |
|---|---:|---:|---:|---:|
| 320 × 568 | none | 44px | none | none |
| 375 × 812 | none | 44px | none | none |
| 414 × 896 | none | 44px | none | none |
| 768 × 1024 | none | 44px | none | none |
| 1280 × 720 | none | 50px | none | none |

320pxではbottom railをPhoto、Video、monitor tools、shutter、Pipelineへ絞り、monitor toolsは2×2 panelへ展開する。Video mode、recording state、24fps timecode、monitor panelの開閉をブラウザ操作で確認済み。

2026-08-16に署名済みdebug appと内蔵cameraでnative previewを確認した。1100 × 760ではright rail、880 × 650ではbottom railとなり、どちらもparameter strip、preview、操作railの重なりはない。カメラ映像は引き継ぎ資料へ保存しない。

2026-08-17に録画ボタンの中央配置を実機UIで再確認した。right railでは垂直中央、bottom railでは水平中央を維持し、Video modeでは赤い正円として表示される。右レールのmonitor toolsは中央領域を侵食しない展開式へ変更した。

2026-09-01にLENS／FPS／SHUTTER／IRIS／EI／WBを載せるparameter stripの背景を86%から72%へ下げ、backdrop blurを約7pxから4pxへ弱めた。選択中parameterと文字色は維持する。Browser QAは320×568、390×844、844×390で行い、全条件で6項目を表示、horizontal overflow 0、FPS panelの開閉成功、console error／warning 0を確認した。

同日、iPhoneの縦横回転後にnative previewがstatus barへ約1 Safe Area分だけ残る場合がある問題を修正した。UIKit側はDOM frameをWKWebViewのSafe Areaと毎回交差させる。Web側も回転直後のdouble `requestAnimationFrame`に加え180／420／800ms後と`visualViewport`変化時にframeを再送し、CSS環境変数の遅延確定へ追従する。390×844と844×390を2往復したBrowser QAではpreview frameが毎回同一位置へ収束し、horizontal overflow 0、console error／warning 0だった。arm64実機build、署名、iPhone 13へのinstall／launchは成功済み。status barが連続回転中も常時黒いことの最終目視は利用者受け入れ対象である。

同日、LENS quick-adjustのnative selectを「背面カメラ」「前面カメラ」「外部カメラ」のoptgroupへ整理した。撮影面には背面cameraかつ連続zoom能力がある場合だけ、1px track、11px thumb、倍率だけを持つ水平sliderを表示する。可視線は細いが操作領域は44pxを確保し、Scope表示中はHistogram／Level meterの直上へ退避する。入力は45msで間引いてAVFoundation smooth rampへ送り、前面camera選択時は即座に非表示にする。390×844／844×390でgroup、3.25×更新、前面で非表示、背面で復帰、horizontal overflow 0、console error／warning 0を確認した。

続いて背面の先頭へ「背面カメラ自動／Rear Camera Auto／后置相机自动」を追加した。対応iPhoneではTriple／Dual Wide／Dual Cameraのうち物理構成が最多のAVFoundation仮想deviceを既定値とし、0.5×から望遠まで同一sliderで操作する。個別のUltra Wide／Wide／Telephotoは固定レンズmodeとして同じ背面groupに残す。仮想device非対応時は背面物理cameraへfallbackし、存在しない「自動」をUIだけで生成しない。

AVFoundationは同じ端末でTriple／Dual Wide／Dualの複数の仮想deviceを同時に返す場合がある。これらをすべて「背面カメラ自動」と表示すると区別不能になるため、構成物理camera数が最多の既定仮想deviceだけをselectorへ表示する。UI側にも非既定仮想deviceを除外する防御を置き、backend移行中や古いpayloadでも重複を再表示しない。

Browser fixtureを390×844と844×390で再検証した。両方で初期LENSは「背面カメラ自動」、sliderは0.5×〜10×、horizontal overflowは0だった。Wide固定modeでは最小1×、前面ではslider非表示、自動へ戻すと0.5×が復帰し、3.25×入力は3.3×表示へ即時反映された。縦横の記録は`/tmp/camera-auto-rear-portrait.png`と`/tmp/camera-auto-rear-landscape.png`。一時viewport overrideは検証後にresetした。

重複修正のfixtureではTriple／Dual Wide／Dualの仮想cameraを同時投入し、390×844と844×390のLENS selectorで「背面カメラ自動」が各1件、物理cameraが各1件、horizontal overflow 0、console error／warning 0であることを確認した。記録は`/tmp/camera-auto-deduplicated.png`と`/tmp/camera-auto-deduplicated-landscape.png`。

2026-09-01、Media thumbnailの長押しをアプリ独自menuへ統一した。写真`img`は表示専用で、drag、文字選択、iOS WebKitの画像callout、pointer hit testを無効にし、親Media cardだけが420msの長押し、右click、keyboard context menuを処理する。指が10pxを超えて移動した場合は長押しをcancelし、縦scrollを優先する。390×844／844×390のBrowser QAでは「写真アプリに保存／削除」、外側tapでclose、horizontal overflow 0を確認した。iPhone上でiOS標準の拡大previewが出ないことは実機受け入れ対象である。

2026-08-30、tool railへ出力statusを追加した。StillはJPEG、Videoは現行native writerが保証するH.264／AACとcontainerだけを表示し、未実装のRAW／HEVC／LOGをpresetとして提示しない。Rustの`statvfs`結果から保存先filesystemのavailable／total bytesを取得し、JPEGは8 MiB／枚、Videoは120 MiB／分のnominal estimateとして概算量を表示する。これは保証時間ではないため、実際のbitrate／scene complexity／filesystem reservationで変動する。

同日、空き容量が256 MiBの安全予約と次の出力概算量を同時に確保できない場合、output statusをerror表示にし、Still／Videoの中央capture controlを無効化するpreflightを追加した。Rustも各capture commandの直前に同じ条件を再検査するため、UI状態だけには依存しない。録画開始後の連続監視と容量低下時の安全な自動停止は次工程である。

続いてforeground録画中は2秒間隔で保存先容量を再取得し、Videoの安全閾値を下回った場合は手動停止と同じ`stop_video_recording`へ一度だけ遷移するようにした。成功時はprobe、rename、manifest保存まで完了したasset名と自動停止理由を英語／日本語／简体中文で表示する。一時的な容量取得失敗では録画を破棄しない。WebView timerが停止し得るbackground状態はnative lifecycle監視を追加するまで保証外である。

desktopと320 × 568でdialogを操作し、320pxでは出力buttonが44 × 44px、中央capture buttonが56px正円かつviewport中心x=160px、root horizontal overflow=0であることを確認した。

容量不足fixtureでも320 × 568を再検証し、output statusがerror状態、中央capture buttonがdisabled、56 × 56px正円、中心x=160px、horizontal overflow=0であることを確認した。console error／warningは0件だった。

2026-08-24、monitor toolsを固定popupとしてpreviewへ重ねる実装では、前面にあるnative `NSView`がmenu文字を隠す問題を確認した。menuをtool rail内へ戻し、320 × 700ではpreview下の独立行、880 × 650でもbottom rail内、1100 × 760ではright rail内へ配置した。1100 × 760では4つのtool buttonとcapture controlの矩形が非重複、capture中心とrail中心が一致、320 × 700ではmenu上端がpreview下端より12px下、horizontal overflowなしを実画面で確認した。

2026-08-18にStill modeの中央シャッターから実機JPEGを保存し、処理中のdisabled状態と完了後の復帰を確認した。成功時はボタン状態と多言語statusを更新する。この時点のVideo modeは疑似timerを開始せず、実収録が接続されるまでボタンをdisabledに保っていた。

2026-08-20にVideo modeの中央録画ボタンから実機MOVを開始／停止した。録画中はボタンが「録画停止」へ変わり、Still／Video mode switchはdisabled、停止完了後は「録画」へ復帰した。保存結果はH.264 1920 × 1080 + AAC 48 kHz mono、5.671秒。疑似timerではなくnative recording lifecycleへ接続済みである。

同日の能力値接続では、内蔵cameraのactive formatとして1920 × 1080／30 FPSをAVFoundationから取得し、上部表示を`1080p · 1920×1080 · 30 FPS · SDR`へ更新した。手動shutter／ISO非対応は`AUTO`かつdisabled、能力モデル未接続のLens／Irisは`—`、WBは`AUTO`とし、従来の固定4K／24／LOG／35mm／T2.8／5600Kを実機値として誤表示しない。

続いてFPS parameterから開くdark format panelを追加した。resolutionを変えた時は現在のFPSが新しいformatでも対応していれば保持し、非対応なら最初の対応値へ移る。録画中はpanelを閉じ、選択controlを無効化する。内蔵cameraで1920 × 1080／30 FPSから1280 × 720／24 FPSへ変更し、panelが閉じた後にFPS parameterとsummaryが`720p · 1280×720 · 24 FPS · SDR`へ同期することを実画面で確認した。
