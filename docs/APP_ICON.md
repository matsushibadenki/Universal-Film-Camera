# Application Icon

更新日: 2026-08-31  
状態: [Done] desktop／iOS／Android向け生成済み

## デザイン意図

Universal Imaging Cameraをフィルム専用製品に見せず、スチルと動画を同格に表す。

- 外周円と6枚のブレード: 光学系／絞り
- 中央の角丸矩形: デジタルセンサー／映像フレーム
- 中央の前向き三角形: 動画、再生、Imaging Pipelineの進行方向
- graphite／gunmetal: プロ向け撮影ソフトウェアの暗色・技術的な基調
- restrained cyan: 撮影画面で使用する状態表示色

文字、製品名、フィルム孔、具体的なカメラ筐体は使わない。特定媒体や単一platformへ意味を限定せず、16–32 pxでも中央silhouetteを識別できることを優先する。

## 正本と生成物

- ベクター正本: `assets/app-icon.svg`
- 初期生成コンセプト: `assets/design/app-icon-concept.png`
- Tauri生成物: `apps/camera/src-tauri/icons/`

生成物にはPNG、ICNS、ICO、Windows Store Logo、iOS AppIcon、Android launcher／round／foregroundが含まれる。生成物を個別編集せず、必ずSVG正本を変更して再生成する。

```sh
./scripts/generate_app_icons.sh
```

このscriptはTauri CLIで全assetを生成した後、iOS AppIconだけを`rgb24`へflattenする。共通SVGはmacOS／Windows用の透明な角を持つため、`npm run tauri -- icon ...`だけで終了するとiOS PNGにもalphaが残る。App Store提出物には必ずscriptを使う。

## 実装上の制約

- 主マークは1024 px canvasの約18% safe margin内に置く。
- 小サイズで消える細線、文字、写真textureを追加しない。
- iOSはOSがcorner maskを適用するため、重要要素を角へ置かない。
- Android adaptive iconではforegroundが端末maskで切られないことを実機で確認する。
- ブランド名とbundle identifierが確定しても、媒体非依存の意味構造は維持する。

## 検証記録

2026-08-31にTauri CLIで全platform assetを再生成し、1024 px masterと32 px PNGを目視確認した。中央のcyan sensor／motion markは32 pxで判別可能。iOS AppIconはalphaを除去し、1024 px提出用assetが不透明であることを確認した。iOS SpringBoard、Android adaptive mask、Windows taskbar、macOS Dock上の実機／実OS確認はリリース前QAとして残る。
