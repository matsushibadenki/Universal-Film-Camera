import { invoke } from "@tauri-apps/api/core";
import "./style.css";

type CameraMode = "still" | "video";
type Locale = "en" | "ja" | "zh-CN";
type Tool = "focus" | "zebra" | "guides" | "scope";
type CameraAuthorization = "not_determined" | "restricted" | "denied" | "authorized" | "unavailable";
type CameraDevice = { id: string; label: string; position: "front" | "back" | "external" | "unspecified" };
type CameraDiscovery = { authorization: CameraAuthorization; devices: CameraDevice[] };
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
type MediaFilter = "all" | MediaState;
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
    pipeline: "Pipeline", media: "Media", settings: "Settings", focus: "Focus assist",
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
    mediaPhoto: "Photo", mediaVideo: "Video", mediaDuration: "Duration", mediaAwaiting: "Awaiting validation",
    mediaValidationFailed: "Validation failed", mediaDetails: "View details", mediaPath: "Resource path",
    mediaUpdated: "Updated", mediaState: "State", mediaCleanup: "Clean up recoverable file",
    mediaCleanupTitle: "Remove recoverable media?", mediaCleanupPrompt: "This permanently removes the incomplete or failed resource and its diagnostic manifest.",
    mediaCleanupConfirm: "Remove file", mediaCleanupCancel: "Keep file", mediaCleanupFailed: "The recoverable media could not be removed.",
    mediaReinspect: "Reinspect file", mediaReinspecting: "Reinspecting media…", mediaReinspectFailed: "The media could not be reinspected.",
    mediaRecapture: "Recapture"
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
    pipeline: "パイプライン", media: "メディア", settings: "設定", focus: "フォーカス",
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
    mediaPhoto: "写真", mediaVideo: "動画", mediaDuration: "長さ", mediaAwaiting: "検証待ち",
    mediaValidationFailed: "検証失敗", mediaDetails: "詳細を表示", mediaPath: "リソースパス",
    mediaUpdated: "更新日時", mediaState: "状態", mediaCleanup: "復旧対象ファイルを削除",
    mediaCleanupTitle: "復旧対象メディアを削除しますか？", mediaCleanupPrompt: "未完了または失敗したリソースと診断マニフェストを完全に削除します。",
    mediaCleanupConfirm: "ファイルを削除", mediaCleanupCancel: "ファイルを残す", mediaCleanupFailed: "復旧対象メディアを削除できませんでした。",
    mediaReinspect: "ファイルを再検査", mediaReinspecting: "メディアを再検査しています…", mediaReinspectFailed: "メディアを再検査できませんでした。",
    mediaRecapture: "再撮影"
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
    pipeline: "成像管线", media: "媒体", settings: "设置", focus: "对焦",
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
    mediaPhoto: "照片", mediaVideo: "视频", mediaDuration: "时长", mediaAwaiting: "等待验证",
    mediaValidationFailed: "验证失败", mediaDetails: "查看详情", mediaPath: "资源路径",
    mediaUpdated: "更新时间", mediaState: "状态", mediaCleanup: "清理可恢复文件",
    mediaCleanupTitle: "删除可恢复媒体？", mediaCleanupPrompt: "这将永久删除未完成或失败的资源及其诊断清单。",
    mediaCleanupConfirm: "删除文件", mediaCleanupCancel: "保留文件", mediaCleanupFailed: "无法删除可恢复媒体。",
    mediaReinspect: "重新检查文件", mediaReinspecting: "正在重新检查媒体…", mediaReinspectFailed: "无法重新检查媒体。",
    mediaRecapture: "重新拍摄"
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
    settings: '<circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M19 5l-2 2M7 17l-2 2"/>',
    nearby: '<path d="M5 8.5a10 10 0 0 1 14 0M8 12a6 6 0 0 1 8 0M11 15.5a2 2 0 0 1 2 0"/><circle cx="12" cy="19" r="1"/>',
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
        <div class="brand-lockup"><span class="brand-mark">UF</span><span>${t.camera}</span></div>
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
      </div>

      <div class="preview-surface">
        <div class="safe-frame" aria-hidden="true"><i></i><i></i><i></i><i></i></div>
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
            <path class="hist-red" d="M2 41 16 37 27 39 38 25 49 35 61 19 71 38 86 32 98 40 112 28 130 41Z"/>
            <path class="hist-green" d="M2 41 15 40 29 31 40 38 55 16 68 26 81 17 94 36 108 33 119 39 130 41Z"/>
            <path class="hist-blue" d="M2 41 18 36 33 18 45 30 58 8 71 25 84 34 97 21 112 37 130 41Z"/>
          </svg>
          <span>HIST</span>
        </section>

        <section class="audio-meter" aria-label="Audio meters">
          <div><span>1</span><i style="--level: 68%"></i></div>
          <div><span>2</span><i style="--level: 54%"></i></div>
          <small>−48&nbsp;&nbsp;−24&nbsp;&nbsp;−12&nbsp;&nbsp;0</small>
        </section>

        <section class="quick-adjust" id="quick-adjust" hidden aria-live="polite">
          <div><span id="adjust-label">EI</span><strong id="adjust-value">400</strong></div>
          <input id="adjust-range" type="range" min="0" max="100" value="50" aria-label="${t.adjust}" />
          <button id="adjust-close" aria-label="${t.close}">${icon("close")}</button>
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
          <button id="media-refresh" type="button" aria-label="${t.mediaRefresh}">${icon("media")}<span>${t.mediaRefresh}</span></button>
          <button id="media-back" type="button" aria-label="${t.mediaBack}">${icon("close")}<span>${t.mediaBack}</span></button>
        </div>
      </header>

      <nav class="media-filters" aria-label="${t.mediaTitle}">
        <button class="is-active" data-media-filter="all" aria-pressed="true"><span>${t.mediaAll}</span><strong data-media-count="all">0</strong></button>
        <button data-media-filter="finalized" aria-pressed="false"><span>${t.mediaReady}</span><strong data-media-count="finalized">0</strong></button>
        <button data-media-filter="incomplete" aria-pressed="false"><span>${t.mediaIncomplete}</span><strong data-media-count="incomplete">0</strong></button>
        <button data-media-filter="failed" aria-pressed="false"><span>${t.mediaFailed}</span><strong data-media-count="failed">0</strong></button>
      </nav>

      <div class="media-status" id="media-status" role="status" aria-live="polite"></div>
      <div class="media-grid" id="media-grid"></div>
      <div class="media-empty" id="media-empty" hidden>
        ${icon("media")}
        <strong>${t.mediaEmpty}</strong>
        <p>${t.mediaEmptyDetail}</p>
      </div>
    </section>

    <section class="nearby-library" id="nearby-library" aria-labelledby="nearby-title" hidden>
      <header class="media-header">
        <div>
          <h1 id="nearby-title">${t.nearbyTitle}</h1>
          <p>${t.nearbySubtitle}</p>
        </div>
        <div class="media-header-actions">
          <button id="nearby-refresh" type="button" aria-label="${t.nearbyRefresh}">${icon("nearby")}<span>${t.nearbyRefresh}</span></button>
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
        <button class="output-status" id="output-status" type="button" aria-label="${t.output}"><span>${t.output} · —</span></button>
        <button class="monitor-tools-toggle" id="monitor-tools-toggle" aria-expanded="false" aria-controls="monitor-tools-panel" aria-label="${t.scopes}">${icon("scope")}<span>${t.scopes}</span></button>

        <nav class="destination-tools" aria-label="Application sections">
          <button class="is-active" aria-label="${t.pipeline}">${icon("pipeline")}<span>${t.pipeline}</span></button>
          <button id="open-media" aria-label="${t.media}" aria-controls="media-library" aria-pressed="false">${icon("media")}<span>${t.media}</span></button>
          <button id="open-nearby" aria-label="${t.nearby}" aria-controls="nearby-library" aria-pressed="false">${icon("nearby")}<span>${t.nearby}</span></button>
          <button aria-label="${t.settings}">${icon("settings")}<span>${t.settings}</span></button>
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
const nearbyLibrary = document.querySelector<HTMLElement>("#nearby-library")!;
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
const outputStatus = document.querySelector<HTMLButtonElement>("#output-status")!;
const outputDialog = document.querySelector<HTMLDialogElement>("#output-dialog")!;
let nativePreviewRunning = false;
let nativePreviewStarting = false;
let activeDeviceId: string | undefined;
let activeDevicePosition: CameraDevice["position"] | undefined;
let lastOrientationKey: string | undefined;
let mediaEntries: MediaIndexEntry[] = [];
let mediaFilter: MediaFilter = "all";
let selectedMediaEntry: MediaIndexEntry | undefined;
let nearbySnapshot: NearbyDiscoverySnapshot = { active: false, local_peer: null, peers: [], last_error: null };
let nearbyPollId: number | undefined;
let selectedNearbyPeerId: string | undefined;
const devQuery = new URLSearchParams(window.location.search);
const recoveryFixtureEnabled = import.meta.env.DEV && devQuery.get("recovery-fixture") === "1";
const storageLowFixtureEnabled = import.meta.env.DEV && devQuery.get("storage-low") === "1";
const nearbyFixtureEnabled = import.meta.env.DEV && devQuery.get("nearby-fixture") === "1";
const nearbyRetryFixtureEnabled = nearbyFixtureEnabled && devQuery.get("nearby-retry") === "1";
const nearbyFailureFixtureValue = devQuery.get("nearby-failure");
const nearbyFailureFixtureKind = nearbyFixtureEnabled && ["disconnected", "timeout", "integrity", "storage", "invitation_expired", "cancelled", "protocol"].includes(nearbyFailureFixtureValue ?? "")
  ? nearbyFailureFixtureValue as NonNullable<NonNullable<NearbyDiscoverySnapshot["approval"]>["failure_kind"]>
  : nearbyRetryFixtureEnabled ? "disconnected" : undefined;
const nearbyIncomingFixtureEnabled = nearbyFixtureEnabled && devQuery.get("nearby-incoming") === "1";

function recoveryFixtureEntries(): MediaIndexEntry[] {
  return [{
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

function renderMediaCard(entry: MediaIndexEntry): HTMLElement {
  const card = document.createElement("article");
  card.className = `media-card is-${entry.state}`;
  card.dataset.mediaId = entry.id;

  const visual = document.createElement("div");
  visual.className = "media-card-visual";
  visual.innerHTML = icon(entry.media_type === "photo" ? "photo" : "video");
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
  mediaGrid.replaceChildren(...filtered.map(renderMediaCard));
  mediaEmpty.hidden = filtered.length !== 0;
  document.querySelectorAll<HTMLElement>("[data-media-count]").forEach((count) => {
    const state = count.dataset.mediaCount as MediaFilter;
    count.textContent = String(state === "all" ? mediaEntries.length : mediaEntries.filter((entry) => entry.state === state).length);
  });
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
  } finally {
    mediaLibrary.removeAttribute("aria-busy");
    mediaRefresh.disabled = false;
  }
}

async function openMediaLibrary(): Promise<void> {
  if (recording || !mediaLibrary.hidden) return;
  mediaButton.disabled = true;
  mediaButton.dataset.state = "loading";
  if (nativePreviewRunning) {
    try {
      await invoke("stop_camera_preview");
      nativePreviewRunning = false;
      document.body.classList.remove("has-native-preview");
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
  nearbyButton.disabled = true;
  if (nativePreviewRunning) {
    try {
      await invoke("stop_camera_preview");
      nativePreviewRunning = false;
      document.body.classList.remove("has-native-preview");
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
  const angle = screen.orientation?.angle ?? 0;
  const normalized = ((Math.round(angle / 90) * 90) % 360 + 360) % 360 as 0 | 90 | 180 | 270;
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
  if (width === 3840 && height === 2160) return "UHD";
  if (width === 4096 && height === 2160) return "DCI 4K";
  if (height === 2160) return "4K";
  if (height === 1080) return "1080p";
  if (height === 720) return "720p";
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
    option.textContent = `${fps} FPS`;
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
    option.textContent = `${resolutionLabel(format.width, format.height)} · ${format.width}×${format.height}`;
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
  lens.disabled = true;
  fpsControl.disabled = capabilities.formats.length === 0;
  iris.disabled = true;
  wb.disabled = true;
  lens.querySelector("strong")!.textContent = "—";
  iris.querySelector("strong")!.textContent = "—";
  wb.querySelector("strong")!.textContent = "AUTO";
  shutter.disabled = !capabilities.manual_shutter;
  ei.disabled = capabilities.manual_iso === null;
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

async function startNativePreview(result: CameraDiscovery): Promise<void> {
  const device = result.authorization === "authorized" ? result.devices[0] : undefined;
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
    document.body.classList.toggle("has-native-preview", status.running);
    captureButton.disabled = !status.running || mode === "video" || !storageAllowsCapture();
  } catch (error) {
    signalTitle.textContent = t.previewFailed;
    signalMessage.textContent = String(error);
  } finally {
    nativePreviewStarting = false;
  }
}

function renderDiscovery(result: CameraDiscovery): void {
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
screen.orientation?.addEventListener("change", () => void syncNativeOrientation());
window.addEventListener("orientationchange", () => void syncNativeOrientation());

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
    if (recording) recordingPausedByLifecycle = true;
    return;
  }
  if (!recordingPausedByLifecycle || !recording) return;
  recordingPausedByLifecycle = false;
  void finishVideoRecording().finally(() => {
    nativePreviewRunning = false;
    activeDeviceId = undefined;
    void refreshCameraDiscovery();
  });
});

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
  });
});

document.querySelector<HTMLButtonElement>("#media-detail-close")!.addEventListener("click", () => mediaDetailDialog.close());
mediaDetailDialog.addEventListener("click", (event) => {
  if (event.target === mediaDetailDialog) mediaDetailDialog.close();
});
mediaCleanup.addEventListener("click", () => {
  if (!selectedMediaEntry || selectedMediaEntry.state === "finalized") return;
  document.querySelector<HTMLElement>("#media-cleanup-name")!.textContent = mediaFileName(selectedMediaEntry.resource_path);
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
  if (!selectedMediaEntry || selectedMediaEntry.state === "finalized") return;
  mediaCleanupConfirm.disabled = true;
  mediaCleanupConfirm.dataset.state = "loading";
  try {
    mediaEntries = await invoke<MediaIndexEntry[]>("cleanup_media_entry", { id: selectedMediaEntry.id });
    mediaCleanupConfirm.dataset.state = "success";
    mediaCleanupDialog.close();
    mediaDetailDialog.close();
    selectedMediaEntry = undefined;
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
  });
});

document.querySelector<HTMLButtonElement>("#monitor-tools-toggle")!.addEventListener("click", (event) => {
  const toggle = event.currentTarget as HTMLButtonElement;
  const tools = document.querySelector<HTMLElement>(".monitor-tools")!;
  const rail = document.querySelector<HTMLElement>(".tool-rail")!;
  const open = toggle.getAttribute("aria-expanded") !== "true";
  toggle.setAttribute("aria-expanded", String(open));
  toggle.classList.toggle("is-active", open);
  tools.classList.toggle("is-open", open);
  rail.classList.toggle("is-menu-open", open);
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
    const asset = await invoke<CaptureAsset>("capture_photo");
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
