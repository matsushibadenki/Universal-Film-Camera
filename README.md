# Universal Film Camera

Tauri 2をアプリシェルとし、Rustの共有Imaging Pipeline、Film Engine、OSネイティブのカメラ／エンコーダーを組み合わせるマルチプラットフォームカメラです。スチル撮影と動画収録の両方を対象にしています。

現在はiOSを優先して開発し、Universal Imaging Pipeline／Film Profile基盤とRust／wgpuによる写真LUT保存まで実装しています。Rust 1.88以上を使用します。写真LUTのGPU処理はRust側で所有し、その処理画素はWebViewへ転送しません。既存ライブ映像のwgpu接続は次工程です。

現在地と次の作業は [`docs/ROADMAP.md`](docs/ROADMAP.md)、GPU処理の検証・性能測定は [`docs/WGPU_RENDERER_MILESTONE.md`](docs/WGPU_RENDERER_MILESTONE.md)、全撮像工程の設計は [`docs/IMAGING_PIPELINE_ARCHITECTURE.md`](docs/IMAGING_PIPELINE_ARCHITECTURE.md)、撮影画面は [`docs/CAMERA_UI_LAYOUT.md`](docs/CAMERA_UI_LAYOUT.md)、開始手順と引き継ぎ事項は [`docs/IMPLEMENTATION_HANDOFF.md`](docs/IMPLEMENTATION_HANDOFF.md) を参照してください。

通常のbuild／testは`npm run check`、実GPUとCPU参照の比較は`npm run check:gpu`、写真保存の交互A/B計測は`npm run bench:photo-lut`で実行できます。後二者はMetalにアクセスできるApple環境が必要です。

iOSのRust compile checkは`./scripts/check_ios_rust.sh`を使用します。iOS targetを持つrustup stableを選び、Xcode 27のSwiftPM互換設定をこのコマンド内だけに適用します。
