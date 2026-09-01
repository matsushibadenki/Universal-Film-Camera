import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "./style.css";

type CameraMode = "still" | "video";
type Locale = "en" | "ja" | "zh-CN";
type Tool = "focus" | "zebra" | "guides" | "scope";
type CameraMonitorSnapshot = { red: number[]; green: number[]; blue: number[]; audio_db: number[]; frame_received: boolean; preview_width: number; preview_height: number; preview_rgb_base64: string };
type LutPayload = { size: number; samples: [number, number, number][]; domain_min: [number, number, number]; domain_max: [number, number, number] };
type LutEntry = { id: string; name: string; category: string; source: "built_in" | "imported"; size: number };
type LutCatalog = { built_in: LutEntry[]; imported: LutEntry[] };
type CameraAuthorization = "not_determined" | "restricted" | "denied" | "authorized" | "unavailable";
type CameraDevice = { id: string; label: string; position: "front" | "back" | "external" | "unspecified" };
type CameraDiscovery = { authorization: CameraAuthorization; devices: CameraDevice[] };
type CameraRuntimeHealth = { preview_attached: boolean; session_running: boolean; recording_pending: boolean };
type CameraCapabilities = {
  supports_still: boolean;
  supports_video: boolean;
  supports_audio: boolean;
  resolutions: [number, number][];
  frame_rates: number[];
  formats: { width: number; height: number; frame_rates: number[] }[];
  manual_iso: [number, number] | null;
  manual_shutter: boolean;
  manual_focus: boolean;
  lens_label?: string | null;
  lens_aperture?: number | null;
  current_shutter_seconds?: number | null;
  current_iso?: number | null;
  manual_white_balance?: boolean;
  current_white_balance_kelvin?: number | null;
  raw_photo: boolean;
  log_video: boolean;
  hdr_video: boolean;
};
type PreviewViewport = { x: number; y: number; width: number; height: number };
type CaptureOrientation = {
  rotation_degrees: 0 | 90 | 180 | 270;
  preview_mirrored: boolean;
  capture_mirrored: boolean;
};
type PreviewStatus = {
  running: boolean;
  device_id: string;
  active_format: PreviewFormat | null;
  format_restored: boolean;
  settings_warning: string | null;
  orientation: CaptureOrientation;
};
type PreviewFormat = {
  width: number;
  height: number;
  fps: number;
  settings_persisted: boolean;
  settings_warning: string | null;
};
type CaptureResource = {
  path: string;
  pixel_width: number;
  pixel_height: number;
  frame_rate: { numerator: number; denominator: number } | null;
  duration_ms: number | null;
};
type RenderProfileSnapshot = {
  schema_version: number;
  pipeline_id: string;
  pipeline_sha256: string;
  profiles: {
    id: string;
    kind: string;
    profile_version: string;
    content_sha256: string;
  }[];
  snapshot_sha256: string;
};
type CaptureAsset = {
  schema_version: number;
  id: string;
  media_type: "photo" | "video";
  state: "incomplete" | "finalized" | "failed";
  original_resource_id: string;
  original: CaptureResource;
  derivatives: {
    resource_id: string;
    purpose: "processed" | "thumbnail" | "proxy" | "export";
    resource: CaptureResource;
    provenance: {
      parent_resource_id: string;
      render_snapshot: RenderProfileSnapshot;
      engine_version: string;
      seed: number;
    };
    created_at_utc: string;
  }[];
  validation: { status: "passed" | "warning" | "failed" };
};
type MediaState = CaptureAsset["state"];
type MediaIndexEntry = {
  schema_version: number;
  id: string;
  state: MediaState;
  media_type: CaptureAsset["media_type"];
  resource_path: string;
  asset: CaptureAsset | null;
  error: string | null;
  updated_at_utc: string;
};
type RecoverableCleanupCandidate = { entry: MediaIndexEntry; age_seconds: number; retention_expired: boolean };
type MediaFilter = "all" | MediaState;
type MediaView = "thumbnails" | "details";
type PhotoPreviewPayload = { id: string; mime_type: string; data_base64: string };
type BulkPhotoMigrationResult = { exported: number; deleted: number };
type CaptureOutputPreset = {
  id: string;
  media_type: "photo" | "video";
  container: string;
  video_codec: string | null;
  audio_codec: string | null;
  estimated_bytes_per_unit: number;
};
type CaptureOutputPresets = { still: CaptureOutputPreset[]; video: CaptureOutputPreset[] };
type CaptureStorageStatus = {
  path: string;
  available_bytes: number;
  total_bytes: number;
  photo_ready: boolean;
  video_ready: boolean;
};
type NearbyPeer = {
  ephemeral_id: string;
  display_label: string | null;
  protocol_version: number;
  addresses: string[];
  port: number;
};
type NearbyDiscoverySnapshot = {
  supported?: boolean;
  active: boolean;
  local_peer: { ephemeral_id: string; display_label: string | null; port: number } | null;
  peers: NearbyPeer[];
  last_error: string | null;
  approval?: {
    invitation_id: string;
    peer_ephemeral_id: string;
    asset_id: string;
    file_name: string;
    byte_length: number;
    confirmation_code: string;
    expires_at_unix_ms: number;
    local_approved: boolean;
    remote_approved?: boolean;
    direction?: "outgoing" | "incoming";
    transferred_bytes?: number;
    transfer_active?: boolean;
    cancel_requested?: boolean;
    retry_available?: boolean;
    failure_kind?: "disconnected" | "timeout" | "integrity" | "storage" | "invitation_expired" | "cancelled" | "protocol";
    finalized?: boolean;
  } | null;
};

type Copy = {
  photo: string;
  video: string;
  camera: string;
  noSignal: string;
  backendMessage: string;
  nativePreview: string;
  record: string;
  stop: string;
  capture: string;
  captured: string;
  captureFailed: string;
  videoSaved: string;
  recordingFailed: string;
  microphoneDenied: string;
  pipeline: string;
  media: string;
  settings: string;
  settingsTitle: string;
  settingsSubtitle: string;
  settingsBack: string;
  settingsDisplay: string;
  settingsCapture: string;
  settingsColor: string;
  monitoringColorSpace: string;
  workingColorSpace: string;
  workingColorSpaceDetail: string;
  colorSpaceSaved: string;
  monitorLook: string;
  monitorLookDetail: string;
  settingsDisplayDetail: string;
  peakingColor: string;
  peakingColorSaved: string;
  colorCyan: string;
  colorRed: string;
  colorGreen: string;
  colorYellow: string;
  colorMagenta: string;
  colorWhite: string;
  settingsCaptureDetail: string;
  shutterSound: string;
  shutterSoundStandard: string;
  shutterSoundFresh: string;
  shutterSoundDslr: string;
  shutterSoundSilent: string;
  shutterSoundDetail: string;
  shutterSoundSaved: string;
  shutterSoundImport: string;
  shutterSoundImportHint: string;
  shutterSoundCustom: string;
  shutterSoundImported: string;
  shutterSoundImportFailed: string;
  settingsLut: string;
  settingsMedia: string;
  settingsMediaDetail: string;
  guideStyle: string;
  guideThirds: string;
  guideGrid: string;
  guideDiagonal: string;
  bulkPhotoTitle: string;
  bulkPhotoDetail: string;
  bulkPhotoAction: string;
  bulkPhotoEmpty: string;
  bulkPhotoConfirmTitle: string;
  bulkPhotoConfirm: string;
  bulkPhotoCancel: string;
  bulkPhotoRunning: string;
  bulkPhotoComplete: string;
  bulkPhotoFailed: string;
  lutSelection: string;
  lutImport: string;
  lutImportHint: string;
  lutImported: string;
  lutImportFailed: string;
  lutBuiltIn: string;
  lutExternal: string;
  lutAccuracyNote: string;
  focus: string;
  zebra: string;
  guides: string;
  scopes: string;
  close: string;
  adjust: string;
  allowCamera: string;
  requesting: string;
  denied: string;
  restricted: string;
  unavailable: string;
  noDevice: string;
  detected: string;
  previewPending: string;
  previewStarting: string;
  previewFailed: string;
  format: string;
  apply: string;
  formatFailed: string;
  formatPersistenceFailed: string;
  assetMetadataWarning: string;
  orientationFailed: string;
  mediaTitle: string;
  mediaSubtitle: string;
  mediaAll: string;
  mediaReady: string;
  mediaIncomplete: string;
  mediaFailed: string;
  mediaEmpty: string;
  mediaEmptyDetail: string;
  mediaLoading: string;
  mediaLoadFailed: string;
  mediaBack: string;
  mediaRefresh: string;
  mediaView: string;
  mediaThumbnails: string;
  mediaDetailed: string;
  mediaPhoto: string;
  mediaVideo: string;
  mediaDuration: string;
  mediaAwaiting: string;
  mediaValidationFailed: string;
  mediaDetails: string;
  mediaPath: string;
  mediaUpdated: string;
  mediaState: string;
  mediaCleanup: string;
  mediaCleanupTitle: string;
  mediaCleanupPrompt: string;
  mediaCleanupConfirm: string;
  mediaCleanupCancel: string;
  mediaCleanupFailed: string;
  mediaReinspect: string;
  mediaReinspecting: string;
  mediaReinspectFailed: string;
  mediaRecapture: string;
  mediaCleanupExpired: string;
  mediaCleanupExpiredPrompt: string;
  mediaMenu: string;
  mediaDelete: string;
  mediaDeleteTitle: string;
  mediaDeletePrompt: string;
  mediaDeleteFailed: string;
  mediaSavePhotos: string;
  mediaSavingPhotos: string;
  mediaSavedPhotos: string;
  mediaSavePhotosFailed: string;
  output: string;
  outputPreset: string;
  storageRemaining: string;
  estimatedCapacity: string;
  photosRemaining: string;
  minutesRemaining: string;
  storageUnavailable: string;
  storageLow: string;
  storageAutoStop: string;
  nearby: string;
  nearbyTitle: string;
  nearbySubtitle: string;
  nearbyStart: string;
  nearbyStop: string;
  nearbyRefresh: string;
  nearbyBack: string;
  nearbySearching: string;
  nearbyEmpty: string;
  nearbyEmptyDetail: string;
  nearbyLocal: string;
  nearbyProtocol: string;
  nearbyAddress: string;
  nearbyPrivacy: string;
  nearbyAsset: string;
  nearbySelectAsset: string;
  nearbyPrepare: string;
  nearbyCodeTitle: string;
  nearbyCodeDetail: string;
  nearbyApprove: string;
  nearbyCancel: string;
  nearbyApproved: string;
  nearbyIncoming: string;
  nearbySecure: string;
  nearbyTransferring: string;
  nearbyProgress: string;
  nearbyCancelTransfer: string;
  nearbyCancelling: string;
  nearbyRetry: string;
  nearbyReconnecting: string;
  nearbyInterrupted: string;
  nearbyFailureTimeout: string;
  nearbyFailureIntegrity: string;
  nearbyFailureStorage: string;
  nearbyFailureExpired: string;
  nearbyFailureCancelled: string;
  nearbyFailureProtocol: string;
  nearbyFailed: string;
  nearbyDiscard: string;
  nearbyDiscardTitle: string;
  nearbyDiscardPrompt: string;
  nearbyDiscardConfirm: string;
  nearbyKeepPartial: string;
  nearbyNewApproval: string;
  nearbyPrepareAgain: string;
  nearbyComplete: string;
};

const copy: Record<Locale, Copy> = {
  en: {
    photo: "Photo", video: "Video", camera: "Camera", noSignal: "NO CAMERA SIGNAL",
    backendMessage: "Camera access is required for native preview.", nativePreview: "Native camera preview",
    record: "Record", stop: "Stop recording", capture: "Capture photo", captured: "Photo saved", captureFailed: "Photo capture failed",
    videoSaved: "Video saved", recordingFailed: "Video recording failed", microphoneDenied: "Microphone access is required for video recording.",
    pipeline: "Pipeline", media: "Media", settings: "Settings", settingsTitle: "Settings",
    settingsSubtitle: "Camera, monitoring, and color management", settingsBack: "Back to camera",
    settingsDisplay: "Display", settingsCapture: "Capture", settingsColor: "Color",
    monitoringColorSpace: "Monitoring color space", workingColorSpace: "Internal working space",
    workingColorSpaceDetail: "Scene-linear ACEScg · fixed by the imaging pipeline", colorSpaceSaved: "Color-space preference saved",
    monitorLook: "Apply color profile and LUT to live preview", monitorLookDetail: "Monitoring only · captured originals are unchanged",
    settingsDisplayDetail: "Dark technical interface · monitoring overlays are controlled from the camera screen",
    peakingColor: "Peaking color", peakingColorSaved: "Peaking color saved",
    colorCyan: "Cyan", colorRed: "Red", colorGreen: "Green", colorYellow: "Yellow", colorMagenta: "Magenta", colorWhite: "White",
    settingsCaptureDetail: "Photo and video remain equal capture modes · format is configured on the camera screen",
    shutterSound: "Shutter sound", shutterSoundStandard: "Standard shutter", shutterSoundFresh: "Fresh", shutterSoundDslr: "DSLR", shutterSoundSilent: "Silent",
    shutterSoundDetail: "Silent requests the official iOS suppression mode. A mandatory system shutter still sounds on unsupported devices or in restricted regions.", shutterSoundSaved: "Shutter sound saved",
    shutterSoundImport: "Import sound file", shutterSoundImportHint: "MP3, M4A, WAV, or CAF · up to 5 MiB", shutterSoundCustom: "Custom", shutterSoundImported: "Shutter sound imported", shutterSoundImportFailed: "Could not import shutter sound",
    settingsLut: "LUT", settingsMedia: "Media", settingsMediaDetail: "Move all finalized photos to Apple Photos, then remove their app copies.",
    guideStyle: "Guide style", guideThirds: "3 × 3 thirds", guideGrid: "Grid", guideDiagonal: "Diagonals",
    bulkPhotoTitle: "Move all photos to Apple Photos", bulkPhotoDetail: "Videos and recovery files are not affected. App copies are deleted only after every photo is saved successfully.",
    bulkPhotoAction: "Save all and delete app copies", bulkPhotoEmpty: "There are no finalized photos to move.",
    bulkPhotoConfirmTitle: "Move and delete all app photos?", bulkPhotoConfirm: "Save every finalized photo to Apple Photos, then permanently delete all transferred copies from this app.",
    bulkPhotoCancel: "Cancel", bulkPhotoRunning: "Saving photos… Keep the app open.", bulkPhotoComplete: "All photos were saved and app copies were deleted.", bulkPhotoFailed: "Migration stopped. App copies were retained unless all exports had completed.",
    lutSelection: "Film-look LUT", lutImport: "Import .cube LUT", lutImportHint: "3D .cube · 2–65 grid · up to 4 MiB",
    lutImported: "LUT imported", lutImportFailed: "LUT import failed", lutBuiltIn: "Built-in film archetypes", lutExternal: "Imported LUTs",
    lutAccuracyNote: "Built-ins represent generic film/process families, not measured reproductions of named film stocks.",
    focus: "Focus peaking",
    zebra: "Zebra", guides: "Frame guides", scopes: "Scopes", close: "Close", adjust: "Adjust",
    allowCamera: "Allow camera access", requesting: "Requesting camera access…",
    denied: "Camera access is denied. Enable it in System Settings.",
    restricted: "Camera access is restricted on this device.", unavailable: "Camera backend is unavailable on this platform.",
    noDevice: "No camera device was detected.", detected: "CAMERA DETECTED", previewPending: "Native preview is ready to start.",
    previewStarting: "Starting native preview…", previewFailed: "Native preview could not be started.",
    format: "Format", apply: "Apply", formatFailed: "Format change failed",
    formatPersistenceFailed: "Format applied, but the setting could not be saved",
    assetMetadataWarning: "saved with a metadata warning", orientationFailed: "Camera orientation sync failed",
    mediaTitle: "Media", mediaSubtitle: "Captured assets and recovery states", mediaAll: "All", mediaReady: "Ready",
    mediaIncomplete: "Incomplete", mediaFailed: "Failed", mediaEmpty: "No captured media",
    mediaEmptyDetail: "Photos and videos appear here after their manifest is safely stored.", mediaLoading: "Loading media…",
    mediaLoadFailed: "The media index could not be read.", mediaBack: "Back to camera", mediaRefresh: "Refresh media",
    mediaView: "Media view", mediaThumbnails: "Thumbnails", mediaDetailed: "Details",
    mediaPhoto: "Photo", mediaVideo: "Video", mediaDuration: "Duration", mediaAwaiting: "Awaiting validation",
    mediaValidationFailed: "Validation failed", mediaDetails: "View details", mediaPath: "Resource path",
    mediaUpdated: "Updated", mediaState: "State", mediaCleanup: "Clean up recoverable file",
    mediaCleanupTitle: "Remove recoverable media?", mediaCleanupPrompt: "This permanently removes the incomplete or failed resource and its diagnostic manifest.",
    mediaCleanupConfirm: "Remove file", mediaCleanupCancel: "Keep file", mediaCleanupFailed: "The recoverable media could not be removed.",
    mediaReinspect: "Reinspect file", mediaReinspecting: "Reinspecting media…", mediaReinspectFailed: "The media could not be reinspected.",
    mediaRecapture: "Recapture", mediaCleanupExpired: "Review expired recovery files", mediaCleanupExpiredPrompt: "Remove all selected recovery files older than 7 days? Finalized media is protected."
    , mediaMenu: "Media actions", mediaDelete: "Delete", mediaDeleteTitle: "Delete this media?",
    mediaDeletePrompt: "This permanently removes the original media, its derivatives, and media record from this app.",
    mediaDeleteFailed: "The media could not be deleted.", mediaSavePhotos: "Save to Photos",
    mediaSavingPhotos: "Saving to Photos…", mediaSavedPhotos: "Saved to Photos.",
    mediaSavePhotosFailed: "The photo could not be saved to Photos. Check Photos access in Settings."
    , output: "Output", outputPreset: "Output preset", storageRemaining: "Storage remaining",
    estimatedCapacity: "Estimated capacity", photosRemaining: "photos", minutesRemaining: "minutes", storageUnavailable: "Storage information unavailable", storageLow: "Not enough free space", storageAutoStop: "Recording stopped safely because storage is low",
    nearby: "Nearby", nearbyTitle: "Nearby Share", nearbySubtitle: "Discover nearby Universal Film Camera users",
    nearbyStart: "Start discovery", nearbyStop: "Stop discovery", nearbyRefresh: "Refresh peers", nearbyBack: "Back to camera",
    nearbySearching: "Visible nearby · searching for peers…", nearbyEmpty: "No nearby users found",
    nearbyEmptyDetail: "Keep this screen open on both devices. Discovery stops when you return to the camera.",
    nearbyLocal: "Your temporary ID", nearbyProtocol: "Protocol", nearbyAddress: "Network path",
    nearbyPrivacy: "Only an ephemeral ID is advertised. Transfer still requires mutual approval and a matching confirmation code.",
    nearbyAsset: "Media to share", nearbySelectAsset: "Select finalized media", nearbyPrepare: "Prepare approval",
    nearbyCodeTitle: "Compare confirmation code", nearbyCodeDetail: "Confirm that this exact code appears on both devices before approving.",
    nearbyApprove: "Code matches · Approve", nearbyCancel: "Cancel", nearbyApproved: "Approved locally · waiting for the other device",
    nearbyIncoming: "Incoming share", nearbySecure: "Secure session established",
    nearbyTransferring: "Encrypted transfer in progress…", nearbyProgress: "Transferred",
    nearbyCancelTransfer: "Cancel transfer", nearbyCancelling: "Cancelling safely…",
    nearbyRetry: "Reconnect and resume", nearbyReconnecting: "Reconnecting securely…",
    nearbyInterrupted: "Connection interrupted · verified partial data is preserved",
    nearbyFailureTimeout: "Connection timed out · verified partial data is preserved",
    nearbyFailureIntegrity: "Integrity verification failed · media was not published",
    nearbyFailureStorage: "Not enough storage · free space before trying again",
    nearbyFailureExpired: "Approval expired · prepare a new confirmation code",
    nearbyFailureCancelled: "Transfer cancelled · verified partial data remains recoverable",
    nearbyFailureProtocol: "Secure transfer protocol failed · start a new approval",
    nearbyFailed: "Transfer stopped",
    nearbyDiscard: "Discard partial data", nearbyDiscardTitle: "Discard received partial data?",
    nearbyDiscardPrompt: "This permanently removes only this transfer's verified partial file and recovery ledger. Finalized media is never removed.",
    nearbyDiscardConfirm: "Discard partial", nearbyKeepPartial: "Keep for recovery",
    nearbyNewApproval: "Prepare new approval", nearbyPrepareAgain: "Select Prepare approval to create a new confirmation code.",
    nearbyComplete: "Transfer complete"
  },
  ja: {
    photo: "写真", video: "動画", camera: "カメラ", noSignal: "カメラ信号なし",
    backendMessage: "ネイティブプレビューにはカメラへのアクセスが必要です。", nativePreview: "ネイティブカメラプレビュー",
    record: "録画", stop: "録画停止", capture: "写真撮影", captured: "写真を保存しました", captureFailed: "写真撮影に失敗しました",
    videoSaved: "動画を保存しました", recordingFailed: "動画収録に失敗しました", microphoneDenied: "動画収録にはマイクへのアクセスが必要です。",
    pipeline: "パイプライン", media: "メディア", settings: "環境設定", settingsTitle: "環境設定",
    settingsSubtitle: "カメラ・モニター・カラー管理", settingsBack: "カメラへ戻る",
    settingsDisplay: "表示", settingsCapture: "撮影", settingsColor: "カラー",
    monitoringColorSpace: "モニタリングカラースペース", workingColorSpace: "内部作業色空間",
    workingColorSpaceDetail: "Scene-linear ACEScg・Imaging Pipelineで固定", colorSpaceSaved: "カラースペース設定を保存しました",
    monitorLook: "カラープロファイルとLUTをライブ表示へ適用", monitorLookDetail: "モニタリング専用・撮影原本は変更しません",
    settingsDisplayDetail: "技術的なダークUI・モニター表示は撮影画面から操作します",
    peakingColor: "ピーキングカラー", peakingColorSaved: "ピーキングカラーを保存しました",
    colorCyan: "シアン", colorRed: "赤", colorGreen: "緑", colorYellow: "黄", colorMagenta: "マゼンタ", colorWhite: "白",
    settingsCaptureDetail: "写真と動画を同等に扱います・フォーマットは撮影画面から設定します",
    shutterSound: "シャッター音", shutterSoundStandard: "通常のシャッター音", shutterSoundFresh: "爽やかな音", shutterSoundDslr: "一眼レフカメラ", shutterSoundSilent: "無音",
    shutterSoundDetail: "「無音」ではiOSの正式な消音機能も要求します。非対応端末や消音が制限される地域では、システムシャッター音が鳴ります。", shutterSoundSaved: "シャッター音を保存しました",
    shutterSoundImport: "音源ファイルを読み込む", shutterSoundImportHint: "MP3・M4A・WAV・CAF、最大5 MiB", shutterSoundCustom: "カスタム", shutterSoundImported: "シャッター音を読み込みました", shutterSoundImportFailed: "シャッター音を読み込めませんでした",
    settingsLut: "LUT", settingsMedia: "メディア", settingsMediaDetail: "完成済み写真をすべて写真アプリへ移し、転送後にアプリ内のコピーを削除します。",
    guideStyle: "グリッド形式", guideThirds: "3分割（3×3）", guideGrid: "方眼（格子線）", guideDiagonal: "対角線",
    bulkPhotoTitle: "すべての写真を写真アプリへ移行", bulkPhotoDetail: "動画と復旧対象ファイルには影響しません。全写真の保存成功後にだけアプリ内コピーを削除します。",
    bulkPhotoAction: "すべて保存してアプリ内コピーを削除", bulkPhotoEmpty: "移行できる完成済み写真はありません。",
    bulkPhotoConfirmTitle: "すべての写真を移行して削除しますか？", bulkPhotoConfirm: "完成済み写真をすべて写真アプリへ保存し、転送済みのアプリ内コピーを完全に削除します。",
    bulkPhotoCancel: "キャンセル", bulkPhotoRunning: "写真を保存しています。アプリを開いたままにしてください…", bulkPhotoComplete: "すべての写真を保存し、アプリ内コピーを削除しました。", bulkPhotoFailed: "移行を中止しました。全件の書き出しが完了していない場合、アプリ内コピーは保持されています。",
    lutSelection: "フィルム調LUT", lutImport: ".cube LUTを読み込む", lutImportHint: "3D .cube・2〜65グリッド・最大4 MiB",
    lutImported: "LUTを読み込みました", lutImportFailed: "LUTを読み込めませんでした", lutBuiltIn: "内蔵フィルム調アーキタイプ", lutExternal: "読み込んだLUT",
    lutAccuracyNote: "内蔵LUTは一般的なフィルム／現像系統の色調で、実在銘柄の測色再現ではありません。",
    focus: "ピーキング",
    zebra: "ゼブラ", guides: "ガイド", scopes: "スコープ", close: "閉じる", adjust: "調整",
    allowCamera: "カメラへのアクセスを許可", requesting: "カメラ権限を確認しています…",
    denied: "カメラへのアクセスが拒否されています。システム設定で許可してください。",
    restricted: "このデバイスではカメラへのアクセスが制限されています。", unavailable: "このプラットフォームではカメラバックエンドを利用できません。",
    noDevice: "カメラデバイスが見つかりません。", detected: "カメラ検出済み", previewPending: "ネイティブプレビューを開始できます。",
    previewStarting: "ネイティブプレビューを開始しています…", previewFailed: "ネイティブプレビューを開始できませんでした。",
    format: "フォーマット", apply: "適用", formatFailed: "フォーマット変更に失敗しました",
    formatPersistenceFailed: "フォーマットは適用されましたが、設定を保存できませんでした",
    assetMetadataWarning: "メタデータ警告付きで保存しました", orientationFailed: "カメラ姿勢の同期に失敗しました",
    mediaTitle: "メディア", mediaSubtitle: "撮影素材と復旧状態", mediaAll: "すべて", mediaReady: "完了",
    mediaIncomplete: "未完了", mediaFailed: "失敗", mediaEmpty: "撮影素材はありません",
    mediaEmptyDetail: "写真と動画は、安全にマニフェストを保存した後でここに表示されます。", mediaLoading: "メディアを読み込んでいます…",
    mediaLoadFailed: "メディアインデックスを読み込めませんでした。", mediaBack: "カメラへ戻る", mediaRefresh: "メディアを更新",
    mediaView: "メディア表示", mediaThumbnails: "サムネイル", mediaDetailed: "詳細",
    mediaPhoto: "写真", mediaVideo: "動画", mediaDuration: "長さ", mediaAwaiting: "検証待ち",
    mediaValidationFailed: "検証失敗", mediaDetails: "詳細を表示", mediaPath: "リソースパス",
    mediaUpdated: "更新日時", mediaState: "状態", mediaCleanup: "復旧対象ファイルを削除",
    mediaCleanupTitle: "復旧対象メディアを削除しますか？", mediaCleanupPrompt: "未完了または失敗したリソースと診断マニフェストを完全に削除します。",
    mediaCleanupConfirm: "ファイルを削除", mediaCleanupCancel: "ファイルを残す", mediaCleanupFailed: "復旧対象メディアを削除できませんでした。",
    mediaReinspect: "ファイルを再検査", mediaReinspecting: "メディアを再検査しています…", mediaReinspectFailed: "メディアを再検査できませんでした。",
    mediaRecapture: "再撮影", mediaCleanupExpired: "期限切れ復旧ファイルを確認", mediaCleanupExpiredPrompt: "7日を超えた復旧ファイルをすべて削除しますか？完了済みメディアは保護されます。"
    , mediaMenu: "メディア操作", mediaDelete: "削除", mediaDeleteTitle: "このメディアを削除しますか？",
    mediaDeletePrompt: "このアプリ内の原本メディア、派生データ、メディア記録を完全に削除します。",
    mediaDeleteFailed: "メディアを削除できませんでした。", mediaSavePhotos: "写真アプリに保存",
    mediaSavingPhotos: "写真アプリに保存しています…", mediaSavedPhotos: "写真アプリに保存しました。",
    mediaSavePhotosFailed: "写真アプリに保存できませんでした。設定で写真へのアクセスを確認してください。"
    , output: "出力", outputPreset: "出力プリセット", storageRemaining: "残容量",
    estimatedCapacity: "推定撮影可能量", photosRemaining: "枚", minutesRemaining: "分", storageUnavailable: "残容量を取得できません", storageLow: "空き容量が不足しています", storageAutoStop: "空き容量が少ないため安全に録画を停止しました",
    nearby: "近距離共有", nearbyTitle: "近距離共有", nearbySubtitle: "近くのUniversal Film Cameraユーザーを検出",
    nearbyStart: "検出を開始", nearbyStop: "検出を停止", nearbyRefresh: "相手を更新", nearbyBack: "カメラへ戻る",
    nearbySearching: "周囲へ一時公開中・相手を検索しています…", nearbyEmpty: "近くのユーザーが見つかりません",
    nearbyEmptyDetail: "両方の端末でこの画面を開いてください。カメラへ戻ると検出を停止します。",
    nearbyLocal: "あなたの一時ID", nearbyProtocol: "プロトコル", nearbyAddress: "ネットワーク経路",
    nearbyPrivacy: "周囲へ公開するのは一時IDだけです。転送には双方の承認と一致する確認コードが必要です。",
    nearbyAsset: "共有するメディア", nearbySelectAsset: "完了メディアを選択", nearbyPrepare: "承認を準備",
    nearbyCodeTitle: "確認コードを比較", nearbyCodeDetail: "承認する前に、両方の端末へ同じコードが表示されていることを確認してください。",
    nearbyApprove: "コード一致・承認", nearbyCancel: "キャンセル", nearbyApproved: "この端末で承認済み・相手の承認待ち",
    nearbyIncoming: "受信する共有", nearbySecure: "安全なセッションを確立しました",
    nearbyTransferring: "暗号化転送中…", nearbyProgress: "転送済み",
    nearbyCancelTransfer: "転送をキャンセル", nearbyCancelling: "安全に停止しています…",
    nearbyRetry: "再接続して再開", nearbyReconnecting: "安全に再接続しています…",
    nearbyInterrupted: "接続が中断されました・検証済みの受信データは保持されています",
    nearbyFailureTimeout: "接続がタイムアウトしました・検証済みの受信データは保持されています",
    nearbyFailureIntegrity: "完全性検証に失敗しました・メディアは公開されていません",
    nearbyFailureStorage: "保存容量が不足しています・空き容量を確保してください",
    nearbyFailureExpired: "承認期限が切れました・新しい確認コードを準備してください",
    nearbyFailureCancelled: "転送をキャンセルしました・検証済みデータは復旧可能です",
    nearbyFailureProtocol: "安全な転送手順に失敗しました・承認をやり直してください",
    nearbyFailed: "転送を停止しました",
    nearbyDiscard: "受信途中データを破棄", nearbyDiscardTitle: "受信途中データを破棄しますか？",
    nearbyDiscardPrompt: "この転送の検証済みpartial fileと復旧ledgerだけを完全に削除します。Finalizedメディアは削除しません。",
    nearbyDiscardConfirm: "途中データを破棄", nearbyKeepPartial: "復旧用に保持",
    nearbyNewApproval: "新しい承認を準備", nearbyPrepareAgain: "「承認を準備」を選び、新しい確認コードを作成してください。",
    nearbyComplete: "転送が完了しました"
  },
  "zh-CN": {
    photo: "照片", video: "视频", camera: "相机", noSignal: "无相机信号",
    backendMessage: "原生预览需要相机访问权限。", nativePreview: "原生相机预览",
    record: "录制", stop: "停止录制", capture: "拍照", captured: "照片已保存", captureFailed: "拍照失败",
    videoSaved: "视频已保存", recordingFailed: "视频录制失败", microphoneDenied: "视频录制需要麦克风访问权限。",
    pipeline: "成像管线", media: "媒体", settings: "设置", settingsTitle: "环境设置",
    settingsSubtitle: "相机、监看与色彩管理", settingsBack: "返回相机",
    settingsDisplay: "显示", settingsCapture: "拍摄", settingsColor: "色彩",
    monitoringColorSpace: "监看色彩空间", workingColorSpace: "内部工作色彩空间",
    workingColorSpaceDetail: "Scene-linear ACEScg・由成像管线固定", colorSpaceSaved: "色彩空间设置已保存",
    monitorLook: "将色彩配置与 LUT 应用于实时预览", monitorLookDetail: "仅用于监看・不会更改拍摄原片",
    settingsDisplayDetail: "技术型深色界面・监看叠加层可在拍摄画面中控制",
    peakingColor: "峰值对焦颜色", peakingColorSaved: "峰值对焦颜色已保存",
    colorCyan: "青色", colorRed: "红色", colorGreen: "绿色", colorYellow: "黄色", colorMagenta: "品红色", colorWhite: "白色",
    settingsCaptureDetail: "照片与视频为同等拍摄模式・格式可在拍摄画面中设置",
    shutterSound: "快门声音", shutterSoundStandard: "标准快门", shutterSoundFresh: "清爽音效", shutterSoundDslr: "单反相机", shutterSoundSilent: "静音",
    shutterSoundDetail: "“静音”也会请求 iOS 官方静音功能。在不支持的设备或受限制地区，系统快门声仍会响起。", shutterSoundSaved: "快门声音已保存",
    shutterSoundImport: "导入声音文件", shutterSoundImportHint: "MP3、M4A、WAV 或 CAF・最大 5 MiB", shutterSoundCustom: "自定义", shutterSoundImported: "快门声音已导入", shutterSoundImportFailed: "无法导入快门声音",
    settingsLut: "LUT", settingsMedia: "媒体", settingsMediaDetail: "将所有已完成照片移到 Apple 照片，然后删除应用内副本。",
    guideStyle: "网格样式", guideThirds: "三分法（3×3）", guideGrid: "方格线", guideDiagonal: "对角线",
    bulkPhotoTitle: "将所有照片移到 Apple 照片", bulkPhotoDetail: "视频和恢复文件不受影响。仅在所有照片成功保存后删除应用内副本。",
    bulkPhotoAction: "全部保存并删除应用副本", bulkPhotoEmpty: "没有可移动的已完成照片。",
    bulkPhotoConfirmTitle: "移动并删除所有应用照片？", bulkPhotoConfirm: "将所有已完成照片保存到 Apple 照片，然后永久删除应用内已传输副本。",
    bulkPhotoCancel: "取消", bulkPhotoRunning: "正在保存照片，请保持应用打开…", bulkPhotoComplete: "所有照片已保存，应用内副本已删除。", bulkPhotoFailed: "迁移已停止。若未完成全部导出，应用内副本会保留。",
    lutSelection: "胶片风格 LUT", lutImport: "导入 .cube LUT", lutImportHint: "3D .cube・2–65 网格・最大 4 MiB",
    lutImported: "LUT 已导入", lutImportFailed: "无法导入 LUT", lutBuiltIn: "内置胶片风格原型", lutExternal: "已导入 LUT",
    lutAccuracyNote: "内置 LUT 表现通用胶片与冲印风格，并非对特定胶片型号的测色复刻。",
    focus: "峰值对焦",
    zebra: "斑马纹", guides: "参考线", scopes: "示波器", close: "关闭", adjust: "调整",
    allowCamera: "允许访问相机", requesting: "正在请求相机权限…",
    denied: "相机访问已被拒绝。请在系统设置中启用。", restricted: "此设备限制了相机访问。",
    unavailable: "此平台无法使用相机后端。", noDevice: "未检测到相机设备。",
    detected: "已检测到相机", previewPending: "可以启动原生预览。",
    previewStarting: "正在启动原生预览…", previewFailed: "无法启动原生预览。",
    format: "格式", apply: "应用", formatFailed: "格式更改失败",
    formatPersistenceFailed: "格式已应用，但无法保存设置",
    assetMetadataWarning: "已保存，但存在元数据警告", orientationFailed: "相机方向同步失败",
    mediaTitle: "媒体", mediaSubtitle: "拍摄素材和恢复状态", mediaAll: "全部", mediaReady: "完成",
    mediaIncomplete: "未完成", mediaFailed: "失败", mediaEmpty: "暂无拍摄素材",
    mediaEmptyDetail: "照片和视频会在清单安全保存后显示在这里。", mediaLoading: "正在加载媒体…",
    mediaLoadFailed: "无法读取媒体索引。", mediaBack: "返回相机", mediaRefresh: "刷新媒体",
    mediaView: "媒体视图", mediaThumbnails: "缩略图", mediaDetailed: "详细信息",
    mediaPhoto: "照片", mediaVideo: "视频", mediaDuration: "时长", mediaAwaiting: "等待验证",
    mediaValidationFailed: "验证失败", mediaDetails: "查看详情", mediaPath: "资源路径",
    mediaUpdated: "更新时间", mediaState: "状态", mediaCleanup: "清理可恢复文件",
    mediaCleanupTitle: "删除可恢复媒体？", mediaCleanupPrompt: "这将永久删除未完成或失败的资源及其诊断清单。",
    mediaCleanupConfirm: "删除文件", mediaCleanupCancel: "保留文件", mediaCleanupFailed: "无法删除可恢复媒体。",
    mediaReinspect: "重新检查文件", mediaReinspecting: "正在重新检查媒体…", mediaReinspectFailed: "无法重新检查媒体。",
    mediaRecapture: "重新拍摄", mediaCleanupExpired: "检查过期恢复文件", mediaCleanupExpiredPrompt: "删除所有超过7天的恢复文件吗？已完成媒体会受到保护。"
    , mediaMenu: "媒体操作", mediaDelete: "删除", mediaDeleteTitle: "删除此媒体？",
    mediaDeletePrompt: "这会永久删除此应用中的原始媒体、衍生文件和媒体记录。",
    mediaDeleteFailed: "无法删除媒体。", mediaSavePhotos: "保存到照片",
    mediaSavingPhotos: "正在保存到照片…", mediaSavedPhotos: "已保存到照片。",
    mediaSavePhotosFailed: "无法保存到照片。请在设置中检查照片访问权限。"
    , output: "输出", outputPreset: "输出预设", storageRemaining: "剩余容量",
    estimatedCapacity: "预计可拍摄量", photosRemaining: "张照片", minutesRemaining: "分钟", storageUnavailable: "无法获取存储信息", storageLow: "可用存储空间不足", storageAutoStop: "存储空间不足，已安全停止录制",
    nearby: "附近共享", nearbyTitle: "附近共享", nearbySubtitle: "发现附近的 Universal Film Camera 用户",
    nearbyStart: "开始发现", nearbyStop: "停止发现", nearbyRefresh: "刷新用户", nearbyBack: "返回相机",
    nearbySearching: "已在附近临时可见・正在搜索用户…", nearbyEmpty: "未发现附近用户",
    nearbyEmptyDetail: "请在两台设备上保持此页面打开。返回相机后会停止发现。",
    nearbyLocal: "你的临时 ID", nearbyProtocol: "协议", nearbyAddress: "网络路径",
    nearbyPrivacy: "仅广播临时 ID。传输仍需双方批准并核对相同的确认码。",
    nearbyAsset: "要共享的媒体", nearbySelectAsset: "选择已完成媒体", nearbyPrepare: "准备批准",
    nearbyCodeTitle: "比较确认码", nearbyCodeDetail: "批准前，请确认两台设备上显示完全相同的代码。",
    nearbyApprove: "代码一致・批准", nearbyCancel: "取消", nearbyApproved: "已在此设备批准・等待对方批准",
    nearbyIncoming: "接收共享", nearbySecure: "已建立安全会话",
    nearbyTransferring: "正在加密传输…", nearbyProgress: "已传输",
    nearbyCancelTransfer: "取消传输", nearbyCancelling: "正在安全停止…",
    nearbyRetry: "重新连接并继续", nearbyReconnecting: "正在安全地重新连接…",
    nearbyInterrupted: "连接已中断・已保留验证通过的接收数据",
    nearbyFailureTimeout: "连接超时・已保留验证通过的接收数据",
    nearbyFailureIntegrity: "完整性验证失败・媒体未发布",
    nearbyFailureStorage: "存储空间不足・请释放空间后重试",
    nearbyFailureExpired: "批准已过期・请准备新的确认码",
    nearbyFailureCancelled: "传输已取消・验证通过的数据仍可恢复",
    nearbyFailureProtocol: "安全传输协议失败・请重新批准",
    nearbyFailed: "传输已停止",
    nearbyDiscard: "丢弃接收中的数据", nearbyDiscardTitle: "丢弃接收中的数据？",
    nearbyDiscardPrompt: "这将永久删除本次传输的已验证部分文件和恢复记录。不会删除已完成媒体。",
    nearbyDiscardConfirm: "丢弃部分数据", nearbyKeepPartial: "保留以便恢复",
    nearbyNewApproval: "准备新的批准", nearbyPrepareAgain: "请选择“准备批准”以创建新的确认码。",
    nearbyComplete: "传输完成"
  }
};

function locale(): Locale {
  const value = navigator.language.toLowerCase();
  if (value.startsWith("ja")) return "ja";
  if (value.startsWith("zh")) return "zh-CN";
  return "en";
}

function icon(name: string): string {
  const paths: Record<string, string> = {
    photo: '<path d="M4 8h3l1.5-2h7L17 8h3v10H4z"/><circle cx="12" cy="13" r="3.5"/>',
    video: '<rect x="3" y="6" width="13" height="12" rx="2"/><path d="m16 10 5-3v10l-5-3z"/>',
    focus: '<path d="M8 3H3v5M16 3h5v5M8 21H3v-5M16 21h5v-5"/><circle cx="12" cy="12" r="3"/>',
    zebra: '<path d="m5 19 6-14M10 21l7-16M15 21l4-8"/>',
    guides: '<rect x="3" y="5" width="18" height="14" rx="1"/><path d="M9 5v14M15 5v14M3 10h18M3 15h18"/>',
    scope: '<path d="M3 16h3l2-7 3 9 3-12 3 10h4"/>',
    pipeline: '<circle cx="5" cy="6" r="2"/><circle cx="19" cy="6" r="2"/><circle cx="12" cy="18" r="2"/><path d="M7 6h10M6 8l5 8M18 8l-5 8"/>',
    media: '<rect x="4" y="4" width="16" height="16" rx="2"/><path d="m10 9 6 3-6 3z"/>',
    thumbnails: '<rect x="3" y="3" width="7" height="7" rx="1"/><rect x="14" y="3" width="7" height="7" rx="1"/><rect x="3" y="14" width="7" height="7" rx="1"/><rect x="14" y="14" width="7" height="7" rx="1"/>',
    details: '<rect x="3" y="4" width="6" height="6" rx="1"/><path d="M12 6h9M12 9h7"/><rect x="3" y="14" width="6" height="6" rx="1"/><path d="M12 16h9M12 19h7"/>',
    settings: '<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M19 5l-2 2M7 17l-2 2"/>',
    nearby: '<path d="M5 8.5a10 10 0 0 1 14 0M8 12a6 6 0 0 1 8 0M11 15.5a2 2 0 0 1 2 0"/><circle cx="12" cy="19" r="1"/>',
    display: '<rect x="3" y="4" width="18" height="14" rx="2"/><path d="M8 21h8M12 18v3"/>',
    captureSettings: '<circle cx="12" cy="12" r="7"/><path d="M12 5v14M5 12h14M7 7l10 10M17 7 7 17"/>',
    color: '<circle cx="12" cy="12" r="8"/><path d="M12 4v8l7 4M12 12l-7 4"/>',
    lut: '<path d="M4 18c4 0 4-12 8-12s4 12 8 12"/><path d="M4 21h16"/>',
    refresh: '<path d="M20 6v5h-5M4 18v-5h5"/><path d="M6.1 9a7 7 0 0 1 11.5-2.6L20 11M4 13l2.4 4.6A7 7 0 0 0 18 15"/>',
    output: '<path d="M5 7h14M5 12h14M5 17h14"/><circle cx="8" cy="7" r="1"/><circle cx="16" cy="12" r="1"/><circle cx="10" cy="17" r="1"/>',
    import: '<path d="M12 3v12M7 10l5 5 5-5"/><path d="M4 19h16"/>',
    close: '<path d="m6 6 12 12M18 6 6 18"/>'
  };
  return `<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">${paths[name]}</svg>`;
}

const activeLocale = locale();
const t = copy[activeLocale];
let mode: CameraMode = "still";
let recording = false;
let microphoneAuthorization: CameraAuthorization = "not_determined";
let recordingStartedAt = 0;
let timerId: number | undefined;
let recordingFrameRate = 24;
let storageMonitorId: number | undefined;
let storageCheckPending = false;
let recordingStopPending = false;
let recordingPausedByLifecycle = false;
let currentCapabilities: CameraCapabilities | undefined;
let adjustmentParameter: "lens" | "iris" | "shutter" | "ei" | "wb" | undefined;
let availableDevices: CameraDevice[] = [];
let outputPresets: CaptureOutputPresets | undefined;
let storageStatus: CaptureStorageStatus | undefined;

const parameters: { key: string; label: string; value: string; active?: boolean }[] = [
  { key: "lens", label: "LENS", value: "35mm" },
  { key: "fps", label: "FPS", value: "24" },
  { key: "shutter", label: "SHUTTER", value: "1/48" },
  { key: "iris", label: "IRIS", value: "T2.8" },
  { key: "ei", label: "EI", value: "400" },
  { key: "wb", label: "WB", value: "5600K" }
];

document.querySelector<HTMLDivElement>("#app")!.innerHTML = `
  <main class="camera-shell">
    <section class="monitor" aria-label="${t.nativePreview}">
      <div class="parameter-strip" aria-label="Exposure controls">
        ${parameters.map((parameter) => `
          <button class="parameter${parameter.active ? " is-selected" : ""}" data-parameter="${parameter.key}" aria-pressed="${parameter.active ? "true" : "false"}">
            <span>${parameter.label}</span><strong>${parameter.value}</strong>
          </button>`).join("")}
        <div class="format-lockup"><strong id="active-resolution">—</strong><span id="active-format-detail">AUTO · SDR</span></div>
        <section class="format-panel" id="format-panel" aria-label="${t.format}" hidden>
          <label><span>${t.format}</span><select id="format-resolution"></select></label>
          <label><span>FPS</span><select id="format-fps"></select></label>
          <small id="format-status" role="status"></small>
          <button id="format-apply" type="button">${t.apply}</button>
          <button id="format-close" type="button" aria-label="${t.close}">${icon("close")}</button>
        </section>
        <section class="quick-adjust" id="quick-adjust" hidden aria-live="polite">
          <div><span id="adjust-label">EI</span><strong id="adjust-value">400</strong></div>
          <select id="adjust-select" aria-label="${t.adjust}"></select>
          <button id="adjust-close" aria-label="${t.close}">${icon("close")}</button>
        </section>
      </div>

      <div class="preview-surface">
        <canvas class="processed-preview" id="processed-preview" aria-hidden="true"></canvas>
        <svg class="guide-overlay" viewBox="0 0 100 100" preserveAspectRatio="none" aria-hidden="true">
          <g data-guide-lines="thirds"><path d="M33.333 0V100M66.667 0V100M0 33.333H100M0 66.667H100"/></g>
          <g data-guide-lines="grid"><path d="M20 0V100M40 0V100M60 0V100M80 0V100M0 20H100M0 40H100M0 60H100M0 80H100"/></g>
          <g data-guide-lines="diagonal"><path d="M0 0L100 100M100 0L0 100"/></g>
        </svg>
        <div class="centre-mark" aria-hidden="true"></div>
        <div class="preview-empty">
          <span class="signal-mark"></span>
          <strong id="camera-signal-title">${t.noSignal}</strong>
          <p id="camera-signal-message">${t.backendMessage}</p>
          <button id="request-camera-access" type="button" hidden>${t.allowCamera}</button>
        </div>
        <div class="timecode" aria-live="polite"><span id="record-indicator"></span><strong id="timecode">00:00:00:00</strong></div>
        <div class="monitor-status"><span>REC.709</span><span>180°</span><span>ND —</span></div>

        <section class="histogram" aria-label="RGB histogram">
          <svg viewBox="0 0 132 44" role="img" aria-label="RGB histogram unavailable">
            <path id="hist-red" class="hist-red" d="M2 41H130V41Z"/>
            <path id="hist-green" class="hist-green" d="M2 41H130V41Z"/>
            <path id="hist-blue" class="hist-blue" d="M2 41H130V41Z"/>
          </svg>
        </section>

        <section class="audio-meter" aria-label="Audio meters">
          <div><span>1</span><i id="audio-level-1" style="--level: 0%"></i></div>
          <div><span>2</span><i id="audio-level-2" style="--level: 0%"></i></div>
          <small>−48&nbsp;&nbsp;−24&nbsp;&nbsp;−12&nbsp;&nbsp;0</small>
        </section>

        <div class="capture-feedback" id="capture-feedback" role="status">${t.captured}</div>
      </div>
    </section>

    <section class="media-library" id="media-library" aria-labelledby="media-title" hidden>
      <header class="media-header">
        <div>
          <h1 id="media-title">${t.mediaTitle}</h1>
          <p>${t.mediaSubtitle}</p>
        </div>
        <div class="media-header-actions">
          <button id="media-cleanup-expired" type="button" hidden>${icon("close")}<span>${t.mediaCleanupExpired}</span></button>
          <button id="media-refresh" type="button" aria-label="${t.mediaRefresh}" title="${t.mediaRefresh}">${icon("refresh")}<span>${t.mediaRefresh}</span></button>
          <button id="media-back" type="button" aria-label="${t.mediaBack}">${icon("close")}<span>${t.mediaBack}</span></button>
        </div>
      </header>

      <nav class="media-filters" aria-label="${t.mediaTitle}">
        <button class="is-active" data-media-filter="all" aria-pressed="true"><span>${t.mediaAll}</span><strong data-media-count="all">0</strong></button>
        <button data-media-filter="finalized" aria-pressed="false"><span>${t.mediaReady}</span><strong data-media-count="finalized">0</strong></button>
        <button data-media-filter="incomplete" aria-pressed="false"><span>${t.mediaIncomplete}</span><strong data-media-count="incomplete">0</strong></button>
        <button data-media-filter="failed" aria-pressed="false"><span>${t.mediaFailed}</span><strong data-media-count="failed">0</strong></button>
      </nav>

      <div class="media-view-switch" role="group" aria-label="${t.mediaView}">
        <button class="is-active" data-media-view="thumbnails" aria-pressed="true">${icon("thumbnails")}<span>${t.mediaThumbnails}</span></button>
        <button data-media-view="details" aria-pressed="false">${icon("details")}<span>${t.mediaDetailed}</span></button>
      </div>

      <div class="media-status" id="media-status" role="status" aria-live="polite"></div>
      <div class="media-grid" id="media-grid"></div>
      <div class="media-empty" id="media-empty" hidden>
        ${icon("media")}
        <strong>${t.mediaEmpty}</strong>
        <p>${t.mediaEmptyDetail}</p>
      </div>
      <div class="media-context-menu" id="media-context-menu" role="menu" aria-label="${t.mediaMenu}" hidden>
        <button id="media-context-save" type="button" role="menuitem">${t.mediaSavePhotos}</button>
        <button class="is-destructive" id="media-context-delete" type="button" role="menuitem">${t.mediaDelete}</button>
      </div>
    </section>

    <section class="nearby-library" id="nearby-library" aria-labelledby="nearby-title" hidden>
      <header class="media-header">
        <div>
          <h1 id="nearby-title">${t.nearbyTitle}</h1>
          <p>${t.nearbySubtitle}</p>
        </div>
        <div class="media-header-actions">
          <button id="nearby-refresh" type="button" aria-label="${t.nearbyRefresh}" title="${t.nearbyRefresh}">${icon("refresh")}<span>${t.nearbyRefresh}</span></button>
          <button id="nearby-back" type="button" aria-label="${t.nearbyBack}">${icon("close")}<span>${t.nearbyBack}</span></button>
        </div>
      </header>
      <div class="nearby-controls">
        <button id="nearby-toggle" type="button" data-state="default">${t.nearbyStart}</button>
        <div class="nearby-identity"><span>${t.nearbyLocal}</span><strong id="nearby-local-id">—</strong></div>
      </div>
      <p class="nearby-privacy">${t.nearbyPrivacy}</p>
      <div class="nearby-share-controls">
        <label><span>${t.nearbyAsset}</span><select id="nearby-asset"><option value="">${t.nearbySelectAsset}</option></select></label>
        <button id="nearby-prepare" type="button" disabled>${t.nearbyPrepare}</button>
      </div>
      <div class="media-status" id="nearby-status" role="status" aria-live="polite"></div>
      <div class="nearby-grid" id="nearby-grid"></div>
      <div class="media-empty" id="nearby-empty">
        ${icon("nearby")}
        <strong>${t.nearbyEmpty}</strong>
        <p>${t.nearbyEmptyDetail}</p>
      </div>
    </section>

    <section class="settings-page" id="settings-page" aria-labelledby="settings-title" hidden>
      <header class="media-header">
        <div><h1 id="settings-title">${t.settingsTitle}</h1><p>${t.settingsSubtitle}</p></div>
        <div class="media-header-actions">
          <button id="settings-back" type="button" aria-label="${t.settingsBack}">${icon("close")}<span>${t.settingsBack}</span></button>
        </div>
      </header>
      <nav class="settings-tabs" role="tablist" aria-label="${t.settingsTitle}">
        <button id="settings-tab-display" role="tab" aria-label="${t.settingsDisplay}" title="${t.settingsDisplay}" aria-selected="false" aria-controls="settings-panel-display" tabindex="-1" data-settings-tab="display">${icon("display")}<span>${t.settingsDisplay}</span></button>
        <button id="settings-tab-capture" role="tab" aria-label="${t.settingsCapture}" title="${t.settingsCapture}" aria-selected="false" aria-controls="settings-panel-capture" tabindex="-1" data-settings-tab="capture">${icon("captureSettings")}<span>${t.settingsCapture}</span></button>
        <button id="settings-tab-color" role="tab" aria-label="${t.settingsColor}" title="${t.settingsColor}" aria-selected="true" aria-controls="settings-panel-color" data-settings-tab="color">${icon("color")}<span>${t.settingsColor}</span></button>
        <button id="settings-tab-lut" role="tab" aria-label="${t.settingsLut}" title="${t.settingsLut}" aria-selected="false" aria-controls="settings-panel-lut" tabindex="-1" data-settings-tab="lut">${icon("lut")}<span>${t.settingsLut}</span></button>
        <button id="settings-tab-media" role="tab" aria-label="${t.settingsMedia}" title="${t.settingsMedia}" aria-selected="false" aria-controls="settings-panel-media" tabindex="-1" data-settings-tab="media">${icon("media")}<span>${t.settingsMedia}</span></button>
      </nav>
      <div class="settings-panel" id="settings-panel-display" role="tabpanel" aria-labelledby="settings-tab-display" hidden>
        <h2 class="settings-panel-title">${t.settingsDisplay}</h2>
        <div class="settings-readonly"><span>${t.settingsDisplay}</span><strong>UI · DARK</strong><small>${t.settingsDisplayDetail}</small></div>
        <label class="settings-field peaking-color-field" for="peaking-color"><span>${t.peakingColor}</span><span class="peaking-color-control"><i id="peaking-color-swatch" aria-hidden="true"></i><select id="peaking-color"><option value="cyan">${t.colorCyan}</option><option value="red">${t.colorRed}</option><option value="green">${t.colorGreen}</option><option value="yellow">${t.colorYellow}</option><option value="magenta">${t.colorMagenta}</option><option value="white">${t.colorWhite}</option></select></span></label>
        <p class="settings-status" id="display-settings-status" role="status" aria-live="polite"></p>
      </div>
      <div class="settings-panel" id="settings-panel-capture" role="tabpanel" aria-labelledby="settings-tab-capture" hidden>
        <h2 class="settings-panel-title">${t.settingsCapture}</h2>
        <div class="settings-readonly"><span>${t.settingsCapture}</span><strong>PHOTO + VIDEO</strong><small>${t.settingsCaptureDetail}</small></div>
        <label class="settings-field" for="shutter-sound"><span>${t.shutterSound}</span>
          <select id="shutter-sound"><option value="standard">${t.shutterSoundStandard}</option><option value="fresh">${t.shutterSoundFresh}</option><option value="dslr">${t.shutterSoundDslr}</option><option value="silent">${t.shutterSoundSilent}</option></select>
        </label>
        <label class="lut-import" for="shutter-sound-file" aria-label="${t.shutterSoundImport}" title="${t.shutterSoundImport} · ${t.shutterSoundImportHint}">${icon("import")}<span>${t.shutterSoundImport}</span><small>${t.shutterSoundImportHint}</small></label>
        <input id="shutter-sound-file" type="file" accept="audio/*,.mp3,.m4a,.wav,.caf" hidden>
        <p class="settings-status" id="shutter-sound-status" role="status" aria-live="polite"></p>
        <p class="settings-note">${t.shutterSoundDetail}</p>
        <label class="settings-field" for="guide-style"><span>${t.guideStyle}</span>
          <select id="guide-style"><option value="thirds">${t.guideThirds}</option><option value="grid">${t.guideGrid}</option><option value="diagonal">${t.guideDiagonal}</option></select>
        </label>
      </div>
      <div class="settings-panel" id="settings-panel-color" role="tabpanel" aria-labelledby="settings-tab-color">
        <h2 class="settings-panel-title">${t.settingsColor}</h2>
        <label class="settings-field" for="monitoring-color-space"><span>${t.monitoringColorSpace}</span>
          <select id="monitoring-color-space"><option value="rec709">Rec.709</option><option value="display-p3">Display P3</option><option value="srgb">sRGB</option></select>
        </label>
        <label class="settings-toggle" for="monitor-look-enabled"><input id="monitor-look-enabled" type="checkbox"><span>${t.monitorLook}<small>${t.monitorLookDetail}</small></span></label>
        <div class="settings-readonly"><span>${t.workingColorSpace}</span><strong>ACEScg</strong><small>${t.workingColorSpaceDetail}</small></div>
        <p class="settings-status" id="settings-status" role="status" aria-live="polite"></p>
      </div>
      <div class="settings-panel" id="settings-panel-lut" role="tabpanel" aria-labelledby="settings-tab-lut" hidden>
        <h2 class="settings-panel-title">${t.settingsLut}</h2>
        <label class="settings-field" for="lut-selection"><span>${t.lutSelection}</span><select id="lut-selection"></select></label>
        <p class="settings-note">${t.lutAccuracyNote}</p>
        <label class="lut-import" for="lut-import-file" aria-label="${t.lutImport}" title="${t.lutImport} · ${t.lutImportHint}">${icon("import")}<span>${t.lutImport}</span><small>${t.lutImportHint}</small></label>
        <input id="lut-import-file" type="file" accept=".cube,text/plain" hidden>
        <p class="settings-status" id="lut-status" role="status" aria-live="polite"></p>
      </div>
      <div class="settings-panel settings-media-panel" id="settings-panel-media" role="tabpanel" aria-labelledby="settings-tab-media" hidden>
        <h2 class="settings-panel-title">${t.settingsMedia}</h2>
        <div class="settings-readonly"><span>${t.bulkPhotoTitle}</span><strong id="bulk-photo-count">—</strong><small>${t.settingsMediaDetail}</small></div>
        <p class="settings-note">${t.bulkPhotoDetail}</p>
        <button id="bulk-photo-start" type="button">${t.bulkPhotoAction}</button>
        <p class="settings-status" id="bulk-photo-status" role="status" aria-live="polite"></p>
      </div>
    </section>

    <dialog class="media-dialog nearby-approval-dialog" id="nearby-approval-dialog" aria-labelledby="nearby-code-title">
      <header>
        <h2 id="nearby-code-title">${t.nearbyCodeTitle}</h2>
        <button id="nearby-approval-close" type="button" aria-label="${t.close}">${icon("close")}</button>
      </header>
      <p>${t.nearbyCodeDetail}</p>
      <strong id="nearby-confirmation-code">—</strong>
      <dl id="nearby-approval-detail"></dl>
      <div class="nearby-transfer-progress" id="nearby-transfer-progress" hidden>
        <div><span id="nearby-progress-label">${t.nearbyProgress}</span><strong id="nearby-progress-value">0%</strong></div>
        <progress id="nearby-progress-bar" max="100" value="0"></progress>
      </div>
      <footer>
        <button id="nearby-approval-cancel" type="button">${t.nearbyCancel}</button>
        <button id="nearby-approval-confirm" type="button">${t.nearbyApprove}</button>
      </footer>
    </dialog>

    <dialog class="media-dialog media-confirm-dialog" id="nearby-discard-dialog" aria-labelledby="nearby-discard-title">
      <h2 id="nearby-discard-title">${t.nearbyDiscardTitle}</h2>
      <p>${t.nearbyDiscardPrompt}</p>
      <footer>
        <button id="nearby-discard-cancel" type="button">${t.nearbyKeepPartial}</button>
        <button class="media-cleanup" id="nearby-discard-confirm" type="button">${t.nearbyDiscardConfirm}</button>
      </footer>
    </dialog>

    <dialog class="media-dialog" id="media-detail-dialog" aria-labelledby="media-detail-title">
      <header>
        <h2 id="media-detail-title">${t.mediaDetails}</h2>
        <button id="media-detail-close" type="button" aria-label="${t.close}">${icon("close")}</button>
      </header>
      <dl id="media-detail-content"></dl>
      <p class="media-detail-diagnostic" id="media-detail-diagnostic" hidden></p>
      <footer>
        <button class="media-reinspect" id="media-reinspect" type="button" data-state="default" hidden>${t.mediaReinspect}</button>
        <button class="media-recapture" id="media-recapture" type="button" data-state="default" hidden>${t.mediaRecapture}</button>
        <button class="media-cleanup" id="media-cleanup" type="button" data-state="default" hidden>${t.mediaCleanup}</button>
      </footer>
    </dialog>

    <dialog class="media-dialog media-confirm-dialog" id="media-cleanup-dialog" aria-labelledby="media-cleanup-title">
      <h2 id="media-cleanup-title">${t.mediaCleanupTitle}</h2>
      <p>${t.mediaCleanupPrompt}</p>
      <strong id="media-cleanup-name"></strong>
      <div>
        <button id="media-cleanup-cancel" type="button">${t.mediaCleanupCancel}</button>
        <button class="media-cleanup-confirm" id="media-cleanup-confirm" type="button" data-state="default">${t.mediaCleanupConfirm}</button>
      </div>
    </dialog>

    <dialog class="media-dialog media-confirm-dialog" id="media-delete-dialog" aria-labelledby="media-delete-title">
      <h2 id="media-delete-title">${t.mediaDeleteTitle}</h2>
      <p>${t.mediaDeletePrompt}</p>
      <strong id="media-delete-name"></strong>
      <div>
        <button id="media-delete-cancel" type="button">${t.mediaCleanupCancel}</button>
        <button class="media-cleanup-confirm" id="media-delete-confirm" type="button" data-state="default">${t.mediaDelete}</button>
      </div>
    </dialog>

    <dialog class="media-dialog media-confirm-dialog" id="bulk-photo-dialog" aria-labelledby="bulk-photo-dialog-title">
      <h2 id="bulk-photo-dialog-title">${t.bulkPhotoConfirmTitle}</h2>
      <p>${t.bulkPhotoConfirm}</p>
      <strong id="bulk-photo-dialog-count">—</strong>
      <div>
        <button id="bulk-photo-cancel" type="button">${t.bulkPhotoCancel}</button>
        <button class="media-cleanup-confirm" id="bulk-photo-confirm" type="button" data-state="default">${t.bulkPhotoAction}</button>
      </div>
    </dialog>

    <dialog class="media-dialog output-dialog" id="output-dialog" aria-labelledby="output-title">
      <header>
        <h2 id="output-title">${t.output}</h2>
        <button id="output-close" type="button" aria-label="${t.close}">${icon("close")}</button>
      </header>
      <dl>
        <div><dt>${t.outputPreset}</dt><dd id="output-preset">—</dd></div>
        <div><dt>${t.storageRemaining}</dt><dd id="storage-remaining">—</dd></div>
        <div><dt>${t.estimatedCapacity}</dt><dd id="storage-estimate">—</dd></div>
      </dl>
    </dialog>

    <aside class="tool-rail" aria-label="Camera tools">
      <div class="rail-leading">
        <div class="mode-switch" role="group" aria-label="Camera mode">
          <button class="is-active" data-mode="still" aria-pressed="true">${icon("photo")}<span>${t.photo}</span></button>
          <button data-mode="video" aria-pressed="false">${icon("video")}<span>${t.video}</span></button>
        </div>
      </div>

      <div class="monitor-tools" id="monitor-tools-panel">
        <button data-tool="focus" aria-pressed="false" aria-label="${t.focus}">${icon("focus")}<span>${t.focus}</span></button>
        <button data-tool="zebra" aria-pressed="false" aria-label="${t.zebra}">${icon("zebra")}<span>${t.zebra}</span></button>
        <button data-tool="guides" aria-pressed="true" class="is-active" aria-label="${t.guides}">${icon("guides")}<span>${t.guides}</span></button>
        <button data-tool="scope" aria-pressed="true" class="is-active" aria-label="${t.scopes}">${icon("scope")}<span>${t.scopes}</span></button>
      </div>

      <button id="capture" class="shutter" aria-label="${t.capture}" data-state="default"><span></span></button>

      <div class="rail-trailing">
        <button class="output-status" id="output-status" type="button" aria-label="${t.output}" title="${t.output}">${icon("output")}<span>${t.output} · —</span></button>
        <button class="monitor-tools-toggle" id="monitor-tools-toggle" aria-expanded="false" aria-controls="monitor-tools-panel" aria-label="${t.scopes}">${icon("scope")}<span>${t.scopes}</span></button>

        <button class="destination-tools-toggle" id="destination-tools-toggle" aria-expanded="false" aria-controls="destination-tools" aria-label="${t.camera}" title="${t.camera}">${icon("pipeline")}<span>${t.camera}</span></button>

        <nav class="destination-tools" id="destination-tools" aria-label="Application sections">
          <button class="is-active" aria-label="${t.pipeline}">${icon("pipeline")}<span>${t.pipeline}</span></button>
          <button id="open-media" aria-label="${t.media}" aria-controls="media-library" aria-pressed="false">${icon("media")}<span>${t.media}</span></button>
          <button id="open-nearby" aria-label="${t.nearby}" aria-controls="nearby-library" aria-pressed="false">${icon("nearby")}<span>${t.nearby}</span></button>
          <button id="open-settings" aria-label="${t.settings}" aria-controls="settings-page" aria-pressed="false">${icon("settings")}<span>${t.settings}</span></button>
        </nav>
      </div>
    </aside>
  </main>`;

document.body.classList.add("tool-guides", "tool-scope");

const timecode = document.querySelector<HTMLElement>("#timecode")!;
const indicator = document.querySelector<HTMLElement>("#record-indicator")!;
const captureButton = document.querySelector<HTMLButtonElement>("#capture")!;
const feedback = document.querySelector<HTMLElement>("#capture-feedback")!;
const signalTitle = document.querySelector<HTMLElement>("#camera-signal-title")!;
const signalMessage = document.querySelector<HTMLElement>("#camera-signal-message")!;
const accessButton = document.querySelector<HTMLButtonElement>("#request-camera-access")!;
const previewSurface = document.querySelector<HTMLElement>(".preview-surface")!;
const activeResolution = document.querySelector<HTMLElement>("#active-resolution")!;
const activeFormatDetail = document.querySelector<HTMLElement>("#active-format-detail")!;
const formatPanel = document.querySelector<HTMLElement>("#format-panel")!;
const formatResolution = document.querySelector<HTMLSelectElement>("#format-resolution")!;
const formatFps = document.querySelector<HTMLSelectElement>("#format-fps")!;
const formatStatus = document.querySelector<HTMLElement>("#format-status")!;
const formatApply = document.querySelector<HTMLButtonElement>("#format-apply")!;
const monitor = document.querySelector<HTMLElement>(".monitor")!;
const mediaLibrary = document.querySelector<HTMLElement>("#media-library")!;
const mediaGrid = document.querySelector<HTMLElement>("#media-grid")!;
const mediaEmpty = document.querySelector<HTMLElement>("#media-empty")!;
const mediaStatus = document.querySelector<HTMLElement>("#media-status")!;
const mediaButton = document.querySelector<HTMLButtonElement>("#open-media")!;
const mediaRefresh = document.querySelector<HTMLButtonElement>("#media-refresh")!;
const mediaCleanupExpired = document.querySelector<HTMLButtonElement>("#media-cleanup-expired")!;
const nearbyLibrary = document.querySelector<HTMLElement>("#nearby-library")!;
const settingsPage = document.querySelector<HTMLElement>("#settings-page")!;
const settingsButton = document.querySelector<HTMLButtonElement>("#open-settings")!;
const monitoringColorSpace = document.querySelector<HTMLSelectElement>("#monitoring-color-space")!;
const monitorLookEnabled = document.querySelector<HTMLInputElement>("#monitor-look-enabled")!;
const settingsStatus = document.querySelector<HTMLElement>("#settings-status")!;
const peakingColor = document.querySelector<HTMLSelectElement>("#peaking-color")!;
const peakingColorSwatch = document.querySelector<HTMLElement>("#peaking-color-swatch")!;
const displaySettingsStatus = document.querySelector<HTMLElement>("#display-settings-status")!;
const lutSelection = document.querySelector<HTMLSelectElement>("#lut-selection")!;
const lutImportFile = document.querySelector<HTMLInputElement>("#lut-import-file")!;
const lutStatus = document.querySelector<HTMLElement>("#lut-status")!;
const guideStyle = document.querySelector<HTMLSelectElement>("#guide-style")!;
const shutterSound = document.querySelector<HTMLSelectElement>("#shutter-sound")!;
const shutterSoundFile = document.querySelector<HTMLInputElement>("#shutter-sound-file")!;
const shutterSoundStatus = document.querySelector<HTMLElement>("#shutter-sound-status")!;
const bulkPhotoCount = document.querySelector<HTMLElement>("#bulk-photo-count")!;
const bulkPhotoStart = document.querySelector<HTMLButtonElement>("#bulk-photo-start")!;
const bulkPhotoStatus = document.querySelector<HTMLElement>("#bulk-photo-status")!;
const bulkPhotoDialog = document.querySelector<HTMLDialogElement>("#bulk-photo-dialog")!;
const bulkPhotoConfirm = document.querySelector<HTMLButtonElement>("#bulk-photo-confirm")!;
const nearbyButton = document.querySelector<HTMLButtonElement>("#open-nearby")!;
const nearbyToggle = document.querySelector<HTMLButtonElement>("#nearby-toggle")!;
const nearbyRefresh = document.querySelector<HTMLButtonElement>("#nearby-refresh")!;
const nearbyStatus = document.querySelector<HTMLElement>("#nearby-status")!;
const nearbyGrid = document.querySelector<HTMLElement>("#nearby-grid")!;
const nearbyEmpty = document.querySelector<HTMLElement>("#nearby-empty")!;
const nearbyAsset = document.querySelector<HTMLSelectElement>("#nearby-asset")!;
const nearbyPrepare = document.querySelector<HTMLButtonElement>("#nearby-prepare")!;
const nearbyApprovalDialog = document.querySelector<HTMLDialogElement>("#nearby-approval-dialog")!;
const nearbyDiscardDialog = document.querySelector<HTMLDialogElement>("#nearby-discard-dialog")!;
const mediaDetailDialog = document.querySelector<HTMLDialogElement>("#media-detail-dialog")!;
const mediaDetailContent = document.querySelector<HTMLDListElement>("#media-detail-content")!;
const mediaDetailDiagnostic = document.querySelector<HTMLParagraphElement>("#media-detail-diagnostic")!;
const mediaCleanup = document.querySelector<HTMLButtonElement>("#media-cleanup")!;
const mediaReinspect = document.querySelector<HTMLButtonElement>("#media-reinspect")!;
const mediaRecapture = document.querySelector<HTMLButtonElement>("#media-recapture")!;
const mediaCleanupDialog = document.querySelector<HTMLDialogElement>("#media-cleanup-dialog")!;
const mediaCleanupConfirm = document.querySelector<HTMLButtonElement>("#media-cleanup-confirm")!;
const mediaContextMenu = document.querySelector<HTMLElement>("#media-context-menu")!;
const mediaContextSave = document.querySelector<HTMLButtonElement>("#media-context-save")!;
const mediaContextDelete = document.querySelector<HTMLButtonElement>("#media-context-delete")!;
const mediaDeleteDialog = document.querySelector<HTMLDialogElement>("#media-delete-dialog")!;
const mediaDeleteConfirm = document.querySelector<HTMLButtonElement>("#media-delete-confirm")!;
const outputStatus = document.querySelector<HTMLButtonElement>("#output-status")!;
const outputDialog = document.querySelector<HTMLDialogElement>("#output-dialog")!;
let nativePreviewRunning = false;
let nativePreviewStarting = false;
let monitorTelemetryPending = false;
let activeDeviceId: string | undefined;
let activeDevicePosition: CameraDevice["position"] | undefined;
let lastOrientationKey: string | undefined;
let mediaEntries: MediaIndexEntry[] = [];
let mediaFilter: MediaFilter = "all";
let mediaView: MediaView = "thumbnails";
let selectedMediaEntry: MediaIndexEntry | undefined;
let contextMediaEntry: MediaIndexEntry | undefined;
let mediaLongPressTimer: number | undefined;
let pendingBulkCleanupIds: string[] = [];
let bulkCleanupRequested = false;
let nearbySnapshot: NearbyDiscoverySnapshot = { active: false, local_peer: null, peers: [], last_error: null };
let nearbyPollId: number | undefined;
let selectedNearbyPeerId: string | undefined;
const monitoringColorSpaceKey = "ufc.monitoring-color-space.v1";
const monitorLookEnabledKey = "ufc.monitor-look-enabled.v1";
const selectedLutKey = "ufc.selected-lut.v1";
const guideStyleKey = "ufc.guide-style.v1";
const peakingColorKey = "ufc.peaking-color.v1";
const shutterSoundKey = "ufc.shutter-sound.v1";
type ShutterSound = "standard" | "fresh" | "dslr" | "silent" | "custom";
const shutterSoundDatabase = "ufc-shutter-sounds-v1";
let customShutterSoundUrl: string | undefined;
let customShutterSoundName: string | undefined;
const peakingColors: Record<string, [number, number, number]> = {
  cyan: [0, 225, 255], red: [255, 48, 48], green: [70, 255, 105],
  yellow: [255, 224, 32], magenta: [255, 64, 224], white: [255, 255, 255]
};
const builtInLuts: LutEntry[] = [
  ["none", "Clean / No LUT", "neutral"], ["negative-daylight-soft", "Daylight Negative · Soft", "negative"],
  ["negative-daylight-rich", "Daylight Negative · Rich", "negative"], ["negative-tungsten", "Tungsten Negative", "negative"],
  ["negative-pastel", "Pastel Negative", "negative"], ["negative-warm-consumer", "Warm Consumer Negative", "negative"],
  ["negative-cool-consumer", "Cool Consumer Negative", "negative"], ["reversal-neutral", "Daylight Reversal · Neutral", "reversal"],
  ["reversal-vivid", "Daylight Reversal · Vivid", "reversal"], ["reversal-warm", "Warm Reversal", "reversal"],
  ["print-warm", "Warm Release Print", "print"], ["print-cool", "Cool Release Print", "print"],
  ["bleach-bypass", "Bleach Bypass", "process"], ["archive-faded", "Faded Archive", "process"],
  ["bw-panchromatic-soft", "B&W Panchromatic · Soft", "monochrome"], ["bw-panchromatic-hard", "B&W Panchromatic · Hard", "monochrome"],
  ["bw-orthochromatic", "B&W Orthochromatic", "monochrome"]
].map(([id, name, category]) => ({ id, name, category, source: "built_in", size: 33 }));

function storedPreference(key: string, fallback: string): string {
  try { return window.localStorage.getItem(key) ?? fallback; } catch { return fallback; }
}

function applyGuideStyle(value: string): void {
  const selected = ["thirds", "grid", "diagonal"].includes(value) ? value : "thirds";
  guideStyle.value = selected;
  document.body.dataset.guideStyle = selected;
}

function applyPeakingColor(value: string): void {
  const selected = value in peakingColors ? value : "cyan";
  peakingColor.value = selected;
  const [red, green, blue] = peakingColors[selected];
  peakingColorSwatch.style.backgroundColor = `rgb(${red} ${green} ${blue})`;
}

function applyShutterSound(value: string): void {
  const supported = ["standard", "fresh", "dslr", "silent", ...(customShutterSoundUrl ? ["custom"] : [])];
  shutterSound.value = supported.includes(value) ? value : "standard";
}

function openShutterSoundDatabase(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(shutterSoundDatabase, 1);
    request.onupgradeneeded = () => request.result.createObjectStore("sounds");
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

async function storeCustomShutterSound(file: File): Promise<void> {
  const database = await openShutterSoundDatabase();
  await new Promise<void>((resolve, reject) => {
    const transaction = database.transaction("sounds", "readwrite");
    transaction.objectStore("sounds").put({ blob: file, name: file.name }, "custom");
    transaction.oncomplete = () => resolve(); transaction.onerror = () => reject(transaction.error);
  });
  database.close();
}

async function loadCustomShutterSound(): Promise<void> {
  const database = await openShutterSoundDatabase();
  const stored = await new Promise<{ blob: Blob; name: string } | undefined>((resolve, reject) => {
    const request = database.transaction("sounds").objectStore("sounds").get("custom");
    request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
  });
  database.close();
  if (!stored?.blob) return;
  if (customShutterSoundUrl) URL.revokeObjectURL(customShutterSoundUrl);
  customShutterSoundUrl = URL.createObjectURL(stored.blob);
  customShutterSoundName = stored.name;
  let option = shutterSound.querySelector<HTMLOptionElement>('option[value="custom"]');
  if (!option) { option = document.createElement("option"); option.value = "custom"; shutterSound.append(option); }
  option.textContent = `${t.shutterSoundCustom} · ${stored.name}`;
}

function playShutterSound(kind = shutterSound.value as ShutterSound): void {
  if (kind === "silent") return;
  if (kind === "custom" && customShutterSoundUrl) {
    const audio = new Audio(customShutterSoundUrl); audio.volume = 1;
    void audio.play().catch(() => { shutterSoundStatus.textContent = t.shutterSoundImportFailed; shutterSoundStatus.dataset.state = "error"; });
    return;
  }
  const AudioContextClass = window.AudioContext ?? (window as typeof window & { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
  if (!AudioContextClass) return;
  const context = new AudioContextClass();
  const start = context.currentTime;
  const noise = (at: number, duration: number, gainValue: number, cutoff: number) => {
    const buffer = context.createBuffer(1, Math.ceil(context.sampleRate * duration), context.sampleRate);
    const samples = buffer.getChannelData(0);
    for (let index = 0; index < samples.length; index++) samples[index] = Math.random() * 2 - 1;
    const source = context.createBufferSource(); source.buffer = buffer;
    const filter = context.createBiquadFilter(); filter.type = "lowpass"; filter.frequency.value = cutoff;
    const gain = context.createGain(); gain.gain.setValueAtTime(gainValue, start + at); gain.gain.exponentialRampToValueAtTime(0.001, start + at + duration);
    source.connect(filter).connect(gain).connect(context.destination); source.start(start + at); source.stop(start + at + duration);
  };
  const tone = (at: number, frequency: number, duration: number, gainValue: number) => {
    const oscillator = context.createOscillator(); const gain = context.createGain();
    oscillator.type = "sine"; oscillator.frequency.setValueAtTime(frequency, start + at);
    gain.gain.setValueAtTime(gainValue, start + at); gain.gain.exponentialRampToValueAtTime(0.001, start + at + duration);
    oscillator.connect(gain).connect(context.destination); oscillator.start(start + at); oscillator.stop(start + at + duration);
  };
  if (kind === "fresh") { tone(0, 1320, .09, .16); tone(.055, 1760, .16, .11); }
  else if (kind === "dslr") { noise(0, .055, .28, 1800); noise(.075, .075, .32, 1200); noise(.18, .055, .2, 1600); }
  else { noise(0, .045, .22, 2400); noise(.065, .055, .18, 1800); }
  window.setTimeout(() => void context.close(), 500);
}

async function refreshBulkPhotoCount(): Promise<number> {
  try {
    const entries = recoveryFixtureEnabled
      ? recoveryFixtureEntries()
      : await invoke<MediaIndexEntry[]>("get_media_index");
    const count = entries.filter((entry) => entry.state === "finalized" && entry.media_type === "photo").length;
    bulkPhotoCount.textContent = `${count} ${t.mediaPhoto}`;
    bulkPhotoStart.disabled = count === 0;
    bulkPhotoStatus.textContent = count === 0 ? t.bulkPhotoEmpty : "";
    return count;
  } catch (error) {
    bulkPhotoCount.textContent = "—";
    bulkPhotoStart.disabled = true;
    bulkPhotoStatus.textContent = String(error);
    bulkPhotoStatus.dataset.state = "error";
    return 0;
  }
}

function populateLutSelection(catalog: LutCatalog): void {
  const selected = storedPreference(selectedLutKey, "none");
  const builtIn = document.createElement("optgroup");
  builtIn.label = t.lutBuiltIn;
  builtIn.append(...catalog.built_in.map((lut) => new Option(lut.name, lut.id)));
  const groups: HTMLOptGroupElement[] = [builtIn];
  if (catalog.imported.length) {
    const imported = document.createElement("optgroup");
    imported.label = t.lutExternal;
    imported.append(...catalog.imported.map((lut) => new Option(`${lut.name} · ${lut.size}³`, lut.id)));
    groups.push(imported);
  }
  lutSelection.replaceChildren(...groups);
  lutSelection.value = [...lutSelection.options].some((option) => option.value === selected) ? selected : "none";
}

async function loadLutCatalog(): Promise<void> {
  try { populateLutSelection(await invoke<LutCatalog>("get_lut_catalog")); }
  catch { populateLutSelection({ built_in: builtInLuts, imported: [] }); }
  await loadSelectedLutPayload();
}

function loadMonitoringColorSpace(): string {
  try {
    const stored = window.localStorage.getItem(monitoringColorSpaceKey);
    return ["rec709", "display-p3", "srgb"].includes(stored ?? "") ? stored! : "rec709";
  } catch { return "rec709"; }
}

function colorSpaceLabel(value: string): string {
  if (value === "display-p3") return "Display P3";
  if (value === "srgb") return "sRGB";
  return "Rec.709";
}

function applyMonitoringColorSpace(value: string): void {
  monitoringColorSpace.value = value;
  document.querySelector<HTMLElement>(".monitor-status span:first-child")!.textContent = colorSpaceLabel(value);
}

let activeLutPayload: LutPayload | undefined;
let processedPreview = document.querySelector<HTMLCanvasElement>("#processed-preview")!;
let processedContext: CanvasRenderingContext2D | undefined;
const sampleCanvas = document.createElement("canvas");
const sampleContext = sampleCanvas.getContext("2d", { alpha: true })!;

function syncMonitorProcessing(): void {
  document.body.classList.toggle("monitor-processing", monitorLookEnabled.checked || document.body.classList.contains("tool-focus") || document.body.classList.contains("tool-zebra"));
}

async function loadSelectedLutPayload(): Promise<void> {
  activeLutPayload = undefined;
  if (!lutSelection.value.startsWith("imported:")) return;
  try { activeLutPayload = await invoke<LutPayload>("get_lut_payload", { id: lutSelection.value }); }
  catch (error) { lutStatus.textContent = `${t.lutImportFailed}: ${String(error)}`; lutStatus.dataset.state = "error"; }
}

function builtInLook(id: string, r: number, g: number, b: number): [number, number, number] {
  if (id === "none") return [r, g, b];
  const monochrome = id.startsWith("bw-");
  const vivid = id.includes("vivid") || id.includes("hard") || id === "bleach-bypass";
  const soft = id.includes("soft") || id.includes("pastel") || id === "archive-faded";
  const warm = id.includes("warm") || id === "negative-tungsten" || id === "print-warm";
  const cool = id.includes("cool") || id === "print-cool";
  const contrast = vivid ? 1.22 : soft ? 0.84 : 1.05;
  const saturation = monochrome ? 0 : id === "bleach-bypass" ? 0.48 : soft ? 0.82 : vivid ? 1.18 : 1.02;
  const lift = soft || id === "archive-faded" ? 0.035 : 0;
  const luma = 0.2126 * r + 0.7152 * g + 0.0722 * b;
  const grade = (value: number, gain: number) => Math.max(0, Math.min(1, ((luma + (value - luma) * saturation - 0.5) * contrast + 0.5 + lift) * gain));
  let output: [number, number, number] = [grade(r, warm ? 1.055 : cool ? 0.955 : 1), grade(g, 1), grade(b, warm ? 0.925 : cool ? 1.06 : 1)];
  if (id === "bw-orthochromatic") { const gray = Math.max(0, Math.min(1, r * 0.15 + g * 0.85)); output = [gray, gray, gray]; }
  return output;
}

function applyMonitoringProfile(color: [number, number, number]): [number, number, number] {
  const decodeSrgb = (value: number) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  const encodeSrgb = (value: number) => value <= 0.0031308 ? 12.92 * value : 1.055 * Math.max(0, value) ** (1 / 2.4) - 0.055;
  if (monitoringColorSpace.value === "rec709") {
    const encode709 = (value: number) => value < 0.018 ? 4.5 * value : 1.099 * Math.max(0, value) ** 0.45 - 0.099;
    return color.map(value => Math.max(0, Math.min(1, encode709(decodeSrgb(value))))) as [number, number, number];
  }
  if (monitoringColorSpace.value === "display-p3") {
    const [r, g, b] = color.map(decodeSrgb);
    const p3 = [0.8226 * r + 0.1775 * g, 0.0332 * r + 0.9668 * g, 0.0171 * r + 0.0724 * g + 0.9105 * b];
    return p3.map(value => Math.max(0, Math.min(1, encodeSrgb(value)))) as [number, number, number];
  }
  return color;
}

function sampleExternalLut(payload: LutPayload, r: number, g: number, b: number): [number, number, number] {
  const n = payload.size;
  const axis = (value: number, channel: number) => Math.max(0, Math.min(n - 1, ((value - payload.domain_min[channel]) / (payload.domain_max[channel] - payload.domain_min[channel])) * (n - 1)));
  const x = axis(r, 0), y = axis(g, 1), z = axis(b, 2);
  const x0 = Math.floor(x), y0 = Math.floor(y), z0 = Math.floor(z), x1 = Math.min(n - 1, x0 + 1), y1 = Math.min(n - 1, y0 + 1), z1 = Math.min(n - 1, z0 + 1);
  const index = (ri: number, gi: number, bi: number) => ri * n * n + gi * n + bi;
  const result: [number, number, number] = [0, 0, 0];
  for (let ri = 0; ri < 2; ri++) for (let gi = 0; gi < 2; gi++) for (let bi = 0; bi < 2; bi++) {
    const weight = (ri ? x - x0 : 1 - (x - x0)) * (gi ? y - y0 : 1 - (y - y0)) * (bi ? z - z0 : 1 - (z - z0));
    const sample = payload.samples[index(ri ? x1 : x0, gi ? y1 : y0, bi ? z1 : z0)];
    for (let channel = 0; channel < 3; channel++) result[channel] += sample[channel] * weight;
  }
  return result.map(value => Math.max(0, Math.min(1, value))) as [number, number, number];
}

function decodePreviewRgb(base64: string): Uint8Array {
  const binary = atob(base64);
  const source = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index++) source[index] = binary.charCodeAt(index);
  return source;
}

function compileMonitorShader(gl: WebGL2RenderingContext, type: number, source: string): WebGLShader {
  const shader = gl.createShader(type);
  if (!shader) throw new Error("unable to allocate monitor shader");
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const message = gl.getShaderInfoLog(shader) ?? "monitor shader compilation failed";
    gl.deleteShader(shader);
    throw new Error(message);
  }
  return shader;
}

function monitorLutTexture(id: string, payload?: LutPayload): { size: number; data: Uint8Array; domainMin: number[]; domainMax: number[]; key: string } {
  const imported = id.startsWith("imported:") && payload;
  const size = imported ? payload.size : id === "none" ? 2 : 33;
  const data = new Uint8Array(size * size * size * 4);
  const scale = Math.max(1, size - 1);
  for (let blue = 0; blue < size; blue++) for (let green = 0; green < size; green++) for (let red = 0; red < size; red++) {
    const input: [number, number, number] = [red / scale, green / scale, blue / scale];
    const sourceIndex = red * size * size + green * size + blue;
    const color = imported ? payload.samples[sourceIndex] : builtInLook(id, ...input);
    const output = ((blue * size + green) * size + red) * 4;
    data[output] = Math.round(Math.max(0, Math.min(1, color[0])) * 255);
    data[output + 1] = Math.round(Math.max(0, Math.min(1, color[1])) * 255);
    data[output + 2] = Math.round(Math.max(0, Math.min(1, color[2])) * 255);
    data[output + 3] = 255;
  }
  return {
    size,
    data,
    domainMin: imported ? [...payload.domain_min] : [0, 0, 0],
    domainMax: imported ? [...payload.domain_max] : [1, 1, 1],
    key: imported ? `${id}:${payload.size}:${payload.samples.length}` : id,
  };
}

class GpuMonitorRenderer {
  private readonly gl: WebGL2RenderingContext;
  private readonly program: WebGLProgram;
  private readonly sourceTexture: WebGLTexture;
  private readonly lutTexture: WebGLTexture;
  private readonly uniforms: Record<string, WebGLUniformLocation>;
  private sourceWidth = 0;
  private sourceHeight = 0;
  private lutKey = "";

  constructor(private readonly canvas: HTMLCanvasElement) {
    const gl = canvas.getContext("webgl2", {
      alpha: true,
      antialias: false,
      depth: false,
      stencil: false,
      premultipliedAlpha: false,
      preserveDrawingBuffer: false,
      powerPreference: "high-performance",
    });
    if (!gl) throw new Error("WebGL2 is unavailable");
    this.gl = gl;
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    const vertex = compileMonitorShader(gl, gl.VERTEX_SHADER, `#version 300 es
      in vec2 a_position;
      in vec2 a_texCoord;
      uniform vec2 u_texScale;
      uniform vec2 u_texOffset;
      out vec2 v_texCoord;
      void main() {
        gl_Position = vec4(a_position, 0.0, 1.0);
        v_texCoord = u_texOffset + a_texCoord * u_texScale;
      }
    `);
    const fragment = compileMonitorShader(gl, gl.FRAGMENT_SHADER, `#version 300 es
      precision highp float;
      precision highp sampler3D;
      uniform sampler2D u_source;
      uniform sampler3D u_lut;
      uniform vec3 u_domainMin;
      uniform vec3 u_domainMax;
      uniform vec3 u_peakingColor;
      uniform vec2 u_texel;
      uniform int u_applyLook;
      uniform int u_focus;
      uniform int u_zebra;
      uniform int u_colorSpace;
      in vec2 v_texCoord;
      out vec4 outColor;

      float decodeSrgb(float value) {
        return value <= 0.04045 ? value / 12.92 : pow((value + 0.055) / 1.055, 2.4);
      }
      float encodeSrgb(float value) {
        value = max(0.0, value);
        return value <= 0.0031308 ? 12.92 * value : 1.055 * pow(value, 1.0 / 2.4) - 0.055;
      }
      float encode709(float value) {
        value = max(0.0, value);
        return value < 0.018 ? 4.5 * value : 1.099 * pow(value, 0.45) - 0.099;
      }
      vec3 monitoringProfile(vec3 color) {
        if (u_colorSpace == 2) return color;
        vec3 linear = vec3(decodeSrgb(color.r), decodeSrgb(color.g), decodeSrgb(color.b));
        if (u_colorSpace == 0) {
          return clamp(vec3(encode709(linear.r), encode709(linear.g), encode709(linear.b)), 0.0, 1.0);
        }
        vec3 p3 = vec3(
          0.8226 * linear.r + 0.1775 * linear.g,
          0.0332 * linear.r + 0.9668 * linear.g,
          0.0171 * linear.r + 0.0724 * linear.g + 0.9105 * linear.b
        );
        return clamp(vec3(encodeSrgb(p3.r), encodeSrgb(p3.g), encodeSrgb(p3.b)), 0.0, 1.0);
      }
      float luma255(vec2 coordinate) {
        return dot(texture(u_source, coordinate).rgb, vec3(54.0, 183.0, 19.0));
      }
      void main() {
        vec3 original = texture(u_source, v_texCoord).rgb;
        vec3 color = original;
        float alpha = 0.0;
        if (u_applyLook == 1) {
          vec3 span = max(u_domainMax - u_domainMin, vec3(0.000001));
          color = texture(u_lut, clamp((original - u_domainMin) / span, 0.0, 1.0)).rgb;
          color = monitoringProfile(color);
          alpha = 1.0;
        }
        if (u_zebra == 1 && dot(original, vec3(0.2126, 0.7152, 0.0722)) >= 0.9216) {
          float stripe = mod(floor(gl_FragCoord.x) + floor(gl_FragCoord.y), 12.0) < 6.0 ? 1.0 : 0.047;
          color = vec3(stripe);
          alpha = 0.745;
        }
        if (u_focus == 1) {
          float tl = luma255(v_texCoord + u_texel * vec2(-1.0, -1.0));
          float tc = luma255(v_texCoord + u_texel * vec2( 0.0, -1.0));
          float tr = luma255(v_texCoord + u_texel * vec2( 1.0, -1.0));
          float ml = luma255(v_texCoord + u_texel * vec2(-1.0,  0.0));
          float mr = luma255(v_texCoord + u_texel * vec2( 1.0,  0.0));
          float bl = luma255(v_texCoord + u_texel * vec2(-1.0,  1.0));
          float bc = luma255(v_texCoord + u_texel * vec2( 0.0,  1.0));
          float br = luma255(v_texCoord + u_texel * vec2( 1.0,  1.0));
          float gx = -tl + tr - 2.0 * ml + 2.0 * mr - bl + br;
          float gy = -tl - 2.0 * tc - tr + bl + 2.0 * bc + br;
          if (length(vec2(gx, gy)) > 180.0) {
            color = u_peakingColor;
            alpha = 1.0;
          }
        }
        outColor = vec4(color, alpha);
      }
    `);
    const program = gl.createProgram();
    if (!program) throw new Error("unable to allocate monitor shader program");
    gl.attachShader(program, vertex);
    gl.attachShader(program, fragment);
    gl.linkProgram(program);
    gl.deleteShader(vertex);
    gl.deleteShader(fragment);
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      const message = gl.getProgramInfoLog(program) ?? "monitor shader link failed";
      gl.deleteProgram(program);
      throw new Error(message);
    }
    this.program = program;
    gl.useProgram(program);
    const buffer = gl.createBuffer();
    if (!buffer) throw new Error("unable to allocate monitor vertex buffer");
    gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([
      -1, -1, 0, 1,  1, -1, 1, 1,  -1, 1, 0, 0,
      -1, 1, 0, 0,   1, -1, 1, 1,   1, 1, 1, 0,
    ]), gl.STATIC_DRAW);
    const position = gl.getAttribLocation(program, "a_position");
    const texCoord = gl.getAttribLocation(program, "a_texCoord");
    gl.enableVertexAttribArray(position);
    gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 16, 0);
    gl.enableVertexAttribArray(texCoord);
    gl.vertexAttribPointer(texCoord, 2, gl.FLOAT, false, 16, 8);
    const uniform = (name: string) => {
      const location = gl.getUniformLocation(program, name);
      if (!location) throw new Error(`missing monitor shader uniform ${name}`);
      return location;
    };
    this.uniforms = Object.fromEntries([
      "u_texScale", "u_texOffset", "u_domainMin", "u_domainMax", "u_peakingColor", "u_texel",
      "u_applyLook", "u_focus", "u_zebra", "u_colorSpace",
    ].map((name) => [name, uniform(name)]));
    const sourceTexture = gl.createTexture();
    const lutTexture = gl.createTexture();
    if (!sourceTexture || !lutTexture) throw new Error("unable to allocate monitor textures");
    this.sourceTexture = sourceTexture;
    this.lutTexture = lutTexture;
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, sourceTexture);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.uniform1i(gl.getUniformLocation(program, "u_source"), 0);
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_3D, lutTexture);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_3D, gl.TEXTURE_WRAP_R, gl.CLAMP_TO_EDGE);
    gl.uniform1i(gl.getUniformLocation(program, "u_lut"), 1);
    this.updateLut("none", undefined);
  }

  private updateLut(id: string, payload?: LutPayload): void {
    const compiled = monitorLutTexture(id, payload);
    if (compiled.key === this.lutKey) return;
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_3D, this.lutTexture);
    gl.texImage3D(gl.TEXTURE_3D, 0, gl.RGBA8, compiled.size, compiled.size, compiled.size, 0, gl.RGBA, gl.UNSIGNED_BYTE, compiled.data);
    gl.uniform3fv(this.uniforms.u_domainMin, compiled.domainMin);
    gl.uniform3fv(this.uniforms.u_domainMax, compiled.domainMax);
    this.lutKey = compiled.key;
  }

  render(snapshot: CameraMonitorSnapshot, source: Uint8Array): void {
    const gl = this.gl;
    const bounds = this.canvas.getBoundingClientRect();
    const width = Math.max(1, Math.round(bounds.width));
    const height = Math.max(1, Math.round(bounds.height));
    if (this.canvas.width !== width || this.canvas.height !== height) {
      this.canvas.width = width;
      this.canvas.height = height;
    }
    gl.viewport(0, 0, width, height);
    gl.useProgram(this.program);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.sourceTexture);
    if (snapshot.preview_width !== this.sourceWidth || snapshot.preview_height !== this.sourceHeight) {
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGB8, snapshot.preview_width, snapshot.preview_height, 0, gl.RGB, gl.UNSIGNED_BYTE, source);
      this.sourceWidth = snapshot.preview_width;
      this.sourceHeight = snapshot.preview_height;
    } else {
      gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, snapshot.preview_width, snapshot.preview_height, gl.RGB, gl.UNSIGNED_BYTE, source);
    }
    const applyLook = monitorLookEnabled.checked;
    if (applyLook) this.updateLut(lutSelection.value, activeLutPayload);
    const sourceAspect = snapshot.preview_width / snapshot.preview_height;
    const destinationAspect = width / height;
    const scale = sourceAspect > destinationAspect
      ? [destinationAspect / sourceAspect, 1]
      : [1, sourceAspect / destinationAspect];
    gl.uniform2f(this.uniforms.u_texScale, scale[0], scale[1]);
    gl.uniform2f(this.uniforms.u_texOffset, (1 - scale[0]) / 2, (1 - scale[1]) / 2);
    gl.uniform2f(this.uniforms.u_texel, 1 / snapshot.preview_width, 1 / snapshot.preview_height);
    gl.uniform1i(this.uniforms.u_applyLook, applyLook ? 1 : 0);
    gl.uniform1i(this.uniforms.u_focus, document.body.classList.contains("tool-focus") ? 1 : 0);
    gl.uniform1i(this.uniforms.u_zebra, document.body.classList.contains("tool-zebra") ? 1 : 0);
    gl.uniform1i(this.uniforms.u_colorSpace, monitoringColorSpace.value === "rec709" ? 0 : monitoringColorSpace.value === "display-p3" ? 1 : 2);
    const [red, green, blue] = peakingColors[peakingColor.value] ?? peakingColors.cyan;
    gl.uniform3f(this.uniforms.u_peakingColor, red / 255, green / 255, blue / 255);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
  }
}

let gpuMonitorRenderer: GpuMonitorRenderer | undefined;
let monitorFramesRendered = 0;
let monitorStatsStartedAt = performance.now();
function recordMonitorFrame(): void {
  monitorFramesRendered += 1;
  const now = performance.now();
  const elapsed = now - monitorStatsStartedAt;
  if (elapsed < 1000) return;
  processedPreview.dataset.observedFps = ((monitorFramesRendered * 1000) / elapsed).toFixed(1);
  monitorFramesRendered = 0;
  monitorStatsStartedAt = now;
}

function activateCanvas2dMonitorRenderer(replaceCanvas = false): CanvasRenderingContext2D {
  if (replaceCanvas) {
    const replacement = processedPreview.cloneNode(false) as HTMLCanvasElement;
    processedPreview.replaceWith(replacement);
    processedPreview = replacement;
  }
  const context = processedPreview.getContext("2d", { alpha: true });
  if (!context) throw new Error("Canvas 2D monitor renderer is unavailable");
  processedContext = context;
  processedPreview.dataset.renderer = "canvas2d";
  processedPreview.dataset.targetFps = "10";
  return context;
}

try {
  gpuMonitorRenderer = new GpuMonitorRenderer(processedPreview);
  processedPreview.dataset.renderer = "webgl2";
  processedPreview.dataset.targetFps = "30";
} catch {
  activateCanvas2dMonitorRenderer();
}

function renderProcessedPreviewCpu(snapshot: CameraMonitorSnapshot, source: Uint8Array): void {
  const pixels = new Uint8ClampedArray(snapshot.preview_width * snapshot.preview_height * 4);
  const luma = new Float32Array(snapshot.preview_width * snapshot.preview_height);
  const applyLook = monitorLookEnabled.checked;
  for (let index = 0, output = 0; index < source.length; index += 3, output += 4) {
    const original: [number, number, number] = [source[index] / 255, source[index + 1] / 255, source[index + 2] / 255];
    const looked = applyLook ? (activeLutPayload ? sampleExternalLut(activeLutPayload, ...original) : builtInLook(lutSelection.value, ...original)) : original;
    const color = applyLook ? applyMonitoringProfile(looked) : looked;
    pixels[output] = color[0] * 255; pixels[output + 1] = color[1] * 255; pixels[output + 2] = color[2] * 255; pixels[output + 3] = applyLook ? 255 : 0;
    luma[output / 4] = original[0] * 54 + original[1] * 183 + original[2] * 19;
  }
  const focusActive = document.body.classList.contains("tool-focus");
  const zebraActive = document.body.classList.contains("tool-zebra");
  if (focusActive || zebraActive) {
    const width = snapshot.preview_width, height = snapshot.preview_height;
    if (zebraActive) {
      for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
        const p = y * width + x;
        if (luma[p] < 235) continue;
        const o = p * 4;
        const brightStripe = ((x + y) % 12) < 6;
        pixels[o] = brightStripe ? 255 : 12;
        pixels[o + 1] = brightStripe ? 255 : 12;
        pixels[o + 2] = brightStripe ? 255 : 12;
        pixels[o + 3] = 190;
      }
    }
    if (focusActive) {
    for (let y = 1; y < height - 1; y++) for (let x = 1; x < width - 1; x++) {
      const p = y * width + x;
      const gx = -luma[p - width - 1] + luma[p - width + 1] - 2 * luma[p - 1] + 2 * luma[p + 1] - luma[p + width - 1] + luma[p + width + 1];
      const gy = -luma[p - width - 1] - 2 * luma[p - width] - luma[p - width + 1] + luma[p + width - 1] + 2 * luma[p + width] + luma[p + width + 1];
      if (Math.hypot(gx, gy) > 180) {
        const o = p * 4;
        const [red, green, blue] = peakingColors[peakingColor.value] ?? peakingColors.cyan;
        pixels[o] = red; pixels[o + 1] = green; pixels[o + 2] = blue; pixels[o + 3] = 255;
      }
    }
    }
  }
  sampleCanvas.width = snapshot.preview_width; sampleCanvas.height = snapshot.preview_height;
  sampleContext.putImageData(new ImageData(pixels, snapshot.preview_width, snapshot.preview_height), 0, 0);
  const bounds = processedPreview.getBoundingClientRect();
  const width = Math.max(1, Math.round(bounds.width)), height = Math.max(1, Math.round(bounds.height));
  if (processedPreview.width !== width || processedPreview.height !== height) { processedPreview.width = width; processedPreview.height = height; }
  const scale = Math.max(width / sampleCanvas.width, height / sampleCanvas.height);
  const drawWidth = sampleCanvas.width * scale, drawHeight = sampleCanvas.height * scale;
  const context = processedContext ?? activateCanvas2dMonitorRenderer();
  context.clearRect(0, 0, width, height);
  context.drawImage(sampleCanvas, (width - drawWidth) / 2, (height - drawHeight) / 2, drawWidth, drawHeight);
}

function renderProcessedPreview(snapshot: CameraMonitorSnapshot): void {
  if (!snapshot.preview_rgb_base64 || !snapshot.preview_width || !snapshot.preview_height) return;
  const source = decodePreviewRgb(snapshot.preview_rgb_base64);
  if (gpuMonitorRenderer) {
    try {
      gpuMonitorRenderer.render(snapshot, source);
      recordMonitorFrame();
      return;
    } catch {
      gpuMonitorRenderer = undefined;
      activateCanvas2dMonitorRenderer(true);
    }
  }
  renderProcessedPreviewCpu(snapshot, source);
  recordMonitorFrame();
}

function setNativePreviewCompositing(active: boolean): void {
  document.documentElement.classList.toggle("has-native-preview", active);
  document.body.classList.toggle("has-native-preview", active);
}

function histogramPath(values: number[]): string {
  if (!values.length) return "M2 41H130V41Z";
  const peak = Math.max(1, ...values);
  const points = values.map((value, index) => {
    const x = 2 + (128 * index) / Math.max(1, values.length - 1);
    const y = 41 - (38 * Math.sqrt(value / peak));
    return `${x.toFixed(1)} ${y.toFixed(1)}`;
  });
  return `M2 41 L${points.join(" L")} L130 41Z`;
}

function audioLevelPercent(db: number | undefined): string {
  if (db === undefined || !Number.isFinite(db)) return "0%";
  return `${Math.max(0, Math.min(100, ((db + 60) / 60) * 100)).toFixed(1)}%`;
}

let lastMonitorTelemetryAt = 0;
async function updateMonitorTelemetry(): Promise<void> {
  if ((!nativePreviewRunning && !controlFixtureEnabled) || monitorTelemetryPending || document.hidden) return;
  const processing = monitorLookEnabled.checked || document.body.classList.contains("tool-focus") || document.body.classList.contains("tool-zebra");
  const interval = processing && gpuMonitorRenderer ? 32 : 100;
  const now = performance.now();
  if (now - lastMonitorTelemetryAt < interval) return;
  lastMonitorTelemetryAt = now;
  monitorTelemetryPending = true;
  try {
    const snapshot = controlFixtureEnabled
      ? controlMonitorFixture
      : await invoke<CameraMonitorSnapshot>("get_camera_monitor_snapshot", { includePreview: processing });
    if (snapshot.frame_received) {
      document.querySelector<SVGPathElement>("#hist-red")!.setAttribute("d", histogramPath(snapshot.red));
      document.querySelector<SVGPathElement>("#hist-green")!.setAttribute("d", histogramPath(snapshot.green));
      document.querySelector<SVGPathElement>("#hist-blue")!.setAttribute("d", histogramPath(snapshot.blue));
      if (processing) renderProcessedPreview(snapshot);
    }
    const channels = snapshot.audio_db.length === 1
      ? [snapshot.audio_db[0], snapshot.audio_db[0]]
      : snapshot.audio_db;
    document.querySelector<HTMLElement>("#audio-level-1")!.style.setProperty("--level", audioLevelPercent(channels[0]));
    document.querySelector<HTMLElement>("#audio-level-2")!.style.setProperty("--level", audioLevelPercent(channels[1]));
  } catch {
    // Browser fixtures and unsupported platforms have no native telemetry IPC.
  } finally {
    monitorTelemetryPending = false;
  }
}

window.setInterval(() => void updateMonitorTelemetry(), 16);
const devQuery = new URLSearchParams(window.location.search);
const recoveryFixtureEnabled = import.meta.env.DEV && devQuery.get("recovery-fixture") === "1";
const controlFixtureEnabled = import.meta.env.DEV && devQuery.get("control-fixture") === "1";
const storageLowFixtureEnabled = import.meta.env.DEV && devQuery.get("storage-low") === "1";
const nearbyFixtureEnabled = import.meta.env.DEV && devQuery.get("nearby-fixture") === "1";
const nearbyRetryFixtureEnabled = nearbyFixtureEnabled && devQuery.get("nearby-retry") === "1";
const nearbyFailureFixtureValue = devQuery.get("nearby-failure");
const nearbyFailureFixtureKind = nearbyFixtureEnabled && ["disconnected", "timeout", "integrity", "storage", "invitation_expired", "cancelled", "protocol"].includes(nearbyFailureFixtureValue ?? "")
  ? nearbyFailureFixtureValue as NonNullable<NonNullable<NearbyDiscoverySnapshot["approval"]>["failure_kind"]>
  : nearbyRetryFixtureEnabled ? "disconnected" : undefined;
const nearbyIncomingFixtureEnabled = nearbyFixtureEnabled && devQuery.get("nearby-incoming") === "1";

const controlMonitorFixture: CameraMonitorSnapshot = (() => {
  const width = 64;
  const height = 36;
  const rgb = new Uint8Array(width * height * 3);
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) {
    const offset = (y * width + x) * 3;
    rgb[offset] = Math.round((x / (width - 1)) * 255);
    rgb[offset + 1] = Math.round((y / (height - 1)) * 255);
    rgb[offset + 2] = Math.round(((x + y) / (width + height - 2)) * 255);
  }
  let binary = "";
  for (const value of rgb) binary += String.fromCharCode(value);
  const histogram = Array.from({ length: 32 }, (_, index) => Math.round(Math.sin((index / 31) * Math.PI) * 100));
  return {
    red: histogram,
    green: [...histogram].reverse(),
    blue: histogram.map((value, index) => Math.round(value * (0.55 + index / 64))),
    audio_db: [-24, -18],
    frame_received: true,
    preview_width: width,
    preview_height: height,
    preview_rgb_base64: btoa(binary),
  };
})();

function recoveryFixtureEntries(): MediaIndexEntry[] {
  const photoFixtures: MediaIndexEntry[] = Array.from({ length: 12 }, (_, index) => ({
    schema_version: 1,
    id: `UFC-photo-fixture-${index + 1}`,
    state: "finalized",
    media_type: "photo",
    resource_path: `/captures/UFC-photo-fixture-${index + 1}.jpg`,
    asset: null,
    error: null,
    updated_at_utc: new Date(Date.UTC(2026, 7, 31, 0, index)).toISOString()
  }));
  return [...photoFixtures, {
    schema_version: 1,
    id: "UFC-recovery-fixture",
    state: "failed",
    media_type: "video",
    resource_path: "/captures/.incomplete/UFC-recovery-fixture.mp4",
    asset: null,
    error: "video container did not contain a complete movie track",
    updated_at_utc: "2026-08-30T00:00:00Z"
  }];
}

function mediaStateLabel(state: MediaState): string {
  if (state === "finalized") return t.mediaReady;
  if (state === "incomplete") return t.mediaIncomplete;
  return t.mediaFailed;
}

function activeOutputPreset(): CaptureOutputPreset | undefined {
  return mode === "video" ? outputPresets?.video[0] : outputPresets?.still[0];
}

function storageAllowsCapture(): boolean {
  return mode === "video" ? storageStatus?.video_ready !== false : storageStatus?.photo_ready !== false;
}

function bytesLabel(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${value >= 10 || unit === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[unit]}`;
}

function presetLabel(preset: CaptureOutputPreset | undefined): string {
  if (!preset) return "—";
  if (preset.media_type === "photo") return preset.container.toUpperCase();
  return [preset.video_codec?.toUpperCase(), preset.audio_codec?.toUpperCase(), preset.container.toUpperCase()]
    .filter(Boolean).join(" / ");
}

function renderOutputStatus(): void {
  const preset = activeOutputPreset();
  const available = storageStatus?.available_bytes ?? 0;
  const storageReady = storageAllowsCapture();
  const label = presetLabel(preset);
  outputStatus.dataset.state = storageReady ? "default" : "error";
  outputStatus.querySelector("span")!.textContent = `${t.output} · ${label}`;
  document.querySelector<HTMLElement>("#output-preset")!.textContent = label;
  document.querySelector<HTMLElement>("#storage-remaining")!.textContent = storageStatus
    ? `${bytesLabel(available)} / ${bytesLabel(storageStatus.total_bytes)}`
    : t.storageUnavailable;
  const units = preset && available > 0
    ? Math.floor(available / preset.estimated_bytes_per_unit)
    : 0;
  document.querySelector<HTMLElement>("#storage-estimate")!.textContent = preset && storageStatus
    ? storageReady
      ? `${units.toLocaleString(activeLocale)} ${preset.media_type === "photo" ? t.photosRemaining : t.minutesRemaining}`
      : t.storageLow
    : "—";
}

async function refreshOutputStatus(): Promise<void> {
  try {
    if (recoveryFixtureEnabled || !("__TAURI_INTERNALS__" in window)) {
      outputPresets = {
        still: [{ id: "jpeg_high", media_type: "photo", container: "jpeg", video_codec: null, audio_codec: null, estimated_bytes_per_unit: 8 * 1024 * 1024 }],
        video: [{ id: "h264_aac_balanced", media_type: "video", container: "mp4", video_codec: "h264", audio_codec: "aac", estimated_bytes_per_unit: 120 * 1024 * 1024 }]
      };
      storageStatus = storageLowFixtureEnabled
        ? { path: "/captures", available_bytes: 128 * 1024 ** 2, total_bytes: 256 * 1024 ** 3, photo_ready: false, video_ready: false }
        : { path: "/captures", available_bytes: 128 * 1024 ** 3, total_bytes: 256 * 1024 ** 3, photo_ready: true, video_ready: true };
    } else {
      [outputPresets, storageStatus] = await Promise.all([
        invoke<CaptureOutputPresets>("get_capture_output_presets"),
        invoke<CaptureStorageStatus>("get_capture_storage_status")
      ]);
    }
  } catch {
    storageStatus = undefined;
  }
  renderOutputStatus();
  if (!recording) {
    captureButton.disabled = !nativePreviewRunning
      || (mode === "video" && microphoneAuthorization !== "authorized")
      || !storageAllowsCapture();
  }
}

function mediaFileName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

function mediaDate(value: string): string {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) return value;
  return new Intl.DateTimeFormat(activeLocale, {
    year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit"
  }).format(parsed);
}

function mediaDuration(milliseconds: number | null | undefined): string {
  if (!milliseconds) return "—";
  const seconds = Math.floor(milliseconds / 1000);
  const minutes = Math.floor(seconds / 60);
  return `${minutes.toString().padStart(2, "0")}:${(seconds % 60).toString().padStart(2, "0")}`;
}

function appendMediaText(parent: HTMLElement, className: string, value: string): HTMLElement {
  const element = document.createElement("span");
  element.className = className;
  element.textContent = value;
  parent.append(element);
  return element;
}

async function loadMediaPhotoPreview(entry: MediaIndexEntry, visual: HTMLElement): Promise<void> {
  try {
    const preview = await invoke<PhotoPreviewPayload>("get_media_photo_preview", { id: entry.id });
    if (!visual.isConnected || preview.id !== entry.id) return;
    const image = document.createElement("img");
    image.className = "media-card-thumbnail";
    image.alt = mediaFileName(entry.resource_path);
    image.decoding = "async";
    image.addEventListener("load", () => visual.classList.add("has-thumbnail"), { once: true });
    image.addEventListener("error", () => image.remove(), { once: true });
    image.src = `data:${preview.mime_type};base64,${preview.data_base64}`;
    visual.prepend(image);
  } catch {
    // Keep the explicit photo/state placeholder when preview decoding fails.
  }
}

function closeMediaContextMenu(): void {
  mediaContextMenu.hidden = true;
  contextMediaEntry = undefined;
}

function openMediaContextMenu(entry: MediaIndexEntry, x: number, y: number): void {
  contextMediaEntry = entry;
  const canSave = entry.state === "finalized" && entry.media_type === "photo";
  mediaContextSave.hidden = !canSave;
  mediaContextMenu.hidden = false;
  mediaContextMenu.style.left = "16px";
  mediaContextMenu.style.top = "16px";
  const bounds = mediaContextMenu.getBoundingClientRect();
  const left = Math.max(16, Math.min(x, window.innerWidth - bounds.width - 16));
  const top = Math.max(16, Math.min(y, window.innerHeight - bounds.height - 16));
  mediaContextMenu.style.left = `${left}px`;
  mediaContextMenu.style.top = `${top}px`;
  (canSave ? mediaContextSave : mediaContextDelete).focus();
}

function attachMediaContextMenu(card: HTMLElement, entry: MediaIndexEntry): void {
  card.tabIndex = 0;
  card.setAttribute("aria-haspopup", "menu");
  card.addEventListener("contextmenu", (event) => {
    event.preventDefault();
    openMediaContextMenu(entry, event.clientX, event.clientY);
  });
  let originX = 0;
  let originY = 0;
  card.addEventListener("pointerdown", (event) => {
    if (event.pointerType === "mouse") return;
    originX = event.clientX;
    originY = event.clientY;
    window.clearTimeout(mediaLongPressTimer);
    mediaLongPressTimer = window.setTimeout(() => openMediaContextMenu(entry, originX, originY), 550);
  });
  card.addEventListener("pointermove", (event) => {
    if (Math.hypot(event.clientX - originX, event.clientY - originY) > 10) window.clearTimeout(mediaLongPressTimer);
  });
  for (const eventName of ["pointerup", "pointercancel", "pointerleave"] as const) {
    card.addEventListener(eventName, () => window.clearTimeout(mediaLongPressTimer));
  }
  card.addEventListener("keydown", (event) => {
    if (event.key === "ContextMenu" || (event.shiftKey && event.key === "F10")) {
      event.preventDefault();
      const bounds = card.getBoundingClientRect();
      openMediaContextMenu(entry, bounds.left + bounds.width / 2, bounds.top + bounds.height / 2);
    }
  });
}

function renderMediaCard(entry: MediaIndexEntry): HTMLElement {
  const card = document.createElement("article");
  card.className = `media-card is-${entry.state}`;
  card.dataset.mediaId = entry.id;
  attachMediaContextMenu(card, entry);

  const visual = document.createElement("div");
  visual.className = "media-card-visual";
  visual.innerHTML = icon(entry.media_type === "photo" ? "photo" : "video");
  if (entry.media_type === "photo" && entry.state === "finalized" && "__TAURI_INTERNALS__" in window) {
    void loadMediaPhotoPreview(entry, visual);
  }
  appendMediaText(visual, "media-kind", entry.media_type === "photo" ? t.mediaPhoto : t.mediaVideo);
  appendMediaText(visual, "media-state", mediaStateLabel(entry.state));

  const body = document.createElement("div");
  body.className = "media-card-body";
  const title = document.createElement("h2");
  title.textContent = mediaFileName(entry.resource_path);
  const timestamp = document.createElement("time");
  timestamp.dateTime = entry.updated_at_utc;
  timestamp.textContent = mediaDate(entry.updated_at_utc);
  body.append(title, timestamp);

  const metadata = document.createElement("dl");
  const resource = entry.asset?.original;
  const pairs = [
    [t.format, resource ? `${resource.pixel_width}×${resource.pixel_height}` : "—"],
    [t.mediaDuration, mediaDuration(resource?.duration_ms)],
    ["ID", entry.id]
  ];
  for (const [label, value] of pairs) {
    const item = document.createElement("div");
    const term = document.createElement("dt");
    const detail = document.createElement("dd");
    term.textContent = label;
    detail.textContent = value;
    item.append(term, detail);
    metadata.append(item);
  }
  body.append(metadata);

  if (entry.state !== "finalized") {
    const diagnostic = document.createElement("p");
    diagnostic.className = "media-diagnostic";
    diagnostic.textContent = entry.error
      ? `${t.mediaValidationFailed}: ${entry.error}`
      : t.mediaAwaiting;
    body.append(diagnostic);
  }
  const details = document.createElement("button");
  details.className = "media-card-details";
  details.type = "button";
  details.textContent = t.mediaDetails;
  details.addEventListener("click", () => openMediaDetail(entry));
  body.append(details);
  card.append(visual, body);
  return card;
}

function openMediaDetail(entry: MediaIndexEntry): void {
  selectedMediaEntry = entry;
  document.querySelector<HTMLElement>("#media-detail-title")!.textContent = mediaFileName(entry.resource_path);
  const resource = entry.asset?.original;
  const pairs = [
    [t.mediaState, mediaStateLabel(entry.state)],
    [t.mediaUpdated, mediaDate(entry.updated_at_utc)],
    [t.mediaPath, entry.resource_path],
    [t.format, resource ? `${resource.pixel_width}×${resource.pixel_height}` : "—"],
    [t.mediaDuration, mediaDuration(resource?.duration_ms)],
    ["Asset ID", entry.id],
    ["Schema", String(entry.schema_version)]
  ];
  mediaDetailContent.replaceChildren(...pairs.map(([label, value]) => {
    const row = document.createElement("div");
    const term = document.createElement("dt");
    const detail = document.createElement("dd");
    term.textContent = label;
    detail.textContent = value;
    row.append(term, detail);
    return row;
  }));
  mediaDetailDiagnostic.hidden = !entry.error;
  mediaDetailDiagnostic.textContent = entry.error ?? "";
  mediaCleanup.hidden = entry.state === "finalized";
  mediaReinspect.hidden = entry.state === "finalized";
  mediaRecapture.hidden = entry.state === "finalized";
  mediaCleanup.dataset.state = "default";
  mediaReinspect.dataset.state = "default";
  mediaRecapture.dataset.state = "default";
  mediaDetailDialog.showModal();
}

function renderMediaIndex(): void {
  const filtered = mediaEntries.filter((entry) => mediaFilter === "all" || entry.state === mediaFilter);
  mediaGrid.dataset.view = mediaView;
  mediaGrid.replaceChildren(...filtered.map(renderMediaCard));
  mediaEmpty.hidden = filtered.length !== 0;
  document.querySelectorAll<HTMLElement>("[data-media-count]").forEach((count) => {
    const state = count.dataset.mediaCount as MediaFilter;
    count.textContent = String(state === "all" ? mediaEntries.length : mediaEntries.filter((entry) => entry.state === state).length);
  });
}

function closeDestinationMenu(): void {
  const toggle = document.querySelector<HTMLButtonElement>("#destination-tools-toggle")!;
  const destinations = document.querySelector<HTMLElement>(".destination-tools")!;
  toggle.setAttribute("aria-expanded", "false");
  toggle.classList.remove("is-active");
  destinations.classList.remove("is-open");
}

async function refreshCleanupCandidateCount(): Promise<void> {
  const candidates = recoveryFixtureEnabled
    ? recoveryFixtureEntries().filter((entry) => entry.state !== "finalized").map((entry) => ({ entry, age_seconds: 8 * 86_400, retention_expired: true }))
    : await invoke<RecoverableCleanupCandidate[]>("get_recoverable_cleanup_candidates");
  pendingBulkCleanupIds = candidates
    .filter((candidate) => candidate.retention_expired)
    .map((candidate) => candidate.entry.id);
  mediaCleanupExpired.hidden = pendingBulkCleanupIds.length === 0;
  mediaCleanupExpired.querySelector("span")!.textContent = `${t.mediaCleanupExpired} · ${pendingBulkCleanupIds.length}`;
}

async function loadMediaIndex(): Promise<void> {
  mediaLibrary.setAttribute("aria-busy", "true");
  mediaRefresh.disabled = true;
  mediaRefresh.dataset.state = "loading";
  mediaStatus.removeAttribute("data-state");
  mediaStatus.textContent = t.mediaLoading;
  try {
    mediaEntries = recoveryFixtureEnabled
      ? recoveryFixtureEntries()
      : await invoke<MediaIndexEntry[]>("reconcile_media_index");
    mediaEntries.sort((a, b) => b.updated_at_utc.localeCompare(a.updated_at_utc));
    await refreshCleanupCandidateCount();
    mediaStatus.textContent = "";
    mediaRefresh.dataset.state = "success";
    renderMediaIndex();
  } catch (error) {
    mediaEntries = [];
    mediaGrid.replaceChildren();
    mediaEmpty.hidden = true;
    mediaStatus.textContent = `${t.mediaLoadFailed} ${String(error)}`;
    mediaStatus.dataset.state = "error";
    mediaRefresh.dataset.state = "error";
    mediaCleanupExpired.hidden = true;
    pendingBulkCleanupIds = [];
  } finally {
    mediaLibrary.removeAttribute("aria-busy");
    mediaRefresh.disabled = false;
  }
}

async function openMediaLibrary(): Promise<void> {
  if (recording || !mediaLibrary.hidden) return;
  if (!settingsPage.hidden) await closeSettingsPage();
  if (!nearbyLibrary.hidden) await closeNearbyLibrary();
  mediaButton.disabled = true;
  mediaButton.dataset.state = "loading";
  if (nativePreviewRunning) {
    try {
      await invoke("stop_camera_preview");
      nativePreviewRunning = false;
      setNativePreviewCompositing(false);
    } catch (error) {
      feedback.textContent = `${t.mediaLoadFailed} ${String(error)}`;
      feedback.classList.add("is-visible");
      mediaButton.disabled = false;
      mediaButton.dataset.state = "error";
      return;
    }
  }
  monitor.hidden = true;
  mediaLibrary.hidden = false;
  document.body.classList.add("section-media");
  document.querySelectorAll<HTMLButtonElement>(".destination-tools button").forEach((button) => button.classList.remove("is-active"));
  mediaButton.classList.add("is-active");
  mediaButton.setAttribute("aria-pressed", "true");
  mediaButton.dataset.state = "default";
  mediaButton.disabled = false;
  closeDestinationMenu();
  await loadMediaIndex();
}

async function closeMediaLibrary(): Promise<void> {
  if (mediaLibrary.hidden) return;
  mediaLibrary.hidden = true;
  monitor.hidden = false;
  document.body.classList.remove("section-media");
  mediaButton.classList.remove("is-active");
  document.querySelector<HTMLButtonElement>(".destination-tools button:first-child")?.classList.add("is-active");
  mediaButton.setAttribute("aria-pressed", "false");
  mediaStatus.removeAttribute("data-state");
  await refreshCameraDiscovery();
  window.requestAnimationFrame(syncNativePreviewFrame);
}

function selectSettingsTab(name: string): void {
  document.querySelectorAll<HTMLButtonElement>("[data-settings-tab]").forEach((button) => {
    const active = button.dataset.settingsTab === name;
    button.setAttribute("aria-selected", String(active));
    button.tabIndex = active ? 0 : -1;
  });
  document.querySelectorAll<HTMLElement>(".settings-panel").forEach((panel) => {
    panel.hidden = panel.id !== `settings-panel-${name}`;
  });
}

async function openSettingsPage(): Promise<void> {
  if (recording || !settingsPage.hidden) return;
  if (!mediaLibrary.hidden) await closeMediaLibrary();
  if (!nearbyLibrary.hidden) await closeNearbyLibrary();
  settingsButton.disabled = true;
  if (nativePreviewRunning) {
    try {
      await invoke("stop_camera_preview");
      nativePreviewRunning = false;
      setNativePreviewCompositing(false);
    } catch (error) {
      feedback.textContent = String(error);
      feedback.classList.add("is-visible");
      settingsButton.disabled = false;
      return;
    }
  }
  monitor.hidden = true;
  settingsPage.hidden = false;
  document.body.classList.add("section-settings");
  document.querySelectorAll<HTMLButtonElement>(".destination-tools button").forEach((button) => button.classList.remove("is-active"));
  settingsButton.classList.add("is-active");
  settingsButton.setAttribute("aria-pressed", "true");
  settingsButton.disabled = false;
  closeDestinationMenu();
  selectSettingsTab("color");
  await Promise.all([loadLutCatalog(), refreshBulkPhotoCount()]);
}

async function closeSettingsPage(): Promise<void> {
  if (settingsPage.hidden) return;
  settingsPage.hidden = true;
  monitor.hidden = false;
  document.body.classList.remove("section-settings");
  settingsButton.classList.remove("is-active");
  settingsButton.setAttribute("aria-pressed", "false");
  document.querySelector<HTMLButtonElement>(".destination-tools button:first-child")?.classList.add("is-active");
  await refreshCameraDiscovery();
  window.requestAnimationFrame(syncNativePreviewFrame);
}

function nearbyFixtureSnapshot(active = true): NearbyDiscoverySnapshot {
  return {
    active,
    local_peer: active ? { ephemeral_id: "a19f30c24e81", display_label: null, port: 49172 } : null,
    peers: active ? [
      { ephemeral_id: "7bc5d01e94a2", display_label: "Studio B", protocol_version: 1, addresses: ["192.168.1.42"], port: 51309 },
      { ephemeral_id: "c2028f31aa09", display_label: null, protocol_version: 1, addresses: ["fe80::21a:7dff:fe3c:9081"], port: 49844 }
    ] : [],
    last_error: null,
    approval: nearbySnapshot?.approval ?? null
  };
}

function nearbyFailureLabel(kind: NonNullable<NonNullable<NearbyDiscoverySnapshot["approval"]>["failure_kind"]>): string {
  switch (kind) {
    case "disconnected": return t.nearbyInterrupted;
    case "timeout": return t.nearbyFailureTimeout;
    case "integrity": return t.nearbyFailureIntegrity;
    case "storage": return t.nearbyFailureStorage;
    case "invitation_expired": return t.nearbyFailureExpired;
    case "cancelled": return t.nearbyFailureCancelled;
    case "protocol": return t.nearbyFailureProtocol;
  }
}

function renderNearbySnapshot(): void {
  nearbyToggle.textContent = nearbySnapshot.active ? t.nearbyStop : t.nearbyStart;
  nearbyToggle.dataset.state = nearbySnapshot.active ? "active" : "default";
  document.querySelector<HTMLElement>("#nearby-local-id")!.textContent = nearbySnapshot.local_peer?.ephemeral_id ?? "—";
  const transfer = nearbySnapshot.approval;
  document.querySelector<HTMLButtonElement>("#nearby-back")!.disabled = transfer?.transfer_active === true;
  nearbyToggle.disabled = transfer?.transfer_active === true;
  nearbyStatus.textContent = transfer?.failure_kind
    ? nearbyFailureLabel(transfer.failure_kind)
    : nearbySnapshot.last_error ? nearbySnapshot.last_error
      : transfer?.cancel_requested ? t.nearbyCancelling
      : transfer?.finalized ? t.nearbyComplete
        : transfer?.transfer_active ? t.nearbyTransferring
          : nearbySnapshot.active ? t.nearbySearching : "";
  nearbyStatus.dataset.state = nearbySnapshot.last_error ? "error" : "default";
  nearbyEmpty.hidden = nearbySnapshot.peers.length !== 0;
  nearbyGrid.replaceChildren(...nearbySnapshot.peers.map((peer) => {
    const article = document.createElement("article");
    article.className = "nearby-peer";
    article.classList.toggle("is-selected", selectedNearbyPeerId === peer.ephemeral_id);
    const heading = document.createElement("div");
    const label = document.createElement("strong");
    label.textContent = peer.display_label || `${t.nearby} · ${peer.ephemeral_id.slice(0, 6)}`;
    const id = document.createElement("code");
    id.textContent = peer.ephemeral_id;
    heading.append(label, id);
    const details = document.createElement("dl");
    const rows: [string, string][] = [
      [t.nearbyProtocol, `UFC/${peer.protocol_version}`],
      [t.nearbyAddress, peer.addresses[0]
        ? `${peer.addresses[0].includes(":") ? `[${peer.addresses[0]}]` : peer.addresses[0]}:${peer.port}`
        : `—:${peer.port}`]
    ];
    for (const [term, value] of rows) {
      const row = document.createElement("div");
      const dt = document.createElement("dt");
      const dd = document.createElement("dd");
      dt.textContent = term;
      dd.textContent = value;
      row.append(dt, dd);
      details.append(row);
    }
    const select = document.createElement("button");
    select.type = "button";
    select.textContent = selectedNearbyPeerId === peer.ephemeral_id ? "✓" : t.nearbyPrepare;
    select.setAttribute("aria-label", `${t.nearbyPrepare}: ${peer.display_label || peer.ephemeral_id}`);
    select.setAttribute("aria-pressed", String(selectedNearbyPeerId === peer.ephemeral_id));
    select.addEventListener("click", () => {
      selectedNearbyPeerId = peer.ephemeral_id;
      renderNearbySnapshot();
      updateNearbyPrepareState();
    });
    article.append(heading, details, select);
    return article;
  }));
}

function updateNearbyPrepareState(): void {
  nearbyPrepare.disabled = !selectedNearbyPeerId || !nearbyAsset.value || !nearbySnapshot.active;
}

async function loadNearbyAssets(): Promise<void> {
  try {
    const entries = nearbyFixtureEnabled ? [{
      schema_version: 1, id: "UFC-finalized-demo", state: "finalized" as const, media_type: "photo" as const,
      resource_path: "/captures/UFC-finalized-demo.jpg", asset: null, error: null, updated_at_utc: "2026-08-31T00:00:00Z"
    }] : await invoke<MediaIndexEntry[]>("get_media_index");
    const options = entries.filter((entry) => entry.state === "finalized").map((entry) => {
      const option = document.createElement("option");
      option.value = entry.id;
      option.textContent = `${entry.media_type === "photo" ? t.mediaPhoto : t.mediaVideo} · ${mediaFileName(entry.resource_path)}`;
      return option;
    });
    nearbyAsset.replaceChildren(new Option(t.nearbySelectAsset, ""), ...options);
  } catch (error) {
    nearbyStatus.textContent = String(error);
    nearbyStatus.dataset.state = "error";
  }
  updateNearbyPrepareState();
}

function showNearbyApproval(): void {
  const approval = nearbySnapshot.approval;
  if (!approval) return;
  document.querySelector<HTMLElement>("#nearby-confirmation-code")!.textContent = approval.confirmation_code;
  const details = document.querySelector<HTMLElement>("#nearby-approval-detail")!;
  details.replaceChildren();
  for (const [term, value] of [
    [t.nearbyAsset, `${approval.file_name} · ${bytesLabel(approval.byte_length)}`],
    [approval.direction === "incoming" ? t.nearbyIncoming : t.nearby, approval.peer_ephemeral_id]
  ]) {
    const row = document.createElement("div");
    const dt = document.createElement("dt");
    const dd = document.createElement("dd");
    dt.textContent = term;
    dd.textContent = value;
    row.append(dt, dd);
    details.append(row);
  }
  const confirm = document.querySelector<HTMLButtonElement>("#nearby-approval-confirm")!;
  confirm.disabled = (approval.local_approved && !approval.retry_available) || approval.transfer_active === true;
  confirm.textContent = approval.retry_available ? t.nearbyRetry : approval.failure_kind ? t.nearbyFailed : approval.remote_approved
    ? approval.finalized ? t.nearbyComplete : approval.transfer_active ? t.nearbyTransferring : t.nearbySecure
    : approval.local_approved ? t.nearbyApproved : t.nearbyApprove;
  const transferred = Math.min(approval.transferred_bytes ?? 0, approval.byte_length);
  const percent = approval.byte_length > 0 ? Math.round((transferred / approval.byte_length) * 100) : 0;
  const progress = document.querySelector<HTMLElement>("#nearby-transfer-progress")!;
  progress.hidden = !approval.transfer_active && transferred === 0 && !approval.finalized;
  document.querySelector<HTMLProgressElement>("#nearby-progress-bar")!.value = approval.finalized ? 100 : percent;
  document.querySelector<HTMLElement>("#nearby-progress-value")!.textContent = approval.finalized
    ? "100%"
    : `${percent}% · ${bytesLabel(transferred)} / ${bytesLabel(approval.byte_length)}`;
  const cancel = document.querySelector<HTMLButtonElement>("#nearby-approval-cancel")!;
  cancel.textContent = approval.failure_kind && !approval.retry_available
    ? approval.direction === "incoming" ? t.nearbyDiscard : t.nearbyNewApproval
    : approval.cancel_requested ? t.nearbyCancelling
      : approval.transfer_active ? t.nearbyCancelTransfer : t.nearbyCancel;
  cancel.disabled = approval.cancel_requested === true || approval.finalized === true;
  document.querySelector<HTMLButtonElement>("#nearby-approval-close")!.disabled = approval.transfer_active === true;
  if (!nearbyApprovalDialog.open) nearbyApprovalDialog.showModal();
}

async function runNearbySecureTransfer(): Promise<void> {
  nearbyStatus.textContent = t.nearbyTransferring;
  try {
    nearbySnapshot = await invoke<NearbyDiscoverySnapshot>("run_nearby_secure_transfer");
    nearbyStatus.textContent = t.nearbyComplete;
    showNearbyApproval();
  } catch (error) {
    nearbyStatus.textContent = String(error);
    nearbyStatus.dataset.state = "error";
  }
}

async function prepareNearbyApproval(): Promise<void> {
  if (!selectedNearbyPeerId || !nearbyAsset.value) return;
  nearbyPrepare.disabled = true;
  try {
    nearbySnapshot = nearbyFixtureEnabled ? {
      ...nearbySnapshot,
      approval: { invitation_id: "inv-fixture", peer_ephemeral_id: selectedNearbyPeerId, asset_id: nearbyAsset.value,
        file_name: "UFC-finalized-demo.jpg", byte_length: 8_421_376, confirmation_code: "482913",
        expires_at_unix_ms: Date.now() + 120_000, local_approved: false, remote_approved: false,
        transferred_bytes: 0, transfer_active: false, cancel_requested: false, retry_available: false,
        failure_kind: undefined, direction: nearbyIncomingFixtureEnabled ? "incoming" : "outgoing", finalized: false }
    } : await invoke<NearbyDiscoverySnapshot>("prepare_nearby_approval", {
      request: { peer_ephemeral_id: selectedNearbyPeerId, asset_id: nearbyAsset.value }
    });
    showNearbyApproval();
  } catch (error) {
    nearbyStatus.textContent = String(error);
    nearbyStatus.dataset.state = "error";
  } finally {
    updateNearbyPrepareState();
  }
}

async function loadNearbyDiscovery(): Promise<void> {
  nearbyRefresh.disabled = true;
  const incomingRetryPending = nearbySnapshot.approval?.direction === "incoming"
    && nearbySnapshot.approval.retry_available === true;
  try {
    nearbySnapshot = nearbyFixtureEnabled
      ? nearbyFixtureSnapshot(nearbySnapshot.active)
      : await invoke<NearbyDiscoverySnapshot>("get_nearby_discovery");
  } catch (error) {
    nearbySnapshot = { ...nearbySnapshot, last_error: String(error) };
  } finally {
    nearbyRefresh.disabled = false;
    renderNearbySnapshot();
    if (nearbySnapshot.approval?.direction === "incoming" || nearbySnapshot.approval?.transfer_active) showNearbyApproval();
    if (incomingRetryPending && nearbySnapshot.approval?.direction === "incoming"
      && nearbySnapshot.approval.remote_approved && !nearbySnapshot.approval.retry_available
      && !nearbySnapshot.approval.transfer_active) {
      void runNearbySecureTransfer();
    }
  }
}

async function setNearbyDiscovery(active: boolean): Promise<void> {
  nearbyToggle.disabled = true;
  nearbyToggle.dataset.state = "loading";
  try {
    nearbySnapshot = nearbyFixtureEnabled
      ? nearbyFixtureSnapshot(active)
      : active
        ? await invoke<NearbyDiscoverySnapshot>("start_nearby_discovery", { request: { display_label: null, port: 0 } })
        : await invoke<NearbyDiscoverySnapshot>("stop_nearby_discovery");
    nearbySnapshot.last_error = null;
  } catch (error) {
    nearbySnapshot = { ...nearbySnapshot, last_error: String(error) };
  } finally {
    nearbyToggle.disabled = false;
    renderNearbySnapshot();
  }
}

async function openNearbyLibrary(): Promise<void> {
  if (recording || !nearbyLibrary.hidden) return;
  if (!mediaLibrary.hidden) await closeMediaLibrary();
  if (!settingsPage.hidden) await closeSettingsPage();
  nearbyButton.disabled = true;
  if (nativePreviewRunning) {
    try {
      await invoke("stop_camera_preview");
      nativePreviewRunning = false;
      setNativePreviewCompositing(false);
    } catch (error) {
      feedback.textContent = String(error);
      feedback.classList.add("is-visible");
      nearbyButton.disabled = false;
      return;
    }
  }
  monitor.hidden = true;
  nearbyLibrary.hidden = false;
  document.body.classList.add("section-nearby");
  document.querySelectorAll<HTMLButtonElement>(".destination-tools button").forEach((button) => button.classList.remove("is-active"));
  nearbyButton.classList.add("is-active");
  nearbyButton.setAttribute("aria-pressed", "true");
  nearbyButton.disabled = false;
  closeDestinationMenu();
  await setNearbyDiscovery(true);
  await loadNearbyAssets();
  nearbyPollId = window.setInterval(() => void loadNearbyDiscovery(), 1500);
}

async function closeNearbyLibrary(): Promise<void> {
  if (nearbyLibrary.hidden) return;
  if (nearbyPollId !== undefined) window.clearInterval(nearbyPollId);
  nearbyPollId = undefined;
  await setNearbyDiscovery(false);
  nearbyLibrary.hidden = true;
  monitor.hidden = false;
  document.body.classList.remove("section-nearby");
  nearbyButton.classList.remove("is-active");
  nearbyButton.setAttribute("aria-pressed", "false");
  document.querySelector<HTMLButtonElement>(".destination-tools button:first-child")?.classList.add("is-active");
  await refreshCameraDiscovery();
  window.requestAnimationFrame(syncNativePreviewFrame);
}

function previewViewport(): PreviewViewport {
  const rect = previewSurface.getBoundingClientRect();
  return { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
}

function captureOrientation(position = activeDevicePosition): CaptureOrientation {
  const legacyAngle = (window as Window & { orientation?: number }).orientation;
  const angle = Number.isFinite(screen.orientation?.angle)
    ? screen.orientation.angle
    : Number.isFinite(legacyAngle)
      ? legacyAngle!
      : 0;
  const screenAngle = ((Math.round(angle / 90) * 90) % 360 + 360) % 360;
  // CSS/Screen Orientation uses portrait-up as 0°, while the iPhone camera
  // sensor's native landscape orientation needs a 90° clockwise connection
  // rotation for portrait-up output.
  const normalized = ((90 - screenAngle + 360) % 360) as 0 | 90 | 180 | 270;
  return {
    rotation_degrees: normalized,
    preview_mirrored: position === "front",
    capture_mirrored: false
  };
}

function orientationKey(orientation: CaptureOrientation): string {
  return `${orientation.rotation_degrees}:${orientation.preview_mirrored}:${orientation.capture_mirrored}`;
}

async function syncNativeOrientation(): Promise<void> {
  if (!nativePreviewRunning || recording) return;
  const orientation = captureOrientation();
  const key = orientationKey(orientation);
  if (key === lastOrientationKey) return;
  try {
    const applied = await invoke<CaptureOrientation>("set_camera_orientation", { orientation });
    lastOrientationKey = orientationKey(applied);
  } catch (error) {
    feedback.textContent = `${t.orientationFailed}: ${String(error)}`;
    feedback.classList.add("is-visible");
    captureButton.setAttribute("aria-label", feedback.textContent);
    window.setTimeout(() => feedback.classList.remove("is-visible"), 5000);
  }
}

function resolutionLabel(width: number, height: number): string {
  return `${width}×${height}`;
}

function selectedFormatCapability() {
  if (!currentCapabilities) return undefined;
  const [width, height] = formatResolution.value.split("x").map(Number);
  return currentCapabilities.formats.find((format) => format.width === width && format.height === height);
}

function populateFrameRates(preferred?: number): void {
  const format = selectedFormatCapability();
  formatFps.replaceChildren(...(format?.frame_rates ?? []).map((fps) => {
    const option = document.createElement("option");
    option.value = String(fps);
    const cadence = fps === 24 ? "CINEMA" : fps === 25 ? "PAL" : fps === 30 ? "NTSC" : fps >= 50 ? "HFR" : "";
    option.textContent = `${fps} fps${cadence ? ` · ${cadence}` : ""}`;
    return option;
  }));
  if (preferred && format?.frame_rates.includes(preferred)) formatFps.value = String(preferred);
}

function populateFormatPanel(capabilities: CameraCapabilities): void {
  currentCapabilities = capabilities;
  const formats = [...capabilities.formats]
    .filter((format) => format.frame_rates.length > 0)
    .sort((a, b) => (b.width * b.height) - (a.width * a.height));
  formatResolution.replaceChildren(...formats.map((format) => {
    const option = document.createElement("option");
    option.value = `${format.width}x${format.height}`;
    option.textContent = resolutionLabel(format.width, format.height);
    return option;
  }));
  populateFrameRates();
}

function applyCapabilities(capabilities: CameraCapabilities): void {
  const lens = document.querySelector<HTMLButtonElement>('[data-parameter="lens"]')!;
  const fpsControl = document.querySelector<HTMLButtonElement>('[data-parameter="fps"]')!;
  const shutter = document.querySelector<HTMLButtonElement>('[data-parameter="shutter"]')!;
  const iris = document.querySelector<HTMLButtonElement>('[data-parameter="iris"]')!;
  const ei = document.querySelector<HTMLButtonElement>('[data-parameter="ei"]')!;
  const wb = document.querySelector<HTMLButtonElement>('[data-parameter="wb"]')!;
  lens.disabled = availableDevices.length === 0;
  fpsControl.disabled = capabilities.formats.length === 0;
  iris.disabled = false;
  wb.disabled = !capabilities.manual_white_balance;
  lens.querySelector("strong")!.textContent = capabilities.lens_label ?? "FIXED";
  iris.querySelector("strong")!.textContent = capabilities.lens_aperture
    ? `ƒ/${capabilities.lens_aperture.toFixed(1)}` : "FIXED";
  const shutterSeconds = capabilities.current_shutter_seconds;
  if (shutterSeconds && shutterSeconds > 0) {
    shutter.querySelector("strong")!.textContent = `1/${Math.max(1, Math.round(1 / shutterSeconds))}`;
  }
  const wbKelvin = capabilities.current_white_balance_kelvin;
  wb.querySelector("strong")!.textContent = wbKelvin
    ? `${Math.round(wbKelvin / 50) * 50}K` : "AUTO";
  shutter.disabled = !capabilities.manual_shutter;
  ei.disabled = capabilities.manual_iso === null;
  if (capabilities.current_iso) ei.querySelector("strong")!.textContent = String(Math.round(capabilities.current_iso));
  if (!capabilities.manual_shutter) shutter.querySelector("strong")!.textContent = "AUTO";
  if (capabilities.manual_iso === null) ei.querySelector("strong")!.textContent = "AUTO";
  if (ei.disabled) {
    ei.classList.remove("is-selected");
    ei.setAttribute("aria-pressed", "false");
  }
  const dynamicRange = capabilities.log_video ? "LOG" : capabilities.hdr_video ? "HDR" : "SDR";
  activeFormatDetail.dataset.dynamicRange = dynamicRange;
  populateFormatPanel(capabilities);
}

function applyActiveFormat(format: PreviewStatus["active_format"]): void {
  if (!format) return;
  const fps = Math.max(1, Math.round(format.fps));
  recordingFrameRate = fps;
  document.querySelector<HTMLElement>('[data-parameter="fps"] strong')!.textContent = String(fps);
  activeResolution.textContent = resolutionLabel(format.width, format.height);
  activeFormatDetail.textContent = `${format.width}×${format.height} · ${fps} FPS · ${activeFormatDetail.dataset.dynamicRange ?? "SDR"}`;
  const resolution = `${format.width}x${format.height}`;
  if ([...formatResolution.options].some((option) => option.value === resolution)) {
    formatResolution.value = resolution;
    populateFrameRates(fps);
  }
}

async function startNativePreview(result: CameraDiscovery, preferredDeviceId?: string): Promise<void> {
  const device = result.authorization === "authorized"
    ? result.devices.find((candidate) => candidate.id === preferredDeviceId) ?? result.devices[0]
    : undefined;
  if (!device || nativePreviewStarting || (nativePreviewRunning && activeDeviceId === device.id)) return;
  nativePreviewStarting = true;
  signalMessage.textContent = t.previewStarting;
  try {
    try {
      applyCapabilities(await invoke<CameraCapabilities>("get_camera_capabilities", { deviceId: device.id }));
    } catch { /* Preview can continue even if capability inspection fails. */ }
    const status = await invoke<PreviewStatus>("start_camera_preview", {
      deviceId: device.id,
      viewport: previewViewport(),
      orientation: captureOrientation(device.position)
    });
    nativePreviewRunning = status.running;
    activeDeviceId = status.device_id;
    activeDevicePosition = device.position;
    lastOrientationKey = orientationKey(status.orientation);
    applyActiveFormat(status.active_format);
    if (status.settings_warning) {
      feedback.textContent = `${t.formatPersistenceFailed}: ${status.settings_warning}`;
      feedback.classList.add("is-visible");
      window.setTimeout(() => feedback.classList.remove("is-visible"), 5000);
    }
    setNativePreviewCompositing(status.running);
    captureButton.disabled = !status.running || mode === "video" || !storageAllowsCapture();
  } catch (error) {
    signalTitle.textContent = t.previewFailed;
    signalMessage.textContent = String(error);
  } finally {
    nativePreviewStarting = false;
  }
}

function renderDiscovery(result: CameraDiscovery): void {
  availableDevices = result.devices;
  accessButton.hidden = true;
  captureButton.disabled = true;
  if (result.authorization === "not_determined") {
    signalTitle.textContent = t.noSignal;
    signalMessage.textContent = t.backendMessage;
    accessButton.hidden = false;
    return;
  }
  if (result.authorization === "denied") {
    signalTitle.textContent = t.noSignal;
    signalMessage.textContent = t.denied;
    return;
  }
  if (result.authorization === "restricted") {
    signalTitle.textContent = t.noSignal;
    signalMessage.textContent = t.restricted;
    return;
  }
  if (result.authorization === "unavailable") {
    signalTitle.textContent = t.noSignal;
    signalMessage.textContent = t.unavailable;
    return;
  }
  if (result.devices.length === 0) {
    signalTitle.textContent = t.noSignal;
    signalMessage.textContent = t.noDevice;
    return;
  }
  signalTitle.textContent = t.detected;
  signalMessage.textContent = `${result.devices[0].label} · ${t.previewPending}`;
}

async function refreshCameraDiscovery(): Promise<void> {
  if (controlFixtureEnabled) {
    availableDevices = [
      { id: "ultra", label: "Ultra Wide 13mm", position: "back" },
      { id: "wide", label: "Wide 26mm", position: "back" },
      { id: "tele", label: "Telephoto 77mm", position: "back" }
    ];
    applyCapabilities({
      supports_still: true, supports_video: true, supports_audio: true,
      resolutions: [[1920, 1080]], frame_rates: [24, 30],
      formats: [{ width: 1920, height: 1080, frame_rates: [24, 30] }],
      manual_iso: [32, 1600], manual_shutter: true, manual_focus: true, current_iso: 400,
      lens_label: "Wide 26mm", lens_aperture: 1.6, current_shutter_seconds: 1 / 48,
      manual_white_balance: true, current_white_balance_kelvin: 5600,
      raw_photo: false, log_video: false, hdr_video: false
    });
    applyActiveFormat({ width: 1920, height: 1080, fps: 30, settings_persisted: true, settings_warning: null });
    return;
  }
  try {
    const result = await invoke<CameraDiscovery>("get_camera_discovery");
    renderDiscovery(result);
    await startNativePreview(result);
  } catch { /* Browser preview has no Tauri IPC. */ }
}

accessButton.addEventListener("click", async () => {
  accessButton.disabled = true;
  accessButton.textContent = t.requesting;
  try {
    const result = await invoke<CameraDiscovery>("request_camera_authorization");
    renderDiscovery(result);
    await startNativePreview(result);
  } catch (error) {
    signalMessage.textContent = String(error);
  } finally {
    accessButton.disabled = false;
    accessButton.textContent = t.allowCamera;
  }
});

if (!nearbyFixtureEnabled) void refreshCameraDiscovery();

let resizeFrame: number | undefined;
let resizeSettleTimer: number | undefined;
function syncNativePreviewFrame(): void {
  if (!nativePreviewRunning) return;
  if (resizeFrame !== undefined) window.cancelAnimationFrame(resizeFrame);
  resizeFrame = window.requestAnimationFrame(() => {
    resizeFrame = window.requestAnimationFrame(() => {
      resizeFrame = undefined;
      void invoke("resize_camera_preview", { viewport: previewViewport() });
    });
  });
  if (resizeSettleTimer !== undefined) window.clearTimeout(resizeSettleTimer);
  resizeSettleTimer = window.setTimeout(() => {
    resizeSettleTimer = undefined;
    void invoke("resize_camera_preview", { viewport: previewViewport() });
  }, 120);
}

new ResizeObserver(syncNativePreviewFrame).observe(previewSurface);
window.addEventListener("resize", syncNativePreviewFrame);
let orientationSettleTimer: number | undefined;
function scheduleOrientationSync(): void {
  if (orientationSettleTimer !== undefined) window.clearTimeout(orientationSettleTimer);
  window.requestAnimationFrame(() => window.requestAnimationFrame(() => {
    void syncNativeOrientation();
    syncNativePreviewFrame();
  }));
  orientationSettleTimer = window.setTimeout(() => {
    orientationSettleTimer = undefined;
    void syncNativeOrientation();
    syncNativePreviewFrame();
  }, 180);
}
screen.orientation?.addEventListener("change", scheduleOrientationSync);
window.addEventListener("orientationchange", scheduleOrientationSync);

window.addEventListener("beforeunload", () => {
  if (nearbySnapshot.active) void invoke("stop_nearby_discovery");
  if (nativePreviewRunning && recording) {
    void invoke("stop_video_recording").finally(() => invoke("stop_camera_preview"));
  } else if (nativePreviewRunning) {
    void invoke("stop_camera_preview");
  }
});

function formatTimecode(elapsedMs: number): string {
  const totalFrames = Math.floor(elapsedMs / (1000 / recordingFrameRate));
  const frames = totalFrames % recordingFrameRate;
  const totalSeconds = Math.floor(totalFrames / recordingFrameRate);
  const seconds = totalSeconds % 60;
  const minutes = Math.floor(totalSeconds / 60) % 60;
  const hours = Math.floor(totalSeconds / 3600);
  return [hours, minutes, seconds, frames].map((value) => value.toString().padStart(2, "0")).join(":");
}

function updateRecordingUI(): void {
  captureButton.classList.toggle("is-recording", recording);
  captureButton.setAttribute("aria-label", recording ? t.stop : mode === "video" ? t.record : t.capture);
  indicator.classList.toggle("is-live", recording);
  document.body.classList.toggle("is-recording", recording);
  document.querySelectorAll<HTMLButtonElement>("[data-mode]").forEach((button) => {
    button.disabled = recording;
  });
  document.querySelector<HTMLButtonElement>('[data-parameter="fps"]')!.disabled =
    recording || !currentCapabilities?.formats.length;
  if (recording) formatPanel.hidden = true;
  if (!recording) return;
  timecode.textContent = formatTimecode(performance.now() - recordingStartedAt);
}

function stopRecording(): void {
  recording = false;
  if (timerId !== undefined) window.clearInterval(timerId);
  timerId = undefined;
  if (storageMonitorId !== undefined) window.clearInterval(storageMonitorId);
  storageMonitorId = undefined;
  recordingStopPending = false;
  updateRecordingUI();
  void syncNativeOrientation();
}

async function finishVideoRecording(storageTriggered = false): Promise<void> {
  if (!recording || recordingStopPending) return;
  recordingStopPending = true;
  captureButton.disabled = true;
  captureButton.dataset.state = "loading";
  try {
    const asset = await invoke<CaptureAsset>("stop_video_recording");
    stopRecording();
    captureButton.dataset.state = "success";
    const path = asset.original.path;
    const warning = asset.validation.status === "warning" ? ` · ${t.assetMetadataWarning}` : "";
    feedback.textContent = storageTriggered
      ? `${t.storageAutoStop} · ${path.split("/").pop() ?? path}${warning}`
      : `${t.videoSaved} · ${path.split("/").pop() ?? path}${warning}`;
    await refreshOutputStatus();
  } catch (error) {
    stopRecording();
    captureButton.dataset.state = "error";
    feedback.textContent = `${t.recordingFailed}: ${String(error)}`;
    captureButton.setAttribute("aria-label", feedback.textContent);
  }
  feedback.classList.add("is-visible");
  window.setTimeout(() => {
    captureButton.dataset.state = "default";
    captureButton.disabled = !nativePreviewRunning
      || microphoneAuthorization !== "authorized"
      || !storageAllowsCapture();
    feedback.classList.remove("is-visible");
  }, storageTriggered ? 5000 : 1800);
}

async function monitorRecordingStorage(): Promise<void> {
  if (!recording || recordingStopPending || storageCheckPending) return;
  storageCheckPending = true;
  try {
    storageStatus = await invoke<CaptureStorageStatus>("get_capture_storage_status");
    renderOutputStatus();
    if (!storageStatus.video_ready) await finishVideoRecording(true);
  } catch {
    // A transient capacity query failure must not discard an active recording.
  } finally {
    storageCheckPending = false;
  }
}

document.addEventListener("visibilitychange", () => {
  if (document.hidden) {
    if (recording) {
      recordingPausedByLifecycle = true;
      void finishVideoRecording().finally(() => {
        nativePreviewRunning = false;
        activeDeviceId = undefined;
      });
    }
    return;
  }
  if (!recordingPausedByLifecycle) return;
  recordingPausedByLifecycle = false;
  void refreshCameraDiscovery();
});

// Polling is deliberately low-frequency and only active in the foreground.
// It covers external-camera hot unplug on backends that cannot yet forward a
// native device notification into the webview.
window.setInterval(() => {
  if (document.hidden || !nativePreviewRunning || !activeDeviceId) return;
  const expectedDevice = activeDeviceId;
  void Promise.all([
    invoke<CameraDiscovery>("get_camera_discovery"),
    invoke<CameraRuntimeHealth>("get_camera_runtime_health")
  ]).then(([discovery, health]) => {
    if (health.preview_attached && !health.session_running) {
      if (recording || health.recording_pending) void finishVideoRecording();
      nativePreviewRunning = false;
      activeDeviceId = undefined;
      void refreshCameraDiscovery();
      return;
    }
    if (discovery.devices.some((device) => device.id === expectedDevice)) return;
    if (recording) void finishVideoRecording();
    nativePreviewRunning = false;
    activeDeviceId = undefined;
    void refreshCameraDiscovery();
  }).catch(() => { /* A transient discovery error is not a confirmed disconnect. */ });
}, 5_000);

if ("__TAURI_INTERNALS__" in window) {
  void getCurrentWindow().onCloseRequested(async (event) => {
    event.preventDefault();
    if (recording) await finishVideoRecording();
    if (nativePreviewRunning) {
      try { await invoke("stop_camera_preview"); } catch { /* Continue close after best-effort teardown. */ }
    }
    await getCurrentWindow().destroy();
  });
}

async function selectMode(nextMode: CameraMode): Promise<void> {
  if (recording) return;
  captureButton.disabled = true;
  mode = nextMode;
  document.querySelectorAll<HTMLButtonElement>("[data-mode]").forEach((button) => {
    const active = button.dataset.mode === mode;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", String(active));
  });
  captureButton.classList.toggle("is-video", mode === "video");
  captureButton.setAttribute("aria-label", mode === "video" ? t.record : t.capture);
  renderOutputStatus();
  try { await invoke("select_camera_mode", { mode }); } catch { /* Browser preview has no Tauri IPC. */ }
  if (mode === "video") {
    try {
      microphoneAuthorization = await invoke<CameraAuthorization>("get_microphone_authorization");
      if (microphoneAuthorization === "not_determined") {
        microphoneAuthorization = await invoke<CameraAuthorization>("request_microphone_authorization");
      }
      if (microphoneAuthorization !== "authorized") {
        feedback.textContent = t.microphoneDenied;
        feedback.classList.add("is-visible");
        window.setTimeout(() => feedback.classList.remove("is-visible"), 1800);
      }
    } catch (error) {
      microphoneAuthorization = "unavailable";
      const message = `${t.microphoneDenied} ${String(error)}`;
      feedback.textContent = message;
      captureButton.setAttribute("aria-label", `${t.record}: ${message}`);
      feedback.classList.add("is-visible");
      window.setTimeout(() => feedback.classList.remove("is-visible"), 5000);
    }
  }
  captureButton.disabled = !nativePreviewRunning
    || (mode === "video" && microphoneAuthorization !== "authorized")
    || !storageAllowsCapture();
}

document.querySelectorAll<HTMLButtonElement>("[data-mode]").forEach((button) => {
  button.addEventListener("click", async () => {
    if (!mediaLibrary.hidden) await closeMediaLibrary();
    if (!nearbyLibrary.hidden) await closeNearbyLibrary();
    await selectMode(button.dataset.mode as CameraMode);
  });
});

mediaButton.addEventListener("click", () => void openMediaLibrary());
nearbyButton.addEventListener("click", () => void openNearbyLibrary());
nearbyToggle.addEventListener("click", () => void setNearbyDiscovery(!nearbySnapshot.active));
nearbyRefresh.addEventListener("click", () => void loadNearbyDiscovery());
document.querySelector<HTMLButtonElement>("#nearby-back")!.addEventListener("click", () => void closeNearbyLibrary());
settingsButton.addEventListener("click", () => void openSettingsPage());
document.querySelector<HTMLButtonElement>("#settings-back")!.addEventListener("click", () => void closeSettingsPage());
document.querySelectorAll<HTMLButtonElement>("[data-settings-tab]").forEach((button) => {
  button.addEventListener("click", () => {
    selectSettingsTab(button.dataset.settingsTab!);
    if (button.dataset.settingsTab === "media") void refreshBulkPhotoCount();
  });
  button.addEventListener("keydown", (event) => {
    if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return;
    event.preventDefault();
    const tabs = [...document.querySelectorAll<HTMLButtonElement>("[data-settings-tab]")];
    const index = tabs.indexOf(button);
    const next = tabs[(index + (event.key === "ArrowRight" ? 1 : -1) + tabs.length) % tabs.length];
    selectSettingsTab(next.dataset.settingsTab!);
    next.focus();
  });
});
monitoringColorSpace.addEventListener("change", () => {
  try { window.localStorage.setItem(monitoringColorSpaceKey, monitoringColorSpace.value); } catch { /* keep session preference */ }
  applyMonitoringColorSpace(monitoringColorSpace.value);
  settingsStatus.textContent = t.colorSpaceSaved;
});
monitorLookEnabled.addEventListener("change", () => {
  try { window.localStorage.setItem(monitorLookEnabledKey, String(monitorLookEnabled.checked)); } catch { /* keep session preference */ }
  syncMonitorProcessing();
});
guideStyle.addEventListener("change", () => {
  try { window.localStorage.setItem(guideStyleKey, guideStyle.value); } catch { /* keep session preference */ }
  applyGuideStyle(guideStyle.value);
});
peakingColor.addEventListener("change", () => {
  try { window.localStorage.setItem(peakingColorKey, peakingColor.value); } catch { /* keep session preference */ }
  applyPeakingColor(peakingColor.value);
  displaySettingsStatus.textContent = t.peakingColorSaved;
});
shutterSound.addEventListener("change", () => {
  try { window.localStorage.setItem(shutterSoundKey, shutterSound.value); } catch { /* keep session preference */ }
  applyShutterSound(shutterSound.value);
  playShutterSound();
});
shutterSoundFile.addEventListener("change", async () => {
  const file = shutterSoundFile.files?.[0];
  if (!file) return;
  shutterSoundStatus.dataset.state = "";
  try {
    if (file.size === 0 || file.size > 5 * 1024 * 1024) throw new Error(t.shutterSoundImportHint);
    await storeCustomShutterSound(file);
    await loadCustomShutterSound();
    applyShutterSound("custom");
    window.localStorage.setItem(shutterSoundKey, "custom");
    shutterSoundStatus.textContent = `${t.shutterSoundImported} · ${customShutterSoundName}`;
    playShutterSound("custom");
  } catch (error) {
    shutterSoundStatus.textContent = `${t.shutterSoundImportFailed}: ${String(error)}`;
    shutterSoundStatus.dataset.state = "error";
  } finally { shutterSoundFile.value = ""; }
});
lutSelection.addEventListener("change", async () => {
  try { window.localStorage.setItem(selectedLutKey, lutSelection.value); } catch { /* keep session preference */ }
  lutStatus.textContent = lutSelection.options[lutSelection.selectedIndex]?.textContent ?? "";
  await loadSelectedLutPayload();
});
lutImportFile.addEventListener("change", async () => {
  const file = lutImportFile.files?.[0];
  if (!file) return;
  lutStatus.textContent = "";
  try {
    if (file.size > 4 * 1024 * 1024) throw new Error(t.lutImportHint);
    const imported = await invoke<LutEntry>("import_lut", { fileName: file.name, content: await file.text() });
    await loadLutCatalog();
    lutSelection.value = imported.id;
    try { window.localStorage.setItem(selectedLutKey, imported.id); } catch { /* keep session preference */ }
    lutStatus.textContent = `${t.lutImported}: ${imported.name} · ${imported.size}³`;
    await loadSelectedLutPayload();
  } catch (error) {
    lutStatus.textContent = `${t.lutImportFailed}: ${String(error)}`;
    lutStatus.dataset.state = "error";
  } finally {
    lutImportFile.value = "";
  }
});
applyMonitoringColorSpace(loadMonitoringColorSpace());
applyGuideStyle(storedPreference(guideStyleKey, "thirds"));
applyPeakingColor(storedPreference(peakingColorKey, "cyan"));
void loadCustomShutterSound().then(() => applyShutterSound(storedPreference(shutterSoundKey, "standard"))).catch(() => applyShutterSound("standard"));
monitorLookEnabled.checked = storedPreference(monitorLookEnabledKey, "false") === "true";
syncMonitorProcessing();
void loadLutCatalog();
bulkPhotoStart.addEventListener("click", async () => {
  const count = await refreshBulkPhotoCount();
  if (count === 0) return;
  document.querySelector<HTMLElement>("#bulk-photo-dialog-count")!.textContent = `${count} ${t.mediaPhoto}`;
  bulkPhotoConfirm.dataset.state = "default";
  bulkPhotoDialog.showModal();
});
document.querySelector<HTMLButtonElement>("#bulk-photo-cancel")!.addEventListener("click", () => bulkPhotoDialog.close());
bulkPhotoDialog.addEventListener("click", (event) => {
  if (event.target === bulkPhotoDialog) bulkPhotoDialog.close();
});
bulkPhotoConfirm.addEventListener("click", async () => {
  bulkPhotoConfirm.disabled = true;
  bulkPhotoConfirm.dataset.state = "loading";
  bulkPhotoStart.disabled = true;
  bulkPhotoStatus.textContent = t.bulkPhotoRunning;
  bulkPhotoStatus.dataset.state = "loading";
  try {
    const result = await invoke<BulkPhotoMigrationResult>("export_all_photos_and_delete");
    bulkPhotoDialog.close();
    await refreshBulkPhotoCount();
    bulkPhotoStatus.textContent = result.exported === 0
      ? t.bulkPhotoEmpty
      : `${t.bulkPhotoComplete} ${result.deleted}/${result.exported}`;
    bulkPhotoStatus.dataset.state = "success";
  } catch (error) {
    bulkPhotoDialog.close();
    await refreshBulkPhotoCount();
    bulkPhotoStatus.textContent = `${t.bulkPhotoFailed} ${String(error)}`;
    bulkPhotoStatus.dataset.state = "error";
  } finally {
    bulkPhotoConfirm.disabled = false;
  }
});
nearbyAsset.addEventListener("change", updateNearbyPrepareState);
nearbyPrepare.addEventListener("click", () => void prepareNearbyApproval());
document.querySelector<HTMLButtonElement>("#nearby-approval-close")!.addEventListener("click", () => nearbyApprovalDialog.close());
document.querySelector<HTMLButtonElement>("#nearby-approval-cancel")!.addEventListener("click", async () => {
  const approval = nearbySnapshot.approval;
  try {
    if (approval?.failure_kind && !approval.retry_available) {
      if (approval.direction === "incoming") {
        nearbyDiscardDialog.showModal();
        return;
      }
      nearbySnapshot = nearbyFixtureEnabled ? { ...nearbySnapshot, approval: null, last_error: null }
        : await invoke<NearbyDiscoverySnapshot>("cancel_nearby_approval");
      renderNearbySnapshot();
      nearbyStatus.textContent = t.nearbyPrepareAgain;
      return;
    }
    if (approval?.transfer_active) {
      nearbySnapshot = nearbyFixtureEnabled
        ? { ...nearbySnapshot, approval: { ...approval, cancel_requested: true } }
        : await invoke<NearbyDiscoverySnapshot>("cancel_nearby_secure_transfer");
      showNearbyApproval();
      renderNearbySnapshot();
      return;
    }
    nearbySnapshot = nearbyFixtureEnabled ? { ...nearbySnapshot, approval: null }
      : await invoke<NearbyDiscoverySnapshot>("cancel_nearby_approval");
  } finally {
    if (!nearbySnapshot.approval?.transfer_active && !nearbySnapshot.approval?.cancel_requested) nearbyApprovalDialog.close();
  }
});
document.querySelector<HTMLButtonElement>("#nearby-discard-cancel")!.addEventListener("click", () => {
  nearbyDiscardDialog.close();
  showNearbyApproval();
});
document.querySelector<HTMLButtonElement>("#nearby-discard-confirm")!.addEventListener("click", async () => {
  const button = document.querySelector<HTMLButtonElement>("#nearby-discard-confirm")!;
  button.disabled = true;
  try {
    nearbySnapshot = nearbyFixtureEnabled ? { ...nearbySnapshot, approval: null, last_error: null }
      : await invoke<NearbyDiscoverySnapshot>("discard_nearby_partial");
    nearbyDiscardDialog.close();
    nearbyApprovalDialog.close();
    renderNearbySnapshot();
  } catch (error) {
    nearbyStatus.textContent = String(error);
    nearbyStatus.dataset.state = "error";
  } finally {
    button.disabled = false;
  }
});
document.querySelector<HTMLButtonElement>("#nearby-approval-confirm")!.addEventListener("click", async () => {
  const approval = nearbySnapshot.approval;
  if (!approval) return;
  const button = document.querySelector<HTMLButtonElement>("#nearby-approval-confirm")!;
  button.disabled = true;
  try {
    if (approval.retry_available) {
      nearbyStatus.textContent = t.nearbyReconnecting;
      if (nearbyFixtureEnabled) {
        nearbySnapshot = { ...nearbySnapshot, last_error: null,
          approval: { ...approval, retry_available: false, failure_kind: undefined, transfer_active: true } };
        showNearbyApproval();
        renderNearbySnapshot();
        return;
      }
      nearbySnapshot = await invoke<NearbyDiscoverySnapshot>("connect_nearby_transfer");
      showNearbyApproval();
      void runNearbySecureTransfer();
      return;
    }
    nearbySnapshot = nearbyFixtureEnabled
      ? { ...nearbySnapshot, last_error: nearbyFailureFixtureKind ? "Fixture transfer failure." : null,
          approval: { ...approval, local_approved: true, remote_approved: true,
            transfer_active: !nearbyFailureFixtureKind,
            retry_available: nearbyFailureFixtureKind === "disconnected" || nearbyFailureFixtureKind === "timeout",
            failure_kind: nearbyFailureFixtureKind,
            transferred_bytes: Math.round(approval.byte_length * 0.42) } }
      : await invoke<NearbyDiscoverySnapshot>("approve_nearby_transfer", { confirmationCode: approval.confirmation_code });
    showNearbyApproval();
    renderNearbySnapshot();
    if (!nearbyFixtureEnabled && approval.direction !== "incoming") {
      void invoke<NearbyDiscoverySnapshot>("connect_nearby_transfer").then((snapshot) => {
        nearbySnapshot = snapshot;
        showNearbyApproval();
        return runNearbySecureTransfer();
      }).catch((error) => {
        nearbyStatus.textContent = String(error);
        nearbyStatus.dataset.state = "error";
      });
    } else if (!nearbyFixtureEnabled && approval.direction === "incoming") {
      void runNearbySecureTransfer();
    }
  } catch (error) {
    nearbyStatus.textContent = String(error);
    nearbyStatus.dataset.state = "error";
    button.disabled = false;
  }
});
outputStatus.addEventListener("click", () => {
  renderOutputStatus();
  outputDialog.showModal();
});
document.querySelector<HTMLButtonElement>("#output-close")!.addEventListener("click", () => outputDialog.close());
outputDialog.addEventListener("click", (event) => {
  if (event.target === outputDialog) outputDialog.close();
});
document.querySelector<HTMLButtonElement>("#media-back")!.addEventListener("click", () => void closeMediaLibrary());
mediaRefresh.addEventListener("click", () => void loadMediaIndex());
document.querySelectorAll<HTMLButtonElement>("[data-media-filter]").forEach((button) => {
  button.addEventListener("click", () => {
    mediaFilter = button.dataset.mediaFilter as MediaFilter;
    document.querySelectorAll<HTMLButtonElement>("[data-media-filter]").forEach((item) => {
      const active = item === button;
      item.classList.toggle("is-active", active);
      item.setAttribute("aria-pressed", String(active));
    });
    renderMediaIndex();
    mediaGrid.scrollTop = 0;
  });
});
document.querySelectorAll<HTMLButtonElement>("[data-media-view]").forEach((button) => {
  button.addEventListener("click", () => {
    mediaView = button.dataset.mediaView as MediaView;
    document.querySelectorAll<HTMLButtonElement>("[data-media-view]").forEach((item) => {
      const active = item === button;
      item.classList.toggle("is-active", active);
      item.setAttribute("aria-pressed", String(active));
    });
    renderMediaIndex();
    mediaGrid.scrollTop = 0;
  });
});

mediaContextSave.addEventListener("click", async () => {
  const entry = contextMediaEntry;
  if (!entry || entry.state !== "finalized" || entry.media_type !== "photo") return;
  closeMediaContextMenu();
  mediaStatus.textContent = t.mediaSavingPhotos;
  mediaStatus.dataset.state = "loading";
  try {
    await invoke("export_photo_to_library", { id: entry.id });
    mediaStatus.textContent = t.mediaSavedPhotos;
    mediaStatus.dataset.state = "success";
  } catch (error) {
    mediaStatus.textContent = `${t.mediaSavePhotosFailed} ${String(error)}`;
    mediaStatus.dataset.state = "error";
  }
});
mediaContextDelete.addEventListener("click", () => {
  const entry = contextMediaEntry;
  if (!entry) return;
  selectedMediaEntry = entry;
  closeMediaContextMenu();
  document.querySelector<HTMLElement>("#media-delete-name")!.textContent = mediaFileName(entry.resource_path);
  mediaDeleteConfirm.dataset.state = "default";
  mediaDeleteDialog.showModal();
});
document.querySelector<HTMLButtonElement>("#media-delete-cancel")!.addEventListener("click", () => mediaDeleteDialog.close());
mediaDeleteDialog.addEventListener("click", (event) => {
  if (event.target === mediaDeleteDialog) mediaDeleteDialog.close();
});
mediaDeleteConfirm.addEventListener("click", async () => {
  if (!selectedMediaEntry) return;
  const entry = selectedMediaEntry;
  mediaDeleteConfirm.disabled = true;
  mediaDeleteConfirm.dataset.state = "loading";
  try {
    mediaEntries = entry.state === "finalized"
      ? await invoke<MediaIndexEntry[]>("delete_media_entry", { id: entry.id })
      : await invoke<MediaIndexEntry[]>("cleanup_media_entry", { id: entry.id });
    mediaEntries.sort((a, b) => b.updated_at_utc.localeCompare(a.updated_at_utc));
    mediaDeleteDialog.close();
    selectedMediaEntry = undefined;
    mediaStatus.textContent = "";
    mediaStatus.dataset.state = "default";
    renderMediaIndex();
  } catch (error) {
    mediaDeleteDialog.close();
    mediaStatus.textContent = `${t.mediaDeleteFailed} ${String(error)}`;
    mediaStatus.dataset.state = "error";
  } finally {
    mediaDeleteConfirm.disabled = false;
  }
});
document.addEventListener("pointerdown", (event) => {
  if (!mediaContextMenu.hidden && !mediaContextMenu.contains(event.target as Node)) closeMediaContextMenu();
});
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && !mediaContextMenu.hidden) closeMediaContextMenu();
});
window.addEventListener("resize", closeMediaContextMenu);
mediaLibrary.addEventListener("scroll", closeMediaContextMenu, { passive: true });

document.querySelector<HTMLButtonElement>("#media-detail-close")!.addEventListener("click", () => mediaDetailDialog.close());
mediaDetailDialog.addEventListener("click", (event) => {
  if (event.target === mediaDetailDialog) mediaDetailDialog.close();
});
mediaCleanup.addEventListener("click", () => {
  if (!selectedMediaEntry || selectedMediaEntry.state === "finalized") return;
  bulkCleanupRequested = false;
  document.querySelector<HTMLElement>("#media-cleanup-dialog p")!.textContent = t.mediaCleanupPrompt;
  document.querySelector<HTMLElement>("#media-cleanup-name")!.textContent = mediaFileName(selectedMediaEntry.resource_path);
  mediaCleanupDialog.showModal();
});
mediaCleanupExpired.addEventListener("click", () => {
  if (pendingBulkCleanupIds.length === 0) return;
  bulkCleanupRequested = true;
  document.querySelector<HTMLElement>("#media-cleanup-dialog p")!.textContent = t.mediaCleanupExpiredPrompt;
  document.querySelector<HTMLElement>("#media-cleanup-name")!.textContent = `${pendingBulkCleanupIds.length} ${t.mediaIncomplete} / ${t.mediaFailed}`;
  mediaCleanupDialog.showModal();
});
mediaReinspect.addEventListener("click", async () => {
  if (!selectedMediaEntry || selectedMediaEntry.state === "finalized") return;
  const id = selectedMediaEntry.id;
  mediaReinspect.disabled = true;
  mediaReinspect.dataset.state = "loading";
  mediaDetailDiagnostic.hidden = false;
  mediaDetailDiagnostic.textContent = t.mediaReinspecting;
  try {
    if (recoveryFixtureEnabled) {
      mediaEntries = recoveryFixtureEntries().map((entry) => ({
        ...entry,
        error: "reinspection failed structural media probe: movie container is incomplete",
        updated_at_utc: new Date().toISOString()
      }));
    } else {
      mediaEntries = await invoke<MediaIndexEntry[]>("reinspect_media_entry", { id });
    }
    mediaEntries.sort((a, b) => b.updated_at_utc.localeCompare(a.updated_at_utc));
    renderMediaIndex();
    const updated = mediaEntries.find((entry) => entry.id === id);
    if (!updated) throw new Error(`media record disappeared after reinspection: ${id}`);
    selectedMediaEntry = updated;
    mediaDetailDiagnostic.textContent = updated.error ?? t.mediaAwaiting;
    mediaReinspect.dataset.state = "success";
  } catch (error) {
    mediaReinspect.dataset.state = "error";
    mediaDetailDiagnostic.textContent = `${t.mediaReinspectFailed} ${String(error)}`;
  } finally {
    mediaReinspect.disabled = false;
  }
});
mediaRecapture.addEventListener("click", async () => {
  if (!selectedMediaEntry || selectedMediaEntry.state === "finalized") return;
  const nextMode: CameraMode = selectedMediaEntry.media_type === "video" ? "video" : "still";
  mediaDetailDialog.close();
  selectedMediaEntry = undefined;
  await closeMediaLibrary();
  await selectMode(nextMode);
});
document.querySelector<HTMLButtonElement>("#media-cleanup-cancel")!.addEventListener("click", () => mediaCleanupDialog.close());
mediaCleanupDialog.addEventListener("click", (event) => {
  if (event.target === mediaCleanupDialog) mediaCleanupDialog.close();
});
mediaCleanupConfirm.addEventListener("click", async () => {
  if (!bulkCleanupRequested && (!selectedMediaEntry || selectedMediaEntry.state === "finalized")) return;
  mediaCleanupConfirm.disabled = true;
  mediaCleanupConfirm.dataset.state = "loading";
  try {
    mediaEntries = bulkCleanupRequested
      ? await invoke<MediaIndexEntry[]>("cleanup_media_entries", { ids: pendingBulkCleanupIds })
      : await invoke<MediaIndexEntry[]>("cleanup_media_entry", { id: selectedMediaEntry!.id });
    mediaCleanupConfirm.dataset.state = "success";
    mediaCleanupDialog.close();
    mediaDetailDialog.close();
    selectedMediaEntry = undefined;
    pendingBulkCleanupIds = [];
    bulkCleanupRequested = false;
    mediaCleanupExpired.hidden = true;
    renderMediaIndex();
  } catch (error) {
    mediaCleanupConfirm.dataset.state = "error";
    mediaCleanupDialog.close();
    mediaDetailDiagnostic.hidden = false;
    mediaDetailDiagnostic.textContent = `${t.mediaCleanupFailed} ${String(error)}`;
  } finally {
    mediaCleanupConfirm.disabled = false;
  }
});

document.querySelectorAll<HTMLButtonElement>("[data-tool]").forEach((button) => {
  button.addEventListener("click", () => {
    const active = button.getAttribute("aria-pressed") !== "true";
    button.setAttribute("aria-pressed", String(active));
    button.classList.toggle("is-active", active);
    const tool = button.dataset.tool as Tool;
    document.body.classList.toggle(`tool-${tool}`, active);
    if (tool === "focus" || tool === "zebra") syncMonitorProcessing();
  });
});

document.querySelector<HTMLButtonElement>("#monitor-tools-toggle")!.addEventListener("click", (event) => {
  const toggle = event.currentTarget as HTMLButtonElement;
  const tools = document.querySelector<HTMLElement>(".monitor-tools")!;
  const rail = document.querySelector<HTMLElement>(".tool-rail")!;
  const open = toggle.getAttribute("aria-expanded") !== "true";
  if (open) {
    const destinationToggle = document.querySelector<HTMLButtonElement>("#destination-tools-toggle")!;
    destinationToggle.setAttribute("aria-expanded", "false");
    destinationToggle.classList.remove("is-active");
    document.querySelector<HTMLElement>(".destination-tools")!.classList.remove("is-open");
    rail.classList.remove("is-destination-open");
  }
  toggle.setAttribute("aria-expanded", String(open));
  toggle.classList.toggle("is-active", open);
  tools.classList.toggle("is-open", open);
  rail.classList.toggle("is-menu-open", open);
  syncNativePreviewFrame();
});

document.querySelector<HTMLButtonElement>("#destination-tools-toggle")!.addEventListener("click", (event) => {
  const toggle = event.currentTarget as HTMLButtonElement;
  const destinations = document.querySelector<HTMLElement>(".destination-tools")!;
  const rail = document.querySelector<HTMLElement>(".tool-rail")!;
  const open = toggle.getAttribute("aria-expanded") !== "true";
  if (open) {
    const monitorToggle = document.querySelector<HTMLButtonElement>("#monitor-tools-toggle")!;
    monitorToggle.setAttribute("aria-expanded", "false");
    monitorToggle.classList.remove("is-active");
    document.querySelector<HTMLElement>(".monitor-tools")!.classList.remove("is-open");
    rail.classList.remove("is-menu-open");
  }
  toggle.setAttribute("aria-expanded", String(open));
  toggle.classList.toggle("is-active", open);
  destinations.classList.toggle("is-open", open);
  rail.classList.toggle("is-destination-open", open);
  syncNativePreviewFrame();
});

formatResolution.addEventListener("change", () => populateFrameRates(recordingFrameRate));
document.querySelector<HTMLButtonElement>("#format-close")!.addEventListener("click", () => {
  formatPanel.hidden = true;
  formatStatus.textContent = "";
});
formatApply.addEventListener("click", async () => {
  const format = selectedFormatCapability();
  const fps = Number(formatFps.value);
  if (!format || !Number.isFinite(fps) || recording) return;
  formatApply.disabled = true;
  formatApply.dataset.state = "loading";
  formatStatus.textContent = "";
  try {
    const active = await invoke<PreviewFormat>("apply_camera_format", {
      width: format.width,
      height: format.height,
      fps
    });
    applyActiveFormat(active);
    if (active.settings_persisted) {
      formatPanel.hidden = true;
    } else {
      formatStatus.textContent = `${t.formatPersistenceFailed}: ${active.settings_warning ?? "unknown"}`;
    }
    window.requestAnimationFrame(syncNativePreviewFrame);
  } catch (error) {
    formatApply.dataset.state = "error";
    formatStatus.textContent = `${t.formatFailed}: ${String(error)}`;
  } finally {
    formatApply.disabled = false;
    if (formatApply.dataset.state !== "error") formatApply.dataset.state = "default";
  }
});

document.querySelectorAll<HTMLButtonElement>("[data-parameter]").forEach((button) => {
  button.addEventListener("click", () => {
    if (button.dataset.parameter === "fps") {
      if (!recording && currentCapabilities?.formats.length) {
        formatPanel.hidden = !formatPanel.hidden;
        formatStatus.textContent = "";
        window.requestAnimationFrame(syncNativePreviewFrame);
      }
      return;
    }
    const parameter = button.dataset.parameter as typeof adjustmentParameter;
    if (!parameter) return;
    adjustmentParameter = parameter;
    const select = document.querySelector<HTMLSelectElement>("#adjust-select")!;
    const setOptions = (entries: { value: string; label: string }[], selected: string) => {
      select.replaceChildren(...entries.map((entry) => {
        const option = document.createElement("option");
        option.value = entry.value;
        option.textContent = entry.label;
        option.selected = entry.value === selected;
        return option;
      }));
    };
    if (parameter === "lens") {
      setOptions(availableDevices.map((device) => ({ value: device.id, label: device.label })), activeDeviceId ?? availableDevices[0]?.id ?? "");
    } else if (parameter === "iris") {
      const aperture = currentCapabilities?.lens_aperture;
      const label = aperture ? `ƒ/${aperture.toFixed(1)} · FIXED` : "FIXED";
      setOptions([{ value: "fixed", label }], "fixed");
    } else if (parameter === "shutter") {
      const stops = [24, 25, 30, 40, 48, 50, 60, 80, 96, 100, 120, 125, 160, 200, 240, 250, 320, 400, 480, 500, 640, 800, 1000];
      const current = Math.max(1, Math.round(1 / (currentCapabilities?.current_shutter_seconds ?? (1 / 48))));
      const selected = stops.reduce((best, value) => Math.abs(value - current) < Math.abs(best - current) ? value : best);
      setOptions(stops.map((value) => ({ value: String(value), label: `1/${value}` })), String(selected));
    } else if (parameter === "ei") {
      const [minimum, maximum] = currentCapabilities?.manual_iso ?? [32, 3200];
      const current = currentCapabilities?.current_iso ?? 400;
      const stops = [25, 32, 40, 50, 64, 80, 100, 125, 160, 200, 250, 320, 400, 500, 640, 800, 1000, 1250, 1600, 2000, 2500, 3200, 4000, 5000, 6400, 8000, 10000, 12800]
        .filter((value) => value >= minimum && value <= maximum);
      if (stops.length === 0) stops.push(Math.round(Math.max(minimum, Math.min(maximum, current))));
      const selected = stops.reduce((best, value) => Math.abs(value - current) < Math.abs(best - current) ? value : best);
      setOptions(stops.map((value) => ({ value: String(value), label: `EI ${value}` })), String(selected));
    } else {
      const stops = [2000, 2400, 2800, 3200, 3600, 4000, 4300, 4800, 5200, 5600, 6000, 6500, 7000, 8000, 9000, 10000];
      const current = currentCapabilities?.current_white_balance_kelvin ?? 5600;
      const selected = stops.reduce((best, value) => Math.abs(value - current) < Math.abs(best - current) ? value : best);
      setOptions(stops.map((value) => ({ value: String(value), label: `${value}K` })), String(selected));
    }
    document.querySelectorAll<HTMLButtonElement>("[data-parameter]").forEach((item) => {
      item.classList.remove("is-selected");
      item.setAttribute("aria-pressed", "false");
    });
    button.classList.add("is-selected");
    button.setAttribute("aria-pressed", "true");
    document.querySelector<HTMLElement>("#adjust-label")!.textContent = button.querySelector("span")!.textContent;
    document.querySelector<HTMLElement>("#adjust-value")!.textContent = button.querySelector("strong")!.textContent;
    document.querySelector<HTMLElement>("#quick-adjust")!.hidden = false;
  });
});

document.querySelector<HTMLButtonElement>("#adjust-close")!.addEventListener("click", () => {
  document.querySelector<HTMLElement>("#quick-adjust")!.hidden = true;
  adjustmentParameter = undefined;
});

document.querySelector<HTMLSelectElement>("#adjust-select")!.addEventListener("change", async (event) => {
  const select = event.currentTarget as HTMLSelectElement;
  if (!adjustmentParameter) return;
  const value = Number(select.value);
  select.disabled = true;
  try {
    if (adjustmentParameter === "lens") {
      if (controlFixtureEnabled) {
        document.querySelector<HTMLElement>('[data-parameter="lens"] strong')!.textContent = select.selectedOptions[0]?.textContent ?? "LENS";
      } else {
        await startNativePreview({ authorization: "authorized", devices: availableDevices }, select.value);
      }
    } else if (adjustmentParameter === "shutter" && Number.isFinite(value)) {
      const seconds = await invoke<number>("set_camera_shutter", { seconds: 1 / value });
      if (currentCapabilities) currentCapabilities.current_shutter_seconds = seconds;
      document.querySelector<HTMLElement>('[data-parameter="shutter"] strong')!.textContent = `1/${Math.round(1 / seconds)}`;
    } else if (adjustmentParameter === "ei" && Number.isFinite(value)) {
      const iso = await invoke<number>("set_camera_iso", { iso: value });
      if (currentCapabilities) currentCapabilities.current_iso = iso;
      document.querySelector<HTMLElement>('[data-parameter="ei"] strong')!.textContent = String(Math.round(iso));
    } else if (adjustmentParameter === "wb" && Number.isFinite(value)) {
      const kelvin = await invoke<number>("set_camera_white_balance", { kelvin: value });
      if (currentCapabilities) currentCapabilities.current_white_balance_kelvin = kelvin;
      document.querySelector<HTMLElement>('[data-parameter="wb"] strong')!.textContent = `${Math.round(kelvin)}K`;
    }
  } catch (error) {
    document.querySelector<HTMLElement>("#adjust-value")!.textContent = String(error);
  } finally {
    select.disabled = false;
  }
});

captureButton.addEventListener("click", async () => {
  if (!nativePreviewRunning || !storageAllowsCapture()) return;
  if (mode === "video") {
    captureButton.disabled = true;
    captureButton.dataset.state = "loading";
    try {
      if (!recording) {
        await invoke("start_video_recording");
        recording = true;
        recordingStartedAt = performance.now();
        timerId = window.setInterval(updateRecordingUI, 1000 / recordingFrameRate);
        storageMonitorId = window.setInterval(() => void monitorRecordingStorage(), 2000);
        captureButton.dataset.state = "default";
        updateRecordingUI();
        captureButton.disabled = false;
        return;
      }
      await finishVideoRecording();
      return;
    } catch (error) {
      if (recording) stopRecording();
      captureButton.dataset.state = "error";
      feedback.textContent = `${t.recordingFailed}: ${String(error)}`;
      captureButton.setAttribute("aria-label", feedback.textContent);
    }
    feedback.classList.add("is-visible");
    window.setTimeout(() => {
      captureButton.dataset.state = "default";
      captureButton.disabled = !nativePreviewRunning
        || microphoneAuthorization !== "authorized"
        || !storageAllowsCapture();
      feedback.classList.remove("is-visible");
    }, 1800);
    return;
  }
  captureButton.disabled = true;
  captureButton.dataset.state = "loading";
  try {
    playShutterSound();
    const asset = await invoke<CaptureAsset>("capture_photo", { suppressShutterSound: shutterSound.value === "silent" });
    captureButton.dataset.state = "success";
    const path = asset.original.path;
    const warning = asset.validation.status === "warning" ? ` · ${t.assetMetadataWarning}` : "";
    feedback.textContent = `${t.captured} · ${path.split("/").pop() ?? path}${warning}`;
    void refreshOutputStatus();
  } catch (error) {
    captureButton.dataset.state = "error";
    feedback.textContent = `${t.captureFailed}: ${String(error)}`;
    captureButton.setAttribute("aria-label", feedback.textContent);
  }
  feedback.classList.add("is-visible");
  window.setTimeout(() => {
    captureButton.dataset.state = "default";
    captureButton.disabled = !nativePreviewRunning || mode === "video" || !storageAllowsCapture();
    feedback.classList.remove("is-visible");
  }, 1600);
});

if (!nearbyFixtureEnabled) void refreshOutputStatus();
if (!nearbyFixtureEnabled) void refreshCleanupCandidateCount().catch(() => {
  mediaCleanupExpired.hidden = true;
});
