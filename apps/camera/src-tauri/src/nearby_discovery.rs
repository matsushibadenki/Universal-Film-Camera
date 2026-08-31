use camera_core::CapturedAsset;
use peer_transfer_core::{
    AssetSelection, AssetTransferManifest, EphemeralKeyPair, EphemeralPublicKey, Invitation,
    MetadataPolicy, PROTOCOL_VERSION, PeerIdentity, TransferSession, TransferState,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Mutex,
};

pub const NEARBY_SERVICE_TYPE: &str = "_ufcamera._tcp.local.";

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
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    apple: Option<AppleDiscoveryRuntime>,
}

struct PreparedApproval {
    peer_ephemeral_id: String,
    asset_id: String,
    session: TransferSession,
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
struct AppleDiscoveryRuntime {
    daemon: mdns_sd::ServiceDaemon,
    events: mdns_sd::Receiver<mdns_sd::ServiceEvent>,
    monitor: mdns_sd::Receiver<mdns_sd::DaemonEvent>,
    service_fullname: String,
    _listener: std::net::TcpListener,
    _key_pair: EphemeralKeyPair,
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
            runtime.approval = None;
            runtime.apple = Some(AppleDiscoveryRuntime {
                daemon,
                events,
                monitor,
                service_fullname,
                _listener: listener,
                _key_pair: key_pair,
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
        let manifest = bundle
            .resources
            .first()
            .ok_or_else(|| "transfer manifest has no original resource".to_owned())?
            .manifest
            .clone();
        let peer_public = decode_public_key(&peer.public_key_hex)?;
        let code = apple
            ._key_pair
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
        let session =
            TransferSession::new(invitation, manifest).map_err(|error| error.to_string())?;
        runtime.approval = Some(PreparedApproval {
            peer_ephemeral_id: peer.ephemeral_id,
            asset_id: asset.id.clone(),
            session,
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
        Ok(snapshot(&runtime, true))
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
                local_approved: matches!(prepared.session.state, TransferState::Negotiating),
            }),
    }
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
}
