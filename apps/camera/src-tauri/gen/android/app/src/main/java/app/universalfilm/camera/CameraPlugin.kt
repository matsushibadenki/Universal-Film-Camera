package app.universalfilm.camera

import android.Manifest
import android.app.Activity
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CaptureRequest
import android.os.Handler
import android.os.Looper
import android.util.Range
import android.util.Size
import android.view.ViewGroup
import android.webkit.WebView
import android.widget.FrameLayout
import androidx.appcompat.app.AppCompatActivity
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageCapture
import androidx.camera.core.ImageCaptureException
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.camera2.interop.Camera2Interop
import androidx.camera.camera2.interop.ExperimentalCamera2Interop
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.video.FileOutputOptions
import androidx.camera.video.Recorder
import androidx.camera.video.Recording
import androidx.camera.video.VideoCapture
import androidx.camera.video.VideoRecordEvent
import androidx.camera.view.PreviewView
import androidx.core.content.ContextCompat
import app.tauri.PermissionState
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.Permission
import app.tauri.annotation.PermissionCallback
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSArray
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class PreviewArgs {
  var deviceId: String = "back"
  var viewport: ViewportArgs = ViewportArgs()
}

@InvokeArg
class ViewportArgs {
  var x: Double = 0.0
  var y: Double = 0.0
  var width: Double = 0.0
  var height: Double = 0.0
}

@InvokeArg
class DeviceArgs {
  var deviceId: String = "back"
}

@InvokeArg
class OrientationArgs {
  var rotationDegrees: Int = 0
  var previewMirrored: Boolean = false
  var captureMirrored: Boolean = false
}

@InvokeArg
class OutputArgs {
  var path: String = ""
}

@InvokeArg
class VideoOutputArgs {
  var path: String = ""
  var audioEnabled: Boolean = true
  var minimumAvailableBytes: Long = 0
}

@InvokeArg
class FormatArgs {
  var width: Int = 0
  var height: Int = 0
  var fps: Int = 0
}

@TauriPlugin(
  permissions = [
    Permission(strings = [Manifest.permission.CAMERA], alias = "camera"),
    Permission(strings = [Manifest.permission.RECORD_AUDIO], alias = "microphone")
  ]
)
class CameraPlugin(private val activity: Activity) : Plugin(activity) {
  private var webView: WebView? = null
  private var previewView: PreviewView? = null
  private var cameraProvider: ProcessCameraProvider? = null
  private var imageCapture: ImageCapture? = null
  private var videoCapture: VideoCapture<Recorder>? = null
  private var recording: Recording? = null
  private var recordingPath: String? = null
  private var stopRecordingInvoke: Invoke? = null
  private var finalizedRecording: JSObject? = null
  private var finalizedRecordingError: String? = null
  private val storageHandler = Handler(Looper.getMainLooper())
  private var storageMonitor: Runnable? = null
  private var storageStopRequested = false
  private var lifecycleStopPending = false
  private var activeDeviceId: String = "back"
  private var requestedFormat: FormatArgs? = null

  override fun load(webView: WebView) {
    this.webView = webView
  }

  private fun authorization(): String = when (getPermissionState("camera")) {
    PermissionState.GRANTED -> "authorized"
    PermissionState.DENIED -> "denied"
    PermissionState.PROMPT, PermissionState.PROMPT_WITH_RATIONALE -> "not_determined"
    null -> "unavailable"
  }

  private fun microphoneAuthorization(): String = when (getPermissionState("microphone")) {
    PermissionState.GRANTED -> "authorized"
    PermissionState.DENIED -> "denied"
    PermissionState.PROMPT, PermissionState.PROMPT_WITH_RATIONALE -> "not_determined"
    null -> "unavailable"
  }

  private fun discoveryResponse(): JSObject {
    val devices = JSArray()
    if (authorization() == "authorized") {
      val manager = activity.getSystemService(CameraManager::class.java)
      val seen = mutableSetOf<String>()
      manager.cameraIdList.forEach { cameraId ->
        val characteristics = manager.getCameraCharacteristics(cameraId)
        val position = when (characteristics.get(CameraCharacteristics.LENS_FACING)) {
          CameraCharacteristics.LENS_FACING_FRONT -> "front"
          CameraCharacteristics.LENS_FACING_BACK -> "back"
          CameraCharacteristics.LENS_FACING_EXTERNAL -> "external"
          else -> "unspecified"
        }
        val logicalId = when (position) {
          "front", "back" -> position
          else -> cameraId
        }
        if (!seen.add(logicalId)) return@forEach
        devices.put(JSObject().apply {
          put("id", logicalId)
          put("label", "Android ${position.replaceFirstChar { it.uppercase() }} Camera")
          put("position", position)
        })
      }
    }
    return JSObject().apply {
      put("authorization", authorization())
      put("devices", devices)
    }
  }

  @Command
  fun discovery(invoke: Invoke) {
    invoke.resolve(discoveryResponse())
  }

  @Command
  fun requestAuthorization(invoke: Invoke) {
    if (getPermissionState("camera") == PermissionState.GRANTED) {
      invoke.resolve(discoveryResponse())
    } else {
      requestPermissionForAlias("camera", invoke, "cameraPermissionResult")
    }
  }

  @PermissionCallback
  fun cameraPermissionResult(invoke: Invoke) {
    invoke.resolve(discoveryResponse())
  }

  @Command
  fun getMicrophoneAuthorization(invoke: Invoke) {
    invoke.resolve(JSObject().apply { put("authorization", microphoneAuthorization()) })
  }

  @Command
  fun requestMicrophoneAuthorization(invoke: Invoke) {
    if (getPermissionState("microphone") == PermissionState.GRANTED) {
      invoke.resolve(JSObject().apply { put("authorization", microphoneAuthorization()) })
    } else {
      requestPermissionForAlias("microphone", invoke, "microphonePermissionResult")
    }
  }

  @PermissionCallback
  fun microphonePermissionResult(invoke: Invoke) {
    invoke.resolve(JSObject().apply { put("authorization", microphoneAuthorization()) })
  }

  @Command
  fun capabilities(invoke: Invoke) {
    val args = invoke.parseArgs(DeviceArgs::class.java)
    val manager = activity.getSystemService(CameraManager::class.java)
    val facing = if (args.deviceId == "front") {
      CameraCharacteristics.LENS_FACING_FRONT
    } else {
      CameraCharacteristics.LENS_FACING_BACK
    }
    val cameraId = manager.cameraIdList.firstOrNull {
      manager.getCameraCharacteristics(it).get(CameraCharacteristics.LENS_FACING) == facing
    } ?: run {
      invoke.reject("camera device not found: ${args.deviceId}")
      return
    }
    val characteristics = manager.getCameraCharacteristics(cameraId)
    val map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP)
    val sizes = map?.getOutputSizes(android.graphics.SurfaceTexture::class.java)
      ?.filter { it.width >= 640 && it.height >= 480 }
      ?.distinctBy { it.width to it.height }
      ?.sortedByDescending { it.width.toLong() * it.height }
      ?.take(24)
      ?: emptyList<Size>()
    val fps = characteristics.get(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES)
      ?.flatMap { listOf(it.lower, it.upper) }
      ?.filter { it in 1..240 }
      ?.distinct()
      ?.sorted()
      ?: listOf(30)
    val formats = JSArray()
    val resolutions = JSArray()
    sizes.forEach { size ->
      resolutions.put(JSArray().apply { put(size.width); put(size.height) })
      formats.put(JSObject().apply {
        put("width", size.width)
        put("height", size.height)
        put("frame_rates", JSArray(fps))
      })
    }
    invoke.resolve(JSObject().apply {
      put("supports_still", true)
      put("supports_video", true)
      put("supports_audio", true)
      put("resolutions", resolutions)
      put("frame_rates", JSArray(fps))
      put("formats", formats)
      put("manual_iso", null)
      put("manual_shutter", false)
      put("manual_focus", false)
      put("raw_photo", false)
      put("log_video", false)
      put("hdr_video", false)
    })
  }

  private fun layoutParams(viewport: ViewportArgs): FrameLayout.LayoutParams {
    val density = activity.resources.displayMetrics.density
    return FrameLayout.LayoutParams(
      (viewport.width * density).toInt().coerceAtLeast(1),
      (viewport.height * density).toInt().coerceAtLeast(1)
    ).apply {
      leftMargin = (viewport.x * density).toInt()
      topMargin = (viewport.y * density).toInt()
    }
  }

  private fun resolutionSelector(format: FormatArgs): ResolutionSelector =
    ResolutionSelector.Builder()
      .setResolutionStrategy(
        ResolutionStrategy(
          Size(format.width, format.height),
          ResolutionStrategy.FALLBACK_RULE_NONE
        )
      )
      .build()

  private fun formatResponse(format: FormatArgs): JSObject = JSObject().apply {
    put("width", format.width)
    put("height", format.height)
    put("fps", format.fps.toDouble())
    put("settings_persisted", false)
    put("settings_warning", null)
  }

  @ExperimentalCamera2Interop
  private fun bindUseCases(
    provider: ProcessCameraProvider,
    view: PreviewView,
    deviceId: String
  ) {
    val selector = CameraSelector.Builder()
      .requireLensFacing(if (deviceId == "front") CameraSelector.LENS_FACING_FRONT else CameraSelector.LENS_FACING_BACK)
      .build()
    val format = requestedFormat
    val previewBuilder = Preview.Builder()
    val stillBuilder = ImageCapture.Builder()
      .setCaptureMode(ImageCapture.CAPTURE_MODE_MAXIMIZE_QUALITY)
    val recorderBuilder = Recorder.Builder()
    val movieBuilder = VideoCapture.Builder(recorderBuilder.build())
    if (format != null) {
      if (format.width <= 0 || format.height <= 0 || format.fps <= 0) {
        throw IllegalArgumentException("camera format values must be positive")
      }
      val resolution = resolutionSelector(format)
      previewBuilder.setResolutionSelector(resolution)
      stillBuilder.setResolutionSelector(resolution)
      movieBuilder.setResolutionSelector(resolution)
      val fpsRange = Range(format.fps, format.fps)
      Camera2Interop.Extender(previewBuilder)
        .setCaptureRequestOption(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, fpsRange)
      Camera2Interop.Extender(stillBuilder)
        .setCaptureRequestOption(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, fpsRange)
      Camera2Interop.Extender(movieBuilder)
        .setCaptureRequestOption(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, fpsRange)
    }
    val preview = previewBuilder.build().also { it.surfaceProvider = view.surfaceProvider }
    val still = stillBuilder.build()
    val movie = movieBuilder.build()
    provider.unbindAll()
    provider.bindToLifecycle(activity as AppCompatActivity, selector, preview, still, movie)
    imageCapture = still
    videoCapture = movie
  }

  @Command
  fun startPreview(invoke: Invoke) {
    if (authorization() != "authorized") {
      invoke.reject("camera access is not authorized")
      return
    }
    val args = invoke.parseArgs(PreviewArgs::class.java)
    activeDeviceId = args.deviceId
    activity.runOnUiThread {
      stopPreviewInternal()
      val view = PreviewView(activity).apply {
        implementationMode = PreviewView.ImplementationMode.COMPATIBLE
        scaleType = PreviewView.ScaleType.FILL_CENTER
      }
      activity.addContentView(view, layoutParams(args.viewport))
      previewView = view
      val future = ProcessCameraProvider.getInstance(activity)
      future.addListener({
        try {
          val provider = future.get()
          bindUseCases(provider, view, args.deviceId)
          cameraProvider = provider
          val format = requestedFormat
          invoke.resolve(JSObject().apply {
            put("running", true)
            put("device_id", args.deviceId)
            put("active_format", format?.let(::formatResponse))
            put("format_restored", false)
            put("settings_warning", null)
            put("orientation", JSObject().apply {
              put("rotation_degrees", 0)
              put("preview_mirrored", args.deviceId == "front")
              put("capture_mirrored", false)
            })
          })
        } catch (error: Exception) {
          stopPreviewInternal()
          invoke.reject("failed to start CameraX preview", error)
        }
      }, ContextCompat.getMainExecutor(activity))
    }
  }

  @Command
  fun applyFormat(invoke: Invoke) {
    val args = invoke.parseArgs(FormatArgs::class.java)
    if (recording != null) {
      invoke.reject("camera format cannot change while recording")
      return
    }
    val provider = cameraProvider
    val view = previewView
    if (provider == null || view == null) {
      invoke.reject("camera format can only change while previewing")
      return
    }
    activity.runOnUiThread {
      val previous = requestedFormat
      requestedFormat = args
      try {
        bindUseCases(provider, view, activeDeviceId)
        invoke.resolve(formatResponse(args))
      } catch (error: Exception) {
        requestedFormat = previous
        try {
          bindUseCases(provider, view, activeDeviceId)
        } catch (_: Exception) {
          stopPreviewInternal()
        }
        invoke.reject("requested CameraX format is not supported by the active use-case combination", error)
      }
    }
  }

  @Command
  fun capturePhoto(invoke: Invoke) {
    val args = invoke.parseArgs(OutputArgs::class.java)
    val capture = imageCapture ?: run {
      invoke.reject("photo capture is unavailable because preview is not running")
      return
    }
    if (args.path.isBlank()) {
      invoke.reject("photo output path is empty")
      return
    }
    val file = java.io.File(args.path)
    file.parentFile?.mkdirs()
    capture.takePicture(
      ImageCapture.OutputFileOptions.Builder(file).build(),
      ContextCompat.getMainExecutor(activity),
      object : ImageCapture.OnImageSavedCallback {
        override fun onImageSaved(output: ImageCapture.OutputFileResults) {
          invoke.resolve(JSObject().apply {
            put("path", file.absolutePath)
            put("device_id", activeDeviceId)
            put("active_format", requestedFormat?.let(::formatResponse))
          })
        }

        override fun onError(error: ImageCaptureException) {
          invoke.reject("CameraX photo capture failed", error)
        }
      }
    )
  }

  @Command
  fun startVideo(invoke: Invoke) {
    val args = invoke.parseArgs(VideoOutputArgs::class.java)
    val capture = videoCapture ?: run {
      invoke.reject("video capture is unavailable because preview is not running")
      return
    }
    if (recording != null) {
      invoke.reject("video recording is already active")
      return
    }
    if (args.path.isBlank()) {
      invoke.reject("video output path is empty")
      return
    }
    if (args.audioEnabled && microphoneAuthorization() != "authorized") {
      invoke.reject("microphone access is not authorized")
      return
    }
    val file = java.io.File(args.path)
    file.parentFile?.mkdirs()
    finalizedRecording = null
    finalizedRecordingError = null
    storageStopRequested = false
    var pending = capture.output.prepareRecording(
      activity,
      FileOutputOptions.Builder(file).build()
    )
    if (args.audioEnabled) pending = pending.withAudioEnabled()
    recordingPath = file.absolutePath
    recording = pending.start(ContextCompat.getMainExecutor(activity)) { event ->
      when (event) {
        is VideoRecordEvent.Start -> {
          invoke.resolve()
          startStorageMonitor(file, args.minimumAvailableBytes)
        }
        is VideoRecordEvent.Finalize -> {
          stopStorageMonitor()
          val path = recordingPath
          recording = null
          recordingPath = null
          val stopInvoke = stopRecordingInvoke
          stopRecordingInvoke = null
          if (event.hasError()) {
            val message = "CameraX video finalize failed (${event.error})"
            if (stopInvoke != null) stopInvoke.reject(message) else finalizedRecordingError = message
          } else if (path == null) {
            val message = "CameraX video output path is unavailable"
            if (stopInvoke != null) stopInvoke.reject(message) else finalizedRecordingError = message
          } else {
            val result = JSObject().apply {
              put("path", path)
              put("device_id", activeDeviceId)
              put("active_format", requestedFormat?.let(::formatResponse))
            }
            if (stopInvoke != null) stopInvoke.resolve(result) else finalizedRecording = result
          }
          if (lifecycleStopPending) {
            lifecycleStopPending = false
            teardownPreview(preserveFinalizedRecording = true)
          }
        }
      }
    }
  }

  @Command
  fun stopVideo(invoke: Invoke) {
    val active = recording ?: run {
      finalizedRecording?.let {
        finalizedRecording = null
        invoke.resolve(it)
        return
      }
      finalizedRecordingError?.let {
        finalizedRecordingError = null
        invoke.reject(it)
        return
      }
      invoke.reject("video recording is not active")
      return
    }
    if (stopRecordingInvoke != null) {
      invoke.reject("video recording is already stopping")
      return
    }
    stopRecordingInvoke = invoke
    active.stop()
  }

  private fun startStorageMonitor(file: java.io.File, minimumAvailableBytes: Long) {
    stopStorageMonitor()
    if (minimumAvailableBytes <= 0) return
    val monitor = object : Runnable {
      override fun run() {
        val parent = file.parentFile
        if (!storageStopRequested && recording != null && parent != null && parent.usableSpace < minimumAvailableBytes) {
          storageStopRequested = true
          recording?.stop()
          return
        }
        if (recording != null) storageHandler.postDelayed(this, 2000)
      }
    }
    storageMonitor = monitor
    storageHandler.postDelayed(monitor, 2000)
  }

  private fun stopStorageMonitor() {
    storageMonitor?.let(storageHandler::removeCallbacks)
    storageMonitor = null
  }

  @Command
  fun resizePreview(invoke: Invoke) {
    val args = invoke.parseArgs(ViewportArgs::class.java)
    activity.runOnUiThread {
      previewView?.layoutParams = layoutParams(args)
      invoke.resolve()
    }
  }

  @Command
  fun setOrientation(invoke: Invoke) {
    val args = invoke.parseArgs(OrientationArgs::class.java)
    activity.runOnUiThread {
      previewView?.display?.rotation?.let { _ -> previewView?.rotation = 0f }
      invoke.resolve(JSObject().apply {
        put("rotation_degrees", args.rotationDegrees)
        put("preview_mirrored", args.previewMirrored)
        put("capture_mirrored", args.captureMirrored)
      })
    }
  }

  private fun teardownPreview(preserveFinalizedRecording: Boolean = false) {
    stopStorageMonitor()
    recording?.close()
    recording = null
    recordingPath = null
    if (!preserveFinalizedRecording) {
      finalizedRecording = null
      finalizedRecordingError = null
    }
    storageStopRequested = false
    stopRecordingInvoke?.reject("video recording was interrupted while closing preview")
    stopRecordingInvoke = null
    cameraProvider?.unbindAll()
    cameraProvider = null
    imageCapture = null
    videoCapture = null
    previewView?.let { (it.parent as? ViewGroup)?.removeView(it) }
    previewView = null
  }

  private fun stopPreviewInternal() {
    teardownPreview()
  }

  private fun stopForLifecyclePause() {
    if (recording == null) {
      teardownPreview(preserveFinalizedRecording = true)
      return
    }
    stopStorageMonitor()
    lifecycleStopPending = true
    storageStopRequested = true
    recording?.stop()
  }

  @Command
  fun stopPreview(invoke: Invoke) {
    activity.runOnUiThread {
      stopPreviewInternal()
      invoke.resolve()
    }
  }

  override fun onPause() {
    stopForLifecyclePause()
  }

  override fun onDestroy(activity: AppCompatActivity) {
    stopForLifecyclePause()
  }
}
