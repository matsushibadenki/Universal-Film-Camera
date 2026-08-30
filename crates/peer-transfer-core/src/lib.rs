//! Transport-independent contract for nearby media transfer.
//!
//! Platform adapters discover peers and move encrypted bytes. This crate owns
//! privacy-preserving session identity, negotiation, manifest validation and
//! the state transitions required before an asset may become visible.

use serde::{Deserialize, Serialize};
use std::fmt;

pub const PROTOCOL_VERSION: u16 = 1;
pub const MIN_CHUNK_BYTES: u32 = 16 * 1024;
pub const MAX_CHUNK_BYTES: u32 = 4 * 1024 * 1024;
pub const MAX_ASSET_BYTES: u64 = 100 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerIdentity {
    /// Random identifier regenerated for each visibility session.
    pub ephemeral_id: String,
    /// Optional user-chosen nearby label. Never substitute a device name.
    pub display_label: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    BleControl,
    LocalNetwork,
    PeerToPeerWifi,
    WifiDirect,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerCapabilities {
    pub protocol_version: u16,
    pub transports: Vec<TransportKind>,
    pub max_chunk_bytes: u32,
    pub supports_resume: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Invitation {
    pub invitation_id: String,
    pub sender: PeerIdentity,
    /// Six decimal digits compared on both devices before transport upgrade.
    pub confirmation_code: String,
    pub expires_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedMediaType {
    Photo,
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetadataPolicy {
    StripLocation,
    StripDeviceAndLocation,
    Preserve,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferManifest {
    pub schema_version: u16,
    pub transfer_id: String,
    pub media_type: SharedMediaType,
    /// A basename only. Receivers choose their own incomplete destination.
    pub file_name: String,
    pub byte_length: u64,
    pub chunk_size: u32,
    pub sha256_hex: String,
    pub metadata_policy: MetadataPolicy,
}

impl TransferManifest {
    pub fn validate(&self) -> Result<(), TransferError> {
        if self.schema_version != PROTOCOL_VERSION {
            return Err(TransferError::UnsupportedVersion(self.schema_version));
        }
        if self.transfer_id.trim().is_empty() {
            return Err(TransferError::InvalidManifest("transfer_id is empty"));
        }
        if self.file_name.is_empty()
            || self.file_name == "."
            || self.file_name == ".."
            || self.file_name.contains('/')
            || self.file_name.contains('\\')
        {
            return Err(TransferError::UnsafeFileName);
        }
        if self.byte_length == 0 || self.byte_length > MAX_ASSET_BYTES {
            return Err(TransferError::InvalidManifest(
                "byte_length is outside policy",
            ));
        }
        if !(MIN_CHUNK_BYTES..=MAX_CHUNK_BYTES).contains(&self.chunk_size) {
            return Err(TransferError::InvalidManifest(
                "chunk_size is outside policy",
            ));
        }
        if self.sha256_hex.len() != 64
            || !self.sha256_hex.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(TransferError::InvalidManifest("sha256_hex is invalid"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TransferState {
    AwaitingApproval,
    Negotiating,
    Transferring { acknowledged_bytes: u64 },
    Verifying,
    Finalized,
    Cancelled,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferSession {
    pub invitation: Invitation,
    pub manifest: TransferManifest,
    pub state: TransferState,
    pub negotiated_transport: Option<TransportKind>,
    pub negotiated_chunk_bytes: Option<u32>,
    pub negotiated_resume: bool,
}

impl TransferSession {
    pub fn new(invitation: Invitation, manifest: TransferManifest) -> Result<Self, TransferError> {
        validate_invitation(&invitation)?;
        manifest.validate()?;
        Ok(Self {
            invitation,
            manifest,
            state: TransferState::AwaitingApproval,
            negotiated_transport: None,
            negotiated_chunk_bytes: None,
            negotiated_resume: false,
        })
    }

    pub fn approve(
        &mut self,
        confirmation_code: &str,
        now_unix_ms: u64,
    ) -> Result<(), TransferError> {
        self.require_state(|state| matches!(state, TransferState::AwaitingApproval))?;
        if now_unix_ms > self.invitation.expires_at_unix_ms {
            return Err(TransferError::InvitationExpired);
        }
        if confirmation_code != self.invitation.confirmation_code {
            return Err(TransferError::ConfirmationMismatch);
        }
        self.state = TransferState::Negotiating;
        Ok(())
    }

    pub fn negotiate(
        &mut self,
        local: &PeerCapabilities,
        remote: &PeerCapabilities,
    ) -> Result<(), TransferError> {
        self.require_state(|state| matches!(state, TransferState::Negotiating))?;
        if local.protocol_version != PROTOCOL_VERSION || remote.protocol_version != PROTOCOL_VERSION
        {
            return Err(TransferError::NoCompatibleProtocol);
        }
        let transport = local
            .transports
            .iter()
            .copied()
            .find(|candidate| {
                *candidate != TransportKind::BleControl && remote.transports.contains(candidate)
            })
            .ok_or(TransferError::NoHighBandwidthTransport)?;
        let chunk_bytes = self
            .manifest
            .chunk_size
            .min(local.max_chunk_bytes)
            .min(remote.max_chunk_bytes);
        if chunk_bytes < MIN_CHUNK_BYTES {
            return Err(TransferError::NoCompatibleChunkSize);
        }
        self.negotiated_transport = Some(transport);
        self.negotiated_chunk_bytes = Some(chunk_bytes);
        self.negotiated_resume = local.supports_resume && remote.supports_resume;
        self.state = TransferState::Transferring {
            acknowledged_bytes: 0,
        };
        Ok(())
    }

    pub fn acknowledge(&mut self, contiguous_bytes: u64) -> Result<(), TransferError> {
        let TransferState::Transferring { acknowledged_bytes } = &mut self.state else {
            return Err(TransferError::InvalidState);
        };
        if contiguous_bytes < *acknowledged_bytes || contiguous_bytes > self.manifest.byte_length {
            return Err(TransferError::InvalidAcknowledgement);
        }
        *acknowledged_bytes = contiguous_bytes;
        if contiguous_bytes == self.manifest.byte_length {
            self.state = TransferState::Verifying;
        }
        Ok(())
    }

    pub fn finalize(&mut self, received_bytes: u64, sha256_hex: &str) -> Result<(), TransferError> {
        self.require_state(|state| matches!(state, TransferState::Verifying))?;
        if received_bytes != self.manifest.byte_length
            || !sha256_hex.eq_ignore_ascii_case(&self.manifest.sha256_hex)
        {
            self.state = TransferState::Failed {
                reason: "content verification failed".into(),
            };
            return Err(TransferError::ContentVerificationFailed);
        }
        self.state = TransferState::Finalized;
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), TransferError> {
        self.require_state(|state| {
            !matches!(state, TransferState::Finalized | TransferState::Cancelled)
        })?;
        self.state = TransferState::Cancelled;
        Ok(())
    }

    fn require_state(
        &self,
        predicate: impl FnOnce(&TransferState) -> bool,
    ) -> Result<(), TransferError> {
        if predicate(&self.state) {
            Ok(())
        } else {
            Err(TransferError::InvalidState)
        }
    }
}

fn validate_invitation(invitation: &Invitation) -> Result<(), TransferError> {
    if invitation.invitation_id.trim().is_empty()
        || invitation.sender.ephemeral_id.trim().is_empty()
    {
        return Err(TransferError::InvalidInvitation);
    }
    if invitation.confirmation_code.len() != 6
        || !invitation
            .confirmation_code
            .bytes()
            .all(|byte| byte.is_ascii_digit())
    {
        return Err(TransferError::InvalidInvitation);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferError {
    UnsupportedVersion(u16),
    InvalidManifest(&'static str),
    InvalidInvitation,
    UnsafeFileName,
    InvitationExpired,
    ConfirmationMismatch,
    NoCompatibleProtocol,
    NoHighBandwidthTransport,
    NoCompatibleChunkSize,
    InvalidAcknowledgement,
    ContentVerificationFailed,
    InvalidState,
}

impl fmt::Display for TransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TransferError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> TransferSession {
        TransferSession::new(
            Invitation {
                invitation_id: "invite-1".into(),
                sender: PeerIdentity {
                    ephemeral_id: "session-peer-a".into(),
                    display_label: None,
                },
                confirmation_code: "482913".into(),
                expires_at_unix_ms: 10_000,
            },
            TransferManifest {
                schema_version: PROTOCOL_VERSION,
                transfer_id: "transfer-1".into(),
                media_type: SharedMediaType::Photo,
                file_name: "UFC-photo.jpg".into(),
                byte_length: 100_000,
                chunk_size: MIN_CHUNK_BYTES,
                sha256_hex: "ab".repeat(32),
                metadata_policy: MetadataPolicy::StripLocation,
            },
        )
        .unwrap()
    }

    fn capabilities(transports: Vec<TransportKind>) -> PeerCapabilities {
        PeerCapabilities {
            protocol_version: PROTOCOL_VERSION,
            transports,
            max_chunk_bytes: 256 * 1024,
            supports_resume: true,
        }
    }

    #[test]
    fn transfer_requires_approval_transport_and_content_verification() {
        let mut transfer = session();
        transfer.approve("482913", 5_000).unwrap();
        let peers = capabilities(vec![TransportKind::BleControl, TransportKind::LocalNetwork]);
        transfer.negotiate(&peers, &peers).unwrap();
        transfer.acknowledge(50_000).unwrap();
        transfer.acknowledge(100_000).unwrap();
        transfer.finalize(100_000, &"AB".repeat(32)).unwrap();
        assert_eq!(transfer.state, TransferState::Finalized);
    }

    #[test]
    fn manifest_rejects_path_traversal() {
        let mut transfer = session();
        transfer.manifest.file_name = "../escape.jpg".into();
        assert_eq!(
            transfer.manifest.validate(),
            Err(TransferError::UnsafeFileName)
        );
    }

    #[test]
    fn ble_only_peers_cannot_start_asset_transport() {
        let mut transfer = session();
        transfer.approve("482913", 5_000).unwrap();
        let peers = capabilities(vec![TransportKind::BleControl]);
        assert_eq!(
            transfer.negotiate(&peers, &peers),
            Err(TransferError::NoHighBandwidthTransport)
        );
    }

    #[test]
    fn acknowledgement_cannot_skip_past_asset_or_move_backward() {
        let mut transfer = session();
        transfer.approve("482913", 5_000).unwrap();
        let peers = capabilities(vec![TransportKind::LocalNetwork]);
        transfer.negotiate(&peers, &peers).unwrap();
        transfer.acknowledge(50_000).unwrap();
        assert_eq!(
            transfer.acknowledge(49_999),
            Err(TransferError::InvalidAcknowledgement)
        );
        assert_eq!(
            transfer.acknowledge(100_001),
            Err(TransferError::InvalidAcknowledgement)
        );
    }

    #[test]
    fn hash_mismatch_fails_instead_of_publishing() {
        let mut transfer = session();
        transfer.approve("482913", 5_000).unwrap();
        let peers = capabilities(vec![TransportKind::LocalNetwork]);
        transfer.negotiate(&peers, &peers).unwrap();
        transfer.acknowledge(100_000).unwrap();
        assert_eq!(
            transfer.finalize(100_000, &"cd".repeat(32)),
            Err(TransferError::ContentVerificationFailed)
        );
        assert!(matches!(transfer.state, TransferState::Failed { .. }));
    }
}
