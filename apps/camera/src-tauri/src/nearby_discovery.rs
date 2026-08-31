use camera_core::CapturedAsset;
use peer_transfer_core::{
    AssetSelection, AssetTransferManifest, DEFAULT_RECEIVE_RESERVE_BYTES, EncryptedChunkCodec,
    EncryptedTransferSender, EphemeralKeyPair, EphemeralPublicKey, HandshakeApproval,
    HandshakeOffer, IndexedOriginalReceive, Invitation, LocalNetworkTransport, MAX_CHUNK_BYTES,
    MetadataPolicy, PROTOCOL_VERSION, PeerCapabilities, PeerIdentity, PeerWireMessage,
    TransferCancelReason, TransferSession, TransferState, TransportKind, complete_mutual_handshake,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

pub const NEARBY_SERVICE_TYPE: &str = "_ufcamera._tcp.local.";

fn current_unix_ms() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .map_err(|_| "system clock is before the Unix epoch".to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NearbyLocalPeer {
    pub ephemeral_id: String,
    pub display_label: Option<String>,
    pub public_key_hex: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DiscoveredNearbyPeer {
    pub ephemeral_id: String,
    pub display_label: Option<String>,
    pub public_key_hex: String,
    pub host: String,
    pub addresses: Vec<String>,
    pub port: u16,
    pub protocol_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NearbyDiscoverySnapshot {
    pub supported: bool,
    pub active: bool,
    pub local_peer: Option<NearbyLocalPeer>,
    pub peers: Vec<DiscoveredNearbyPeer>,
    pub last_error: Option<String>,
    pub approval: Option<NearbyApprovalSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NearbyApprovalSnapshot {
    pub invitation_id: String,
    pub peer_ephemeral_id: String,
    pub asset_id: String,
    pub file_name: String,
    pub byte_length: u64,
    pub confirmation_code: String,
    pub expires_at_unix_ms: u64,
    pub local_approved: bool,
    pub remote_approved: bool,
    pub direction: NearbyApprovalDirection,
    pub transferred_bytes: u64,
    pub transfer_active: bool,
    pub cancel_requested: bool,
    pub retry_available: bool,
    pub failure_kind: Option<NearbyTransferFailureKind>,
    pub finalized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NearbyTransferFailureKind {
    Disconnected,
    Timeout,
    Integrity,
    Storage,
    InvitationExpired,
    Cancelled,
    Protocol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NearbyApprovalDirection {
    Outgoing,
    Incoming,
}

#[derive(Debug, Deserialize)]
pub struct StartNearbyDiscoveryRequest {
    pub display_label: Option<String>,
    pub port: u16,
}

pub struct NearbyDiscoveryState {
    runtime: Mutex<NearbyDiscoveryRuntime>,
}

impl Default for NearbyDiscoveryState {
    fn default() -> Self {
        Self {
            runtime: Mutex::new(NearbyDiscoveryRuntime::default()),
        }
    }
}

#[derive(Default)]
struct NearbyDiscoveryRuntime {
    local_peer: Option<NearbyLocalPeer>,
    peers: BTreeMap<String, DiscoveredNearbyPeer>,
    last_error: Option<String>,
    approval: Option<PreparedApproval>,
    secure_session: Option<SecureNearbySession>,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    apple: Option<AppleDiscoveryRuntime>,
}

struct PreparedApproval {
    peer_ephemeral_id: String,
    asset_id: String,
    session: TransferSession,
    offer: HandshakeOffer,
    direction: NearbyApprovalDirection,
    transport: Option<LocalNetworkTransport>,
    remote_approved: bool,
    source_path: Option<std::path::PathBuf>,
    transferred_bytes: u64,
    progress: Arc<TransferProgress>,
    retry_available: bool,
    failure_kind: Option<NearbyTransferFailureKind>,
    finalized: bool,
}

#[derive(Default)]
struct TransferProgress {
    transferred_bytes: AtomicU64,
    active: AtomicBool,
    cancel_requested: AtomicBool,
}

struct SecureNearbySession {
    session: TransferSession,
    codec: EncryptedChunkCodec,
    transport: LocalNetworkTransport,
    offer: HandshakeOffer,
    direction: NearbyApprovalDirection,
    source_path: Option<std::path::PathBuf>,
    progress: Arc<TransferProgress>,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
struct AppleDiscoveryRuntime {
    daemon: mdns_sd::ServiceDaemon,
    events: mdns_sd::Receiver<mdns_sd::ServiceEvent>,
    monitor: mdns_sd::Receiver<mdns_sd::DaemonEvent>,
    service_fullname: String,
    listener: std::net::TcpListener,
    key_pair: Option<EphemeralKeyPair>,
}

impl NearbyDiscoveryState {
    pub fn start(
        &self,
        request: StartNearbyDiscoveryRequest,
    ) -> Result<NearbyDiscoverySnapshot, String> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "nearby discovery lock poisoned".to_owned())?;
        let display_label = validate_display_label(request.display_label)?;
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        {
            if runtime.apple.is_some() {
                return Err("nearby discovery is already active".into());
            }
            let listener =
                std::net::TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, request.port))
                    .map_err(|error| format!("failed to bind nearby TCP listener: {error}"))?;
            listener
                .set_nonblocking(true)
                .map_err(|error| format!("failed to configure nearby TCP listener: {error}"))?;
            let listening_port = listener
                .local_addr()
                .map_err(|error| format!("failed to inspect nearby TCP listener: {error}"))?
                .port();
            let key_pair = EphemeralKeyPair::generate().map_err(|error| error.to_string())?;
            let public_key_hex = encode_hex(&key_pair.public_key().bytes);
            let ephemeral_id = public_key_hex[..12].to_owned();
            let local_peer = NearbyLocalPeer {
                ephemeral_id: ephemeral_id.clone(),
                display_label: display_label.clone(),
                public_key_hex: public_key_hex.clone(),
                port: listening_port,
            };
            let daemon = mdns_sd::ServiceDaemon::new().map_err(|error| error.to_string())?;
            daemon
                .include_apple_p2p(true)
                .map_err(|error| error.to_string())?;
            let events = daemon
                .browse(NEARBY_SERVICE_TYPE)
                .map_err(|error| error.to_string())?;
            let monitor = daemon.monitor().map_err(|error| error.to_string())?;
            let instance_name = format!("ufc-{ephemeral_id}");
            let hostname = format!("{instance_name}.local.");
            let mut properties = HashMap::new();
            properties.insert("v".to_owned(), PROTOCOL_VERSION.to_string());
            properties.insert("peer".to_owned(), ephemeral_id);
            properties.insert("pk".to_owned(), public_key_hex);
            if let Some(label) = &display_label {
                properties.insert("label".to_owned(), label.clone());
            }
            let service = mdns_sd::ServiceInfo::new(
                NEARBY_SERVICE_TYPE,
                &instance_name,
                &hostname,
                "",
                listening_port,
                properties,
            )
            .map_err(|error| error.to_string())?
            .enable_addr_auto();
            let service_fullname = service.get_fullname().to_owned();
            daemon
                .register(service)
                .map_err(|error| error.to_string())?;
            runtime.local_peer = Some(local_peer);
            runtime.peers.clear();
            runtime.last_error = None;
            if let Some(prepared) = runtime.approval.as_mut() {
                prepared.failure_kind = None;
                prepared.retry_available = false;
            }
            runtime.approval = None;
            runtime.secure_session = None;
            runtime.apple = Some(AppleDiscoveryRuntime {
                daemon,
                events,
                monitor,
                service_fullname,
                listener,
                key_pair: Some(key_pair),
            });
            poll_apple_events(&mut runtime);
            Ok(snapshot(&runtime, true))
        }
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        {
            let _ = display_label;
            Err("nearby Bonjour discovery is not implemented on this platform".into())
        }
    }

    pub fn snapshot(&self) -> Result<NearbyDiscoverySnapshot, String> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "nearby discovery lock poisoned".to_owned())?;
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        poll_apple_events(&mut runtime);
        Ok(snapshot(
            &runtime,
            cfg!(any(target_os = "macos", target_os = "ios")),
        ))
    }

    pub fn stop(&self) -> Result<NearbyDiscoverySnapshot, String> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "nearby discovery lock poisoned".to_owned())?;
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if let Some(apple) = runtime.apple.take() {
            shutdown_apple(apple);
        }
        runtime.local_peer = None;
        runtime.peers.clear();
        runtime.last_error = None;
        runtime.approval = None;
        runtime.secure_session = None;
        Ok(snapshot(
            &runtime,
            cfg!(any(target_os = "macos", target_os = "ios")),
        ))
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn prepare_approval(
        &self,
        peer_ephemeral_id: &str,
        asset: &CapturedAsset,
        now_unix_ms: u64,
    ) -> Result<NearbyDiscoverySnapshot, String> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "nearby discovery lock poisoned")?;
        let peer = runtime
            .peers
            .values()
            .find(|peer| peer.ephemeral_id == peer_ephemeral_id)
            .cloned()
            .ok_or_else(|| "selected nearby peer is no longer available".to_owned())?;
        let apple = runtime
            .apple
            .as_ref()
            .ok_or_else(|| "nearby discovery is not active".to_owned())?;
        let local = runtime
            .local_peer
            .as_ref()
            .ok_or_else(|| "nearby local identity is unavailable".to_owned())?;
        let mut random = [0_u8; 16];
        getrandom::getrandom(&mut random)
            .map_err(|_| "secure invitation ID generation failed".to_owned())?;
        let invitation_id = format!("inv-{}", encode_hex(&random));
        let transfer_prefix = format!("share-{}", encode_hex(&random[..8]));
        let bundle = AssetTransferManifest::from_captured_asset(
            asset,
            AssetSelection::Original,
            &transfer_prefix,
            256 * 1024,
            MetadataPolicy::Preserve,
        )
        .map_err(|error| error.to_string())?;
        let resource = bundle
            .resources
            .first()
            .ok_or_else(|| "transfer manifest has no original resource".to_owned())?
            .clone();
        let manifest = resource.manifest.clone();
        let peer_public = decode_public_key(&peer.public_key_hex)?;
        let key_pair = apple
            .key_pair
            .as_ref()
            .ok_or_else(|| "nearby session key is already in use".to_owned())?;
        let code = key_pair
            .confirmation_code(peer_public, &invitation_id, &local.ephemeral_id, &manifest)
            .map_err(|error| error.to_string())?;
        let invitation = Invitation {
            invitation_id,
            sender: PeerIdentity {
                ephemeral_id: local.ephemeral_id.clone(),
                display_label: local.display_label.clone(),
            },
            confirmation_code: code,
            expires_at_unix_ms: now_unix_ms.saturating_add(120_000),
        };
        let session = TransferSession::new(invitation.clone(), manifest.clone())
            .map_err(|error| error.to_string())?;
        let offer = HandshakeOffer {
            invitation,
            manifest,
            sender_public_key: key_pair.public_key(),
            sender_capabilities: local_capabilities(),
            resource,
            capture: bundle.capture,
        };
        runtime.approval = Some(PreparedApproval {
            peer_ephemeral_id: peer.ephemeral_id,
            asset_id: asset.id.clone(),
            session,
            offer,
            direction: NearbyApprovalDirection::Outgoing,
            transport: None,
            remote_approved: false,
            source_path: Some(asset.original.path.clone()),
            transferred_bytes: 0,
            progress: Arc::new(TransferProgress::default()),
            retry_available: false,
            failure_kind: None,
            finalized: false,
        });
        Ok(snapshot(&runtime, true))
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    pub fn prepare_approval(
        &self,
        _peer_ephemeral_id: &str,
        _asset: &CapturedAsset,
        _now_unix_ms: u64,
    ) -> Result<NearbyDiscoverySnapshot, String> {
        Err("nearby approval is not implemented on this platform".into())
    }

    pub fn approve(
        &self,
        confirmation_code: &str,
        now_unix_ms: u64,
    ) -> Result<NearbyDiscoverySnapshot, String> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "nearby discovery lock poisoned")?;
        let approval = runtime
            .approval
            .as_mut()
            .ok_or_else(|| "no nearby approval is prepared".to_owned())?;
        approval
            .session
            .approve(confirmation_code, now_unix_ms)
            .map_err(|error| error.to_string())?;
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if approval.direction == NearbyApprovalDirection::Incoming {
            complete_incoming_approval(&mut runtime)?;
        }
        Ok(snapshot(&runtime, true))
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    pub fn connect_outgoing(&self) -> Result<NearbyDiscoverySnapshot, String> {
        let result = self.connect_outgoing_inner();
        if let Err(error) = &result
            && let Ok(mut runtime) = self.runtime.lock()
        {
            record_transfer_failure(&mut runtime, error);
        }
        result
    }

    #[cfg(any(target_os = "macos", target_os = "ios"))]
    fn connect_outgoing_inner(&self) -> Result<NearbyDiscoverySnapshot, String> {
        let (peer, offer, session, is_retry) = {
            let runtime = self
                .runtime
                .lock()
                .map_err(|_| "nearby discovery lock poisoned")?;
            let prepared = runtime
                .approval
                .as_ref()
                .ok_or_else(|| "no nearby approval is prepared".to_owned())?;
            if prepared.direction != NearbyApprovalDirection::Outgoing {
                return Err("outgoing nearby transfer is not locally approved".into());
            }
            let (session, is_retry) =
                if matches!(prepared.session.state, TransferState::Negotiating) {
                    (prepared.session.clone(), false)
                } else if prepared.retry_available
                    && matches!(prepared.session.state, TransferState::Transferring { .. })
                {
                    let mut session = TransferSession::new(
                        prepared.offer.invitation.clone(),
                        prepared.offer.manifest.clone(),
                    )
                    .map_err(|error| error.to_string())?;
                    session
                        .approve(
                            &prepared.offer.invitation.confirmation_code,
                            current_unix_ms()?,
                        )
                        .map_err(|error| error.to_string())?;
                    (session, true)
                } else {
                    return Err("outgoing nearby transfer is not ready to reconnect".into());
                };
            let peer = runtime
                .peers
                .values()
                .find(|peer| peer.ephemeral_id == prepared.peer_ephemeral_id)
                .cloned()
                .ok_or_else(|| "selected nearby peer is no longer available".to_owned())?;
            (peer, prepared.offer.clone(), session, is_retry)
        };
        let mut last_error = "nearby peer has no usable network address".to_owned();
        let mut connected = None;
        for address in &peer.addresses {
            let endpoint = if address.contains(':') {
                format!("[{address}]:{}", peer.port)
            } else {
                format!("{address}:{}", peer.port)
            };
            let Ok(endpoint) = endpoint.parse() else {
                continue;
            };
            match LocalNetworkTransport::connect_timeout(
                endpoint,
                std::time::Duration::from_secs(5),
            ) {
                Ok(transport) => {
                    connected = Some(transport);
                    break;
                }
                Err(error) => last_error = error.to_string(),
            }
        }
        let mut transport = connected.ok_or(last_error)?;
        transport
            .set_timeouts(std::time::Duration::from_secs(125))
            .map_err(|error| error.to_string())?;
        transport
            .send(&PeerWireMessage::HandshakeOffer(offer.clone()))
            .map_err(|error| error.to_string())?;
        let PeerWireMessage::HandshakeApproval(remote_approval) =
            transport.receive().map_err(|error| error.to_string())?
        else {
            return Err("nearby peer returned an unexpected handshake message".into());
        };
        remote_approval
            .validate_for(&offer)
            .map_err(|error| error.to_string())?;
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "nearby discovery lock poisoned")?;
        let prepared = runtime
            .approval
            .as_ref()
            .ok_or_else(|| "nearby approval was cancelled".to_owned())?;
        if prepared.offer != offer
            || (!is_retry && prepared.session != session)
            || (is_retry && !prepared.retry_available)
        {
            return Err("nearby approval changed while connecting".into());
        }
        let source_path = prepared.source_path.clone();
        let progress = Arc::clone(&prepared.progress);
        let key_pair = runtime
            .apple
            .as_ref()
            .and_then(|apple| apple.key_pair.as_ref())
            .ok_or_else(|| "nearby session key is unavailable".to_owned())?;
        let (session, codec) =
            complete_mutual_handshake(key_pair, &offer, &remote_approval, session)
                .map_err(|error| error.to_string())?;
        let snapshot_session = session.clone();
        runtime.secure_session = Some(SecureNearbySession {
            session,
            codec,
            transport,
            offer: offer.clone(),
            direction: NearbyApprovalDirection::Outgoing,
            source_path,
            progress,
        });
        if let Some(prepared) = runtime.approval.as_mut() {
            prepared.session = snapshot_session;
            prepared.remote_approved = true;
            prepared.retry_available = false;
            prepared.failure_kind = None;
        }
        Ok(snapshot(&runtime, true))
    }

    #[cfg(not(any(target_os = "macos", target_os = "ios")))]
    pub fn connect_outgoing(&self) -> Result<NearbyDiscoverySnapshot, String> {
        Err("nearby connection is not implemented on this platform".into())
    }

    pub fn run_secure_transfer(
        &self,
        captures_root: &std::path::Path,
        available_bytes: u64,
    ) -> Result<NearbyDiscoverySnapshot, String> {
        let secure = {
            let mut runtime = self
                .runtime
                .lock()
                .map_err(|_| "nearby discovery lock poisoned")?;
            runtime.last_error = None;
            runtime
                .secure_session
                .take()
                .ok_or_else(|| "secure nearby session is not established".to_owned())?
        };
        let direction = secure.direction;
        secure
            .progress
            .cancel_requested
            .store(false, Ordering::Release);
        secure.progress.active.store(true, Ordering::Release);
        let progress = Arc::clone(&secure.progress);
        let result = match direction {
            NearbyApprovalDirection::Outgoing => run_outgoing_transfer(secure),
            NearbyApprovalDirection::Incoming => {
                run_incoming_transfer(secure, captures_root, available_bytes)
            }
        };
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "nearby discovery lock poisoned")?;
        progress.active.store(false, Ordering::Release);
        match result {
            Ok(transferred_bytes) => {
                if let Some(prepared) = runtime.approval.as_mut() {
                    prepared.transferred_bytes = transferred_bytes;
                    prepared.failure_kind = None;
                    prepared.finalized = true;
                }
                Ok(snapshot(
                    &runtime,
                    cfg!(any(target_os = "macos", target_os = "ios")),
                ))
            }
            Err(error) => {
                record_transfer_failure(&mut runtime, &error);
                Err(error)
            }
        }
    }

    pub fn request_transfer_cancel(&self) -> Result<NearbyDiscoverySnapshot, String> {
        let runtime = self
            .runtime
            .lock()
            .map_err(|_| "nearby discovery lock poisoned")?;
        let prepared = runtime
            .approval
            .as_ref()
            .ok_or_else(|| "no nearby transfer is prepared".to_owned())?;
        if !prepared.progress.active.load(Ordering::Acquire) {
            return Err("nearby transfer is not active".into());
        }
        prepared
            .progress
            .cancel_requested
            .store(true, Ordering::Release);
        Ok(snapshot(
            &runtime,
            cfg!(any(target_os = "macos", target_os = "ios")),
        ))
    }

    pub fn cancel_approval(&self) -> Result<NearbyDiscoverySnapshot, String> {
        let mut runtime = self
            .runtime
            .lock()
            .map_err(|_| "nearby discovery lock poisoned")?;
        runtime.approval = None;
        Ok(snapshot(
            &runtime,
            cfg!(any(target_os = "macos", target_os = "ios")),
        ))
    }
}

fn classify_transfer_failure(error: &str) -> NearbyTransferFailureKind {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("cancel") {
        NearbyTransferFailureKind::Cancelled
    } else if normalized.contains("invitationexpired") || normalized.contains("invitation expired")
    {
        NearbyTransferFailureKind::InvitationExpired
    } else if normalized.contains("insufficientstorage")
        || normalized.contains("not enough")
        || normalized.contains("no space")
    {
        NearbyTransferFailureKind::Storage
    } else if normalized.contains("contentverificationfailed")
        || normalized.contains("resumeprefixmismatch")
        || normalized.contains("authenticationfailed")
        || normalized.contains("manifestmismatch")
        || normalized.contains("hash")
        || normalized.contains("finalization did not match")
    {
        NearbyTransferFailureKind::Integrity
    } else if normalized.contains("timed out") || normalized.contains("timeout") {
        NearbyTransferFailureKind::Timeout
    } else if normalized.contains("connection")
        || normalized.contains("broken pipe")
        || normalized.contains("unexpected eof")
        || normalized.contains("peer disconnected")
        || normalized.starts_with("io(")
    {
        NearbyTransferFailureKind::Disconnected
    } else {
        NearbyTransferFailureKind::Protocol
    }
}

fn retryable_failure(kind: NearbyTransferFailureKind) -> bool {
    matches!(
        kind,
        NearbyTransferFailureKind::Disconnected | NearbyTransferFailureKind::Timeout
    )
}

fn record_transfer_failure(runtime: &mut NearbyDiscoveryRuntime, error: &str) {
    let kind = classify_transfer_failure(error);
    runtime.last_error = Some(error.to_owned());
    if let Some(prepared) = runtime.approval.as_mut() {
        prepared.failure_kind = Some(kind);
        prepared.retry_available = retryable_failure(kind)
            && !prepared.progress.cancel_requested.load(Ordering::Acquire)
            && current_unix_ms()
                .is_ok_and(|now| now <= prepared.session.invitation.expires_at_unix_ms);
    }
}

fn run_outgoing_transfer(mut secure: SecureNearbySession) -> Result<u64, String> {
    let source_path = secure
        .source_path
        .take()
        .ok_or_else(|| "outgoing source path is unavailable".to_owned())?;
    let total = secure.session.manifest.byte_length;
    let PeerWireMessage::ResumeCheckpoint(checkpoint) = secure
        .transport
        .receive()
        .map_err(|error| error.to_string())?
    else {
        return Err("nearby receiver did not provide a durable resume checkpoint".into());
    };
    let mut sender = EncryptedTransferSender::open_at_checkpoint(
        &source_path,
        &secure.session,
        secure.codec,
        &checkpoint,
    )
    .map_err(|error| error.to_string())?;
    while let Some(message) = sender.next_chunk().map_err(|error| error.to_string())? {
        if secure.progress.cancel_requested.load(Ordering::Acquire) {
            let cancel = sender
                .cancel(TransferCancelReason::User)
                .map_err(|error| error.to_string())?;
            secure
                .transport
                .send(&cancel)
                .map_err(|error| error.to_string())?;
            return Err("nearby transfer was cancelled".into());
        }
        secure
            .transport
            .send(&message)
            .map_err(|error| error.to_string())?;
        let ack = secure
            .transport
            .receive()
            .map_err(|error| error.to_string())?;
        let persisted = sender.accept_ack(&ack).map_err(|error| error.to_string())?;
        secure
            .session
            .acknowledge(persisted)
            .map_err(|error| error.to_string())?;
        secure
            .progress
            .transferred_bytes
            .store(persisted, Ordering::Release);
    }
    let PeerWireMessage::TransferFinalized {
        transfer_id,
        sha256_hex,
    } = secure
        .transport
        .receive()
        .map_err(|error| error.to_string())?
    else {
        return Err("nearby receiver did not confirm Media finalization".into());
    };
    if transfer_id != secure.session.manifest.transfer_id
        || !sha256_hex.eq_ignore_ascii_case(&secure.session.manifest.sha256_hex)
    {
        return Err("nearby receiver finalization did not match the transfer manifest".into());
    }
    Ok(total)
}

fn run_incoming_transfer(
    mut secure: SecureNearbySession,
    captures_root: &std::path::Path,
    available_bytes: u64,
) -> Result<u64, String> {
    let total = secure.session.manifest.byte_length;
    let mut receive = IndexedOriginalReceive::create_or_resume(
        captures_root,
        secure.offer.resource.clone(),
        secure.offer.capture.clone(),
        available_bytes,
        DEFAULT_RECEIVE_RESERVE_BYTES,
    )
    .map_err(|error| error.to_string())?;
    secure
        .transport
        .send(&PeerWireMessage::ResumeCheckpoint(
            receive.resume_checkpoint(),
        ))
        .map_err(|error| error.to_string())?;
    while receive.persisted_bytes() < total {
        let message = secure
            .transport
            .receive()
            .map_err(|error| error.to_string())?;
        if secure.progress.cancel_requested.load(Ordering::Acquire) {
            secure
                .transport
                .send(&PeerWireMessage::Cancel {
                    transfer_id: secure.session.manifest.transfer_id.clone(),
                    reason: TransferCancelReason::User,
                })
                .map_err(|error| error.to_string())?;
            return Err("nearby transfer was cancelled".into());
        }
        if let PeerWireMessage::Cancel {
            transfer_id,
            reason: _,
        } = &message
        {
            if transfer_id != &secure.session.manifest.transfer_id {
                return Err("nearby peer cancelled a different transfer".into());
            }
            return Err("nearby peer cancelled the transfer".into());
        }
        let PeerWireMessage::EncryptedChunk(chunk) = message else {
            return Err("nearby peer returned an unexpected transfer message".into());
        };
        let persisted = receive
            .write_encrypted_chunk(&secure.codec, &chunk)
            .map_err(|error| error.to_string())?;
        secure
            .session
            .acknowledge(persisted)
            .map_err(|error| error.to_string())?;
        secure
            .progress
            .transferred_bytes
            .store(persisted, Ordering::Release);
        secure
            .transport
            .send(&PeerWireMessage::DurableAck {
                transfer_id: secure.session.manifest.transfer_id.clone(),
                persisted_bytes: persisted,
            })
            .map_err(|error| error.to_string())?;
    }
    let asset = secure
        .session
        .finalize_indexed_original(receive)
        .map_err(|error| error.to_string())?;
    if asset.state != camera_core::AssetState::Finalized {
        return Err("received nearby asset was not finalized".into());
    }
    secure
        .transport
        .send(&PeerWireMessage::TransferFinalized {
            transfer_id: secure.session.manifest.transfer_id.clone(),
            sha256_hex: secure.session.manifest.sha256_hex.clone(),
        })
        .map_err(|error| error.to_string())?;
    Ok(total)
}

impl Drop for NearbyDiscoveryState {
    fn drop(&mut self) {
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        if let Ok(runtime) = self.runtime.get_mut()
            && let Some(apple) = runtime.apple.take()
        {
            shutdown_apple(apple);
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn shutdown_apple(apple: AppleDiscoveryRuntime) {
    let _ = apple.daemon.stop_browse(NEARBY_SERVICE_TYPE);
    let _ = apple.daemon.unregister(&apple.service_fullname);
    let _ = apple.daemon.shutdown();
}

fn validate_display_label(value: Option<String>) -> Result<Option<String>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    if trimmed.chars().count() > 32 || trimmed.chars().any(char::is_control) {
        return Err("nearby display label is invalid".into());
    }
    Ok(Some(trimmed.to_owned()))
}

fn snapshot(runtime: &NearbyDiscoveryRuntime, supported: bool) -> NearbyDiscoverySnapshot {
    NearbyDiscoverySnapshot {
        supported,
        active: runtime.local_peer.is_some(),
        local_peer: runtime.local_peer.clone(),
        peers: runtime.peers.values().cloned().collect(),
        last_error: runtime.last_error.clone(),
        approval: runtime
            .approval
            .as_ref()
            .map(|prepared| NearbyApprovalSnapshot {
                invitation_id: prepared.session.invitation.invitation_id.clone(),
                peer_ephemeral_id: prepared.peer_ephemeral_id.clone(),
                asset_id: prepared.asset_id.clone(),
                file_name: prepared.session.manifest.file_name.clone(),
                byte_length: prepared.session.manifest.byte_length,
                confirmation_code: prepared.session.invitation.confirmation_code.clone(),
                expires_at_unix_ms: prepared.session.invitation.expires_at_unix_ms,
                local_approved: matches!(
                    prepared.session.state,
                    TransferState::Negotiating | TransferState::Transferring { .. }
                ),
                remote_approved: prepared.remote_approved,
                direction: prepared.direction,
                transferred_bytes: prepared
                    .progress
                    .transferred_bytes
                    .load(Ordering::Acquire)
                    .max(prepared.transferred_bytes),
                transfer_active: prepared.progress.active.load(Ordering::Acquire),
                cancel_requested: prepared.progress.cancel_requested.load(Ordering::Acquire),
                retry_available: prepared.retry_available,
                failure_kind: prepared.failure_kind,
                finalized: prepared.finalized,
            }),
    }
}

fn local_capabilities() -> PeerCapabilities {
    PeerCapabilities {
        protocol_version: PROTOCOL_VERSION,
        transports: vec![TransportKind::LocalNetwork],
        max_chunk_bytes: MAX_CHUNK_BYTES,
        supports_resume: true,
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn complete_incoming_approval(runtime: &mut NearbyDiscoveryRuntime) -> Result<(), String> {
    let mut prepared = runtime
        .approval
        .take()
        .ok_or_else(|| "no incoming approval is prepared".to_owned())?;
    let mut transport = prepared
        .transport
        .take()
        .ok_or_else(|| "incoming nearby transport is unavailable".to_owned())?;
    let key_pair = runtime
        .apple
        .as_ref()
        .and_then(|apple| apple.key_pair.as_ref())
        .ok_or_else(|| "nearby session key is unavailable".to_owned())?;
    let remote_approval = HandshakeApproval {
        invitation_id: prepared.offer.invitation.invitation_id.clone(),
        transfer_id: prepared.offer.manifest.transfer_id.clone(),
        confirmation_code: prepared.offer.invitation.confirmation_code.clone(),
        approver_public_key: key_pair.public_key(),
        approver_capabilities: local_capabilities(),
        approved: true,
    };
    transport
        .send(&PeerWireMessage::HandshakeApproval(remote_approval.clone()))
        .map_err(|error| error.to_string())?;
    let (session, codec) = complete_mutual_handshake(
        key_pair,
        &prepared.offer,
        &remote_approval,
        prepared.session,
    )
    .map_err(|error| error.to_string())?;
    transport
        .set_timeouts(std::time::Duration::from_secs(125))
        .map_err(|error| error.to_string())?;
    prepared.session = session.clone();
    prepared.remote_approved = true;
    prepared.retry_available = false;
    prepared.failure_kind = None;
    runtime.secure_session = Some(SecureNearbySession {
        session,
        codec,
        transport,
        offer: prepared.offer.clone(),
        direction: NearbyApprovalDirection::Incoming,
        source_path: None,
        progress: Arc::clone(&prepared.progress),
    });
    runtime.approval = Some(prepared);
    Ok(())
}

fn decode_public_key(value: &str) -> Result<EphemeralPublicKey, String> {
    if value.len() != 64 {
        return Err("nearby public key has an invalid length".into());
    }
    let mut bytes = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let text =
            std::str::from_utf8(chunk).map_err(|_| "nearby public key is not hexadecimal")?;
        bytes[index] =
            u8::from_str_radix(text, 16).map_err(|_| "nearby public key is not hexadecimal")?;
    }
    Ok(EphemeralPublicKey { bytes })
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn poll_apple_events(runtime: &mut NearbyDiscoveryRuntime) {
    let Some(apple) = runtime.apple.as_ref() else {
        return;
    };
    while let Ok(event) = apple.monitor.try_recv() {
        if let mdns_sd::DaemonEvent::Error(error) = event {
            runtime.last_error = Some(error.to_string());
        }
    }
    while let Ok(event) = apple.events.try_recv() {
        match event {
            mdns_sd::ServiceEvent::ServiceResolved(service) => {
                let Some(peer) = resolved_peer(&service) else {
                    continue;
                };
                if runtime
                    .local_peer
                    .as_ref()
                    .is_some_and(|local| local.ephemeral_id == peer.ephemeral_id)
                {
                    continue;
                }
                runtime
                    .peers
                    .insert(service.get_fullname().to_owned(), peer);
            }
            mdns_sd::ServiceEvent::ServiceRemoved(_, fullname) => {
                runtime.peers.remove(&fullname);
            }
            _ => {}
        }
    }
    let can_accept_offer = runtime.secure_session.is_none()
        && (runtime.approval.is_none()
            || runtime.approval.as_ref().is_some_and(|prepared| {
                prepared.direction == NearbyApprovalDirection::Incoming
                    && prepared.retry_available
                    && !prepared.progress.active.load(Ordering::Acquire)
            }));
    if can_accept_offer {
        if let Err(error) = poll_incoming_offer(runtime) {
            runtime.last_error = Some(error);
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn poll_incoming_offer(runtime: &mut NearbyDiscoveryRuntime) -> Result<(), String> {
    let accepted = match runtime
        .apple
        .as_ref()
        .ok_or("nearby discovery is not active")?
        .listener
        .accept()
    {
        Ok(value) => value,
        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
        Err(error) => return Err(format!("failed to accept nearby connection: {error}")),
    };
    let mut transport =
        LocalNetworkTransport::from_stream(accepted.0).map_err(|error| error.to_string())?;
    transport
        .set_timeouts(std::time::Duration::from_secs(2))
        .map_err(|error| error.to_string())?;
    let PeerWireMessage::HandshakeOffer(offer) =
        transport.receive().map_err(|error| error.to_string())?
    else {
        return Err("incoming nearby connection did not start with a handshake offer".into());
    };
    let sender_key_hex = encode_hex(&offer.sender_public_key.bytes);
    let peer = runtime
        .peers
        .values()
        .find(|peer| {
            peer.ephemeral_id == offer.invitation.sender.ephemeral_id
                && peer.public_key_hex == sender_key_hex
        })
        .cloned()
        .ok_or_else(|| "incoming offer sender is not a currently discovered peer".to_owned())?;
    let key_pair = runtime
        .apple
        .as_ref()
        .and_then(|apple| apple.key_pair.as_ref())
        .ok_or_else(|| "nearby session key is unavailable".to_owned())?;
    let expected_code = key_pair
        .confirmation_code(
            offer.sender_public_key,
            &offer.invitation.invitation_id,
            &offer.invitation.sender.ephemeral_id,
            &offer.manifest,
        )
        .map_err(|error| error.to_string())?;
    if expected_code != offer.invitation.confirmation_code {
        return Err("incoming offer confirmation transcript does not match".into());
    }
    if runtime.approval.as_ref().is_some_and(|prepared| {
        prepared.direction == NearbyApprovalDirection::Incoming
            && prepared.retry_available
            && prepared.offer == offer
    }) {
        let mut session = TransferSession::new(offer.invitation.clone(), offer.manifest.clone())
            .map_err(|error| error.to_string())?;
        session
            .approve(&offer.invitation.confirmation_code, current_unix_ms()?)
            .map_err(|error| error.to_string())?;
        let remote_approval = HandshakeApproval {
            invitation_id: offer.invitation.invitation_id.clone(),
            transfer_id: offer.manifest.transfer_id.clone(),
            confirmation_code: offer.invitation.confirmation_code.clone(),
            approver_public_key: key_pair.public_key(),
            approver_capabilities: local_capabilities(),
            approved: true,
        };
        transport
            .send(&PeerWireMessage::HandshakeApproval(remote_approval.clone()))
            .map_err(|error| error.to_string())?;
        let (session, codec) =
            complete_mutual_handshake(key_pair, &offer, &remote_approval, session)
                .map_err(|error| error.to_string())?;
        transport
            .set_timeouts(std::time::Duration::from_secs(125))
            .map_err(|error| error.to_string())?;
        let prepared = runtime
            .approval
            .as_mut()
            .ok_or_else(|| "incoming retry approval disappeared".to_owned())?;
        prepared.session = session.clone();
        prepared.retry_available = false;
        prepared.failure_kind = None;
        prepared
            .progress
            .cancel_requested
            .store(false, Ordering::Release);
        let progress = Arc::clone(&prepared.progress);
        runtime.secure_session = Some(SecureNearbySession {
            session,
            codec,
            transport,
            offer,
            direction: NearbyApprovalDirection::Incoming,
            source_path: None,
            progress,
        });
        runtime.last_error = None;
        return Ok(());
    }
    let session = TransferSession::new(offer.invitation.clone(), offer.manifest.clone())
        .map_err(|error| error.to_string())?;
    runtime.approval = Some(PreparedApproval {
        peer_ephemeral_id: peer.ephemeral_id,
        asset_id: offer.manifest.transfer_id.clone(),
        session,
        offer,
        direction: NearbyApprovalDirection::Incoming,
        transport: Some(transport),
        remote_approved: false,
        source_path: None,
        transferred_bytes: 0,
        progress: Arc::new(TransferProgress::default()),
        retry_available: false,
        failure_kind: None,
        finalized: false,
    });
    Ok(())
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
fn resolved_peer(service: &mdns_sd::ResolvedService) -> Option<DiscoveredNearbyPeer> {
    let ephemeral_id = service.get_property_val_str("peer")?.to_owned();
    let public_key_hex = service.get_property_val_str("pk")?.to_owned();
    let protocol_version = service.get_property_val_str("v")?.parse().ok()?;
    if ephemeral_id.len() != 12
        || !ephemeral_id.bytes().all(|byte| byte.is_ascii_hexdigit())
        || public_key_hex.len() != 64
        || !public_key_hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !public_key_hex.starts_with(&ephemeral_id)
        || protocol_version != PROTOCOL_VERSION
        || service.get_port() == 0
    {
        return None;
    }
    let mut addresses = service
        .get_addresses()
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    addresses.sort();
    addresses.dedup();
    if addresses.is_empty() {
        return None;
    }
    Some(DiscoveredNearbyPeer {
        ephemeral_id,
        display_label: service.get_property_val_str("label").and_then(|label| {
            validate_display_label(Some(label.to_owned()))
                .ok()
                .flatten()
        }),
        public_key_hex,
        host: service.get_hostname().to_owned(),
        addresses,
        port: service.get_port(),
        protocol_version,
    })
}

fn encode_hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_label_is_trimmed_and_bounded() {
        assert_eq!(
            validate_display_label(Some("  Nearby Camera  ".into())).unwrap(),
            Some("Nearby Camera".into())
        );
        assert!(validate_display_label(Some("x".repeat(33))).is_err());
        assert!(validate_display_label(Some("bad\nlabel".into())).is_err());
    }

    #[test]
    fn hex_identity_encoding_is_stable() {
        assert_eq!(encode_hex(&[0, 1, 0xab, 0xff]), "0001abff");
    }

    #[test]
    fn public_key_decoder_requires_exact_hex() {
        let value = "ab".repeat(32);
        assert_eq!(decode_public_key(&value).unwrap().bytes, [0xab; 32]);
        assert!(decode_public_key(&"ab".repeat(31)).is_err());
        assert!(decode_public_key(&format!("{}zz", "ab".repeat(31))).is_err());
    }

    #[test]
    fn transfer_failures_expose_only_safe_retry_categories() {
        for (message, expected, retryable) in [
            (
                "Io(\"connection reset by peer\")",
                NearbyTransferFailureKind::Disconnected,
                true,
            ),
            (
                "Io(\"operation timed out\")",
                NearbyTransferFailureKind::Timeout,
                true,
            ),
            (
                "ResumePrefixMismatch",
                NearbyTransferFailureKind::Integrity,
                false,
            ),
            (
                "InsufficientStorage",
                NearbyTransferFailureKind::Storage,
                false,
            ),
            (
                "InvitationExpired",
                NearbyTransferFailureKind::InvitationExpired,
                false,
            ),
            (
                "nearby transfer was cancelled",
                NearbyTransferFailureKind::Cancelled,
                false,
            ),
            (
                "unexpected control message",
                NearbyTransferFailureKind::Protocol,
                false,
            ),
        ] {
            let kind = classify_transfer_failure(message);
            assert_eq!(kind, expected);
            assert_eq!(retryable_failure(kind), retryable);
        }
    }
}
