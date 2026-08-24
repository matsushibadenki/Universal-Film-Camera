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
type CaptureAsset = {
  schema_version: number;
  id: string;
  media_type: "photo" | "video";
  state: "incomplete" | "finalized" | "failed";
  original: {
    path: string;
    pixel_width: number;
    pixel_height: number;
    frame_rate: { numerator: number; denominator: number } | null;
    duration_ms: number | null;
  };
  validation: { status: "passed" | "warning" | "failed" };
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
    assetMetadataWarning: "saved with a metadata warning", orientationFailed: "Camera orientation sync failed"
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
    assetMetadataWarning: "メタデータ警告付きで保存しました", orientationFailed: "カメラ姿勢の同期に失敗しました"
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
    assetMetadataWarning: "已保存，但存在元数据警告", orientationFailed: "相机方向同步失败"
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
    close: '<path d="m6 6 12 12M18 6 6 18"/>'
  };
  return `<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">${paths[name]}</svg>`;
}

const t = copy[locale()];
let mode: CameraMode = "still";
let recording = false;
let microphoneAuthorization: CameraAuthorization = "not_determined";
let recordingStartedAt = 0;
let timerId: number | undefined;
let recordingFrameRate = 24;
let currentCapabilities: CameraCapabilities | undefined;

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
        <button class="monitor-tools-toggle" id="monitor-tools-toggle" aria-expanded="false" aria-controls="monitor-tools-panel" aria-label="${t.scopes}">${icon("scope")}<span>${t.scopes}</span></button>

        <nav class="destination-tools" aria-label="Application sections">
          <button class="is-active" aria-label="${t.pipeline}">${icon("pipeline")}<span>${t.pipeline}</span></button>
          <button aria-label="${t.media}">${icon("media")}<span>${t.media}</span></button>
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
let nativePreviewRunning = false;
let nativePreviewStarting = false;
let activeDeviceId: string | undefined;
let activeDevicePosition: CameraDevice["position"] | undefined;
let lastOrientationKey: string | undefined;

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
    captureButton.disabled = !status.running || mode === "video";
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

void refreshCameraDiscovery();

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
  if (!nativePreviewRunning) return;
  if (recording) {
    void invoke("stop_video_recording").finally(() => invoke("stop_camera_preview"));
  } else {
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
  updateRecordingUI();
  void syncNativeOrientation();
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
  captureButton.disabled = !nativePreviewRunning || (mode === "video" && microphoneAuthorization !== "authorized");
}

document.querySelectorAll<HTMLButtonElement>("[data-mode]").forEach((button) => {
  button.addEventListener("click", () => void selectMode(button.dataset.mode as CameraMode));
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
  if (!nativePreviewRunning) return;
  if (mode === "video") {
    captureButton.disabled = true;
    captureButton.dataset.state = "loading";
    try {
      if (!recording) {
        await invoke("start_video_recording");
        recording = true;
        recordingStartedAt = performance.now();
        timerId = window.setInterval(updateRecordingUI, 1000 / recordingFrameRate);
        captureButton.dataset.state = "default";
        updateRecordingUI();
        captureButton.disabled = false;
        return;
      }
      const asset = await invoke<CaptureAsset>("stop_video_recording");
      stopRecording();
      captureButton.dataset.state = "success";
      const path = asset.original.path;
      const warning = asset.validation.status === "warning" ? ` · ${t.assetMetadataWarning}` : "";
      feedback.textContent = `${t.videoSaved} · ${path.split("/").pop() ?? path}${warning}`;
    } catch (error) {
      if (recording) stopRecording();
      captureButton.dataset.state = "error";
      feedback.textContent = `${t.recordingFailed}: ${String(error)}`;
      captureButton.setAttribute("aria-label", feedback.textContent);
    }
    feedback.classList.add("is-visible");
    window.setTimeout(() => {
      captureButton.dataset.state = "default";
      captureButton.disabled = !nativePreviewRunning || microphoneAuthorization !== "authorized";
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
  } catch (error) {
    captureButton.dataset.state = "error";
    feedback.textContent = `${t.captureFailed}: ${String(error)}`;
    captureButton.setAttribute("aria-label", feedback.textContent);
  }
  feedback.classList.add("is-visible");
  window.setTimeout(() => {
    captureButton.dataset.state = "default";
    captureButton.disabled = !nativePreviewRunning || mode === "video";
    feedback.classList.remove("is-visible");
  }, 1600);
});
