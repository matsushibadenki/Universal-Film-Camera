import AVFoundation
import SwiftUI
import UIKit
import VPXIPhoneCapture

private final class CameraVPXFrames: @unchecked Sendable {
    static let shared = CameraVPXFrames()
    private let lock = NSLock()
    private var input: ExternalVideoInput?

    func setInput(_ input: ExternalVideoInput?) { lock.withLock { self.input = input } }

    func submit(_ sample: CMSampleBuffer, clock: CMClock) {
        guard lock.try() else { return }
        let input = self.input
        lock.unlock()
        input?.submit(sample, synchronizationClock: clock, cameraID: "Universal Film Camera")
    }
}

@_cdecl("ufc_vpx_video_sample")
func ufcVPXVideoSample(_ sample: UnsafeRawPointer, _ clock: UnsafeRawPointer) {
    CameraVPXFrames.shared.submit(Unmanaged<CMSampleBuffer>.fromOpaque(sample).takeUnretainedValue(),
                                  clock: Unmanaged<CMClock>.fromOpaque(clock).takeUnretainedValue())
}

@MainActor
private enum CameraVPXConnection {
    static let session = CaptureNodeSessionModel(
        displayName: "Universal Film Camera", externalCapabilities: [])

    static let backgroundObserver = NotificationCenter.default.addObserver(
        forName: UIApplication.didEnterBackgroundNotification, object: nil, queue: .main
    ) { _ in
        MainActor.assumeIsolated {
            session.stop()
            CameraVPXFrames.shared.setInput(nil)
        }
    }
}

@MainActor
@_cdecl("ufc_vpx_present")
func ufcVPXPresent() -> Bool {
    guard let scene = UIApplication.shared.connectedScenes.compactMap({ $0 as? UIWindowScene })
        .first(where: { $0.activationState == .foregroundActive }),
        let root = scene.keyWindow?.rootViewController, root.presentedViewController == nil else { return false }
    _ = CameraVPXConnection.backgroundObserver
    let controller = UIHostingController(rootView: CameraVPXPanel())
    controller.modalPresentationStyle = .pageSheet
    root.present(controller, animated: true)
    return true
}

private struct CameraVPXPanel: View {
    private var vpxSession: CaptureNodeSessionModel { CameraVPXConnection.session }
    @State private var inputError = false

    var body: some View {
        CaptureNodeControlView(model: vpxSession, stopOnDisappear: false)
            .onChange(of: vpxSession.externalCaptureID) { _, captureID in
                CameraVPXFrames.shared.setInput(nil)
                guard captureID != nil else { return }
                do {
                    CameraVPXFrames.shared.setInput(try vpxSession.makeExternalVideoInput())
                } catch {
                    inputError = true
                    vpxSession.stopCapture()
                }
            }
            .overlay(alignment: .bottom) {
                if inputError {
                    Image(systemName: "exclamationmark.triangle.fill")
                        .foregroundStyle(.orange)
                        .accessibilityLabel("VPX HEVC")
                }
            }
    }

}
