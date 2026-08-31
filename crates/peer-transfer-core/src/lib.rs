//! Transport-independent contract for nearby media transfer.
//!
//! Platform adapters discover peers and move encrypted bytes. This crate owns
//! privacy-preserving session identity, negotiation, manifest validation and
//! the state transitions required before an asset may become visible.

use camera_core::{
    CaptureMetadata, CapturedAsset, CapturedMediaType, DerivativeProvenance, DerivativePurpose,
    MediaIndex, probe_media_resource,
};
use chacha20poly1305::{
    ChaCha20Poly1305, Key, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use hkdf::Hkdf;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    net::TcpStream,
    path::{Path, PathBuf},
};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};
use zeroize::Zeroizing;

pub const PROTOCOL_VERSION: u16 = 1;
pub const MIN_CHUNK_BYTES: u32 = 16 * 1024;
pub const MAX_CHUNK_BYTES: u32 = 4 * 1024 * 1024;
pub const MAX_ASSET_BYTES: u64 = 100 * 1024 * 1024 * 1024;
pub const DEFAULT_RECEIVE_RESERVE_BYTES: u64 = 256 * 1024 * 1024;
const WIRE_MAGIC: [u8; 4] = *b"UFC1";
const MAX_WIRE_FRAME_BYTES: usize = MAX_CHUNK_BYTES as usize + 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EphemeralPublicKey {
    pub bytes: [u8; 32],
}

pub struct EphemeralKeyPair {
    secret: StaticSecret,
    public: EphemeralPublicKey,
}

pub struct AgreedSessionSecrets {
    chunk_key: Zeroizing<[u8; 32]>,
    nonce_prefix: [u8; 16],
}

impl EphemeralKeyPair {
    pub fn generate() -> Result<Self, TransferError> {
        let mut secret = Zeroizing::new([0_u8; 32]);
        getrandom::getrandom(secret.as_mut()).map_err(|_| TransferError::SecureRandomFailed)?;
        Self::from_secret_bytes(*secret)
    }

    /// Creates a one-session key pair from 32 bytes supplied by a platform CSPRNG.
    pub fn from_secret_bytes(secret_bytes: [u8; 32]) -> Result<Self, TransferError> {
        if secret_bytes.iter().all(|byte| *byte == 0) {
            return Err(TransferError::InvalidEphemeralKey);
        }
        let secret = StaticSecret::from(secret_bytes);
        let public = EphemeralPublicKey {
            bytes: X25519PublicKey::from(&secret).to_bytes(),
        };
        Ok(Self { secret, public })
    }

    pub fn public_key(&self) -> EphemeralPublicKey {
        self.public
    }

    /// Six digits shown on both peers after public-key exchange.
    pub fn confirmation_code(
        &self,
        peer_public: EphemeralPublicKey,
        invitation_id: &str,
        sender_ephemeral_id: &str,
        manifest: &TransferManifest,
    ) -> Result<String, TransferError> {
        manifest.validate()?;
        if !safe_token(invitation_id)
            || !safe_token(sender_ephemeral_id)
            || peer_public.bytes == self.public.bytes
            || peer_public.bytes.iter().all(|byte| *byte == 0)
        {
            return Err(TransferError::InvalidEphemeralKey);
        }
        let shared = self
            .secret
            .diffie_hellman(&X25519PublicKey::from(peer_public.bytes));
        if shared.as_bytes().iter().all(|byte| *byte == 0) {
            return Err(TransferError::InvalidEphemeralKey);
        }
        let mut ordered_public_keys = [self.public.bytes, peer_public.bytes];
        ordered_public_keys.sort();
        let mut hasher = Sha256::new();
        hasher.update(b"ufc-peer-confirmation-v1");
        hasher.update(shared.as_bytes());
        hasher.update(ordered_public_keys[0]);
        hasher.update(ordered_public_keys[1]);
        hasher.update(invitation_id.as_bytes());
        hasher.update(sender_ephemeral_id.as_bytes());
        hasher.update(manifest.transfer_id.as_bytes());
        hasher.update(manifest.sha256_hex.as_bytes());
        hasher.update(manifest.byte_length.to_be_bytes());
        let digest = hasher.finalize();
        let value = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) % 1_000_000;
        Ok(format!("{value:06}"))
    }

    pub fn agree(
        self,
        peer_public: EphemeralPublicKey,
        invitation: &Invitation,
        confirmation_code: &str,
        manifest: &TransferManifest,
    ) -> Result<AgreedSessionSecrets, TransferError> {
        validate_invitation(invitation)?;
        manifest.validate()?;
        let expected_confirmation = self.confirmation_code(
            peer_public,
            &invitation.invitation_id,
            &invitation.sender.ephemeral_id,
            manifest,
        )?;
        if confirmation_code != expected_confirmation
            || confirmation_code != invitation.confirmation_code
            || confirmation_code.len() != 6
            || !confirmation_code.bytes().all(|byte| byte.is_ascii_digit())
            || peer_public.bytes == self.public.bytes
            || peer_public.bytes.iter().all(|byte| *byte == 0)
        {
            return Err(TransferError::KeyAgreementContextMismatch);
        }
        let shared = self
            .secret
            .diffie_hellman(&X25519PublicKey::from(peer_public.bytes));
        if shared.as_bytes().iter().all(|byte| *byte == 0) {
            return Err(TransferError::InvalidEphemeralKey);
        }
        let mut ordered_public_keys = [self.public.bytes, peer_public.bytes];
        ordered_public_keys.sort();
        let mut salt_hasher = Sha256::new();
        salt_hasher.update(b"ufc-peer-handshake-salt-v1");
        salt_hasher.update(invitation.invitation_id.as_bytes());
        salt_hasher.update(invitation.sender.ephemeral_id.as_bytes());
        salt_hasher.update(confirmation_code.as_bytes());
        let salt = salt_hasher.finalize();
        let hkdf = Hkdf::<Sha256>::new(Some(&salt), shared.as_bytes());
        let mut info = Vec::with_capacity(180);
        info.extend_from_slice(b"ufc-peer-session-v1");
        info.extend_from_slice(&ordered_public_keys[0]);
        info.extend_from_slice(&ordered_public_keys[1]);
        info.extend_from_slice(manifest.transfer_id.as_bytes());
        info.extend_from_slice(manifest.sha256_hex.as_bytes());
        info.extend_from_slice(&manifest.byte_length.to_be_bytes());
        let mut output = Zeroizing::new([0_u8; 48]);
        hkdf.expand(&info, output.as_mut())
            .map_err(|_| TransferError::KeyDerivationFailed)?;
        let mut chunk_key = [0_u8; 32];
        chunk_key.copy_from_slice(&output[..32]);
        let mut nonce_prefix = [0_u8; 16];
        nonce_prefix.copy_from_slice(&output[32..]);
        Ok(AgreedSessionSecrets {
            chunk_key: Zeroizing::new(chunk_key),
            nonce_prefix,
        })
    }
}

impl AgreedSessionSecrets {
    pub fn into_chunk_codec(
        self,
        manifest: &TransferManifest,
    ) -> Result<EncryptedChunkCodec, TransferError> {
        EncryptedChunkCodec::new(*self.chunk_key, self.nonce_prefix, manifest)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EncryptedChunk {
    pub transfer_id: String,
    pub offset: u64,
    pub plaintext_bytes: u32,
    pub nonce: [u8; 12],
    pub ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeCheckpoint {
    pub transfer_id: String,
    pub persisted_bytes: u64,
    pub prefix_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PeerWireMessage {
    EncryptedChunk(EncryptedChunk),
    ResumeCheckpoint(ResumeCheckpoint),
    DurableAck {
        transfer_id: String,
        persisted_bytes: u64,
    },
    Cancel {
        transfer_id: String,
        reason: TransferCancelReason,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferCancelReason {
    User,
    Timeout,
    PeerDisconnected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportLifecycleState {
    Transferring,
    AwaitingAck { end_offset: u64 },
    Cancelled { reason: TransferCancelReason },
    Complete,
}

pub struct LocalNetworkTransport {
    stream: TcpStream,
}

impl LocalNetworkTransport {
    pub fn connect(address: std::net::SocketAddr) -> Result<Self, TransferError> {
        let stream = TcpStream::connect(address).map_err(io_error)?;
        Self::from_stream(stream)
    }

    pub fn from_stream(stream: TcpStream) -> Result<Self, TransferError> {
        stream.set_nodelay(true).map_err(io_error)?;
        Ok(Self { stream })
    }

    pub fn set_timeouts(&self, timeout: std::time::Duration) -> Result<(), TransferError> {
        if timeout.is_zero() {
            return Err(TransferError::InvalidTransportTimeout);
        }
        self.stream
            .set_read_timeout(Some(timeout))
            .map_err(io_error)?;
        self.stream
            .set_write_timeout(Some(timeout))
            .map_err(io_error)
    }

    pub fn send(&mut self, message: &PeerWireMessage) -> Result<(), TransferError> {
        write_wire_message(&mut self.stream, message)
    }

    pub fn receive(&mut self) -> Result<PeerWireMessage, TransferError> {
        read_wire_message(&mut self.stream)
    }
}

pub struct EncryptedTransferSender {
    source_path: PathBuf,
    source: File,
    manifest: TransferManifest,
    codec: EncryptedChunkCodec,
    chunk_bytes: u32,
    acknowledged_bytes: u64,
    state: TransportLifecycleState,
}

pub struct EncryptedTransferReceiver {
    writer: ReceiveWriter,
    codec: EncryptedChunkCodec,
    state: TransportLifecycleState,
}

pub struct EncryptedChunkCodec {
    key: Zeroizing<[u8; 32]>,
    nonce_prefix: [u8; 16],
    transfer_id: String,
    total_plaintext_bytes: u64,
}

impl EncryptedChunkCodec {
    pub fn new(
        key: [u8; 32],
        nonce_prefix: [u8; 16],
        manifest: &TransferManifest,
    ) -> Result<Self, TransferError> {
        manifest.validate()?;
        Ok(Self {
            key: Zeroizing::new(key),
            nonce_prefix,
            transfer_id: manifest.transfer_id.clone(),
            total_plaintext_bytes: manifest.byte_length,
        })
    }

    pub fn encrypt(&self, offset: u64, plaintext: &[u8]) -> Result<EncryptedChunk, TransferError> {
        self.validate_range(offset, plaintext.len())?;
        let nonce = self.derive_nonce(offset, plaintext);
        let aad = self.aad(offset, plaintext.len() as u32);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(self.key.as_ref()));
        let ciphertext = cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: plaintext,
                    aad: &aad,
                },
            )
            .map_err(|_| TransferError::ChunkAuthenticationFailed)?;
        Ok(EncryptedChunk {
            transfer_id: self.transfer_id.clone(),
            offset,
            plaintext_bytes: plaintext.len() as u32,
            nonce,
            ciphertext,
        })
    }

    pub fn decrypt(&self, chunk: &EncryptedChunk) -> Result<Vec<u8>, TransferError> {
        if chunk.transfer_id != self.transfer_id
            || chunk.plaintext_bytes == 0
            || chunk.plaintext_bytes as usize > MAX_CHUNK_BYTES as usize
            || chunk.ciphertext.len() != chunk.plaintext_bytes as usize + 16
        {
            return Err(TransferError::InvalidEncryptedChunk);
        }
        self.validate_range(chunk.offset, chunk.plaintext_bytes as usize)?;
        let aad = self.aad(chunk.offset, chunk.plaintext_bytes);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(self.key.as_ref()));
        let plaintext = cipher
            .decrypt(
                Nonce::from_slice(&chunk.nonce),
                Payload {
                    msg: &chunk.ciphertext,
                    aad: &aad,
                },
            )
            .map_err(|_| TransferError::ChunkAuthenticationFailed)?;
        if self.derive_nonce(chunk.offset, &plaintext) != chunk.nonce {
            return Err(TransferError::ChunkAuthenticationFailed);
        }
        Ok(plaintext)
    }

    fn validate_range(&self, offset: u64, length: usize) -> Result<(), TransferError> {
        if length == 0
            || length > MAX_CHUNK_BYTES as usize
            || offset
                .checked_add(length as u64)
                .filter(|end| *end <= self.total_plaintext_bytes)
                .is_none()
        {
            return Err(TransferError::InvalidEncryptedChunk);
        }
        Ok(())
    }

    fn derive_nonce(&self, offset: u64, plaintext: &[u8]) -> [u8; 12] {
        let mut hasher = Sha256::new();
        hasher.update(b"ufc-peer-chunk-nonce-v1");
        hasher.update(self.key.as_ref());
        hasher.update(self.nonce_prefix);
        hasher.update(self.transfer_id.as_bytes());
        hasher.update(offset.to_be_bytes());
        hasher.update(Sha256::digest(plaintext));
        let digest = hasher.finalize();
        let mut nonce = [0_u8; 12];
        nonce.copy_from_slice(&digest[..12]);
        nonce
    }

    fn aad(&self, offset: u64, plaintext_bytes: u32) -> Vec<u8> {
        let mut aad = Vec::with_capacity(64 + self.transfer_id.len());
        aad.extend_from_slice(b"ufc-peer-chunk-v1");
        aad.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
        aad.extend_from_slice(&(self.transfer_id.len() as u16).to_be_bytes());
        aad.extend_from_slice(self.transfer_id.as_bytes());
        aad.extend_from_slice(&offset.to_be_bytes());
        aad.extend_from_slice(&plaintext_bytes.to_be_bytes());
        aad.extend_from_slice(&self.total_plaintext_bytes.to_be_bytes());
        aad
    }
}

impl EncryptedTransferSender {
    pub fn open(
        source_path: &Path,
        session: &TransferSession,
        codec: EncryptedChunkCodec,
    ) -> Result<Self, TransferError> {
        if !matches!(session.state, TransferState::Transferring { .. })
            || session.negotiated_transport == Some(TransportKind::BleControl)
        {
            return Err(TransferError::InvalidState);
        }
        if codec.transfer_id != session.manifest.transfer_id
            || codec.total_plaintext_bytes != session.manifest.byte_length
        {
            return Err(TransferError::ManifestMismatch);
        }
        verify_resume_checkpoint(
            source_path,
            &session.manifest,
            &ResumeCheckpoint {
                transfer_id: session.manifest.transfer_id.clone(),
                persisted_bytes: session.manifest.byte_length,
                prefix_sha256: session.manifest.sha256_hex.clone(),
            },
        )?;
        let source = File::open(source_path).map_err(io_error)?;
        Ok(Self {
            source_path: source_path.to_owned(),
            source,
            manifest: session.manifest.clone(),
            codec,
            chunk_bytes: session
                .negotiated_chunk_bytes
                .ok_or(TransferError::InvalidState)?,
            acknowledged_bytes: 0,
            state: TransportLifecycleState::Transferring,
        })
    }

    pub fn state(&self) -> TransportLifecycleState {
        self.state
    }

    pub fn next_chunk(&mut self) -> Result<Option<PeerWireMessage>, TransferError> {
        if self.acknowledged_bytes == self.manifest.byte_length {
            self.state = TransportLifecycleState::Complete;
            return Ok(None);
        }
        if self.state != TransportLifecycleState::Transferring {
            return Err(TransferError::InvalidTransportLifecycle);
        }
        self.source
            .seek(SeekFrom::Start(self.acknowledged_bytes))
            .map_err(io_error)?;
        let length = (self.manifest.byte_length - self.acknowledged_bytes)
            .min(self.chunk_bytes as u64) as usize;
        let mut plaintext = vec![0_u8; length];
        self.source.read_exact(&mut plaintext).map_err(io_error)?;
        let chunk = self.codec.encrypt(self.acknowledged_bytes, &plaintext)?;
        let end_offset = self.acknowledged_bytes + length as u64;
        self.state = TransportLifecycleState::AwaitingAck { end_offset };
        Ok(Some(PeerWireMessage::EncryptedChunk(chunk)))
    }

    pub fn accept_ack(&mut self, message: &PeerWireMessage) -> Result<u64, TransferError> {
        let TransportLifecycleState::AwaitingAck { end_offset } = self.state else {
            return Err(TransferError::InvalidTransportLifecycle);
        };
        let PeerWireMessage::DurableAck {
            transfer_id,
            persisted_bytes,
        } = message
        else {
            return Err(TransferError::InvalidWireMessage);
        };
        if transfer_id != &self.manifest.transfer_id || *persisted_bytes != end_offset {
            return Err(TransferError::InvalidAcknowledgement);
        }
        self.acknowledged_bytes = *persisted_bytes;
        self.state = if self.acknowledged_bytes == self.manifest.byte_length {
            TransportLifecycleState::Complete
        } else {
            TransportLifecycleState::Transferring
        };
        Ok(self.acknowledged_bytes)
    }

    pub fn mark_disconnected(&mut self) {
        self.state = TransportLifecycleState::Cancelled {
            reason: TransferCancelReason::PeerDisconnected,
        };
    }

    pub fn resume_from_checkpoint(
        &mut self,
        session: &TransferSession,
        checkpoint: &ResumeCheckpoint,
    ) -> Result<u64, TransferError> {
        if self.state
            != (TransportLifecycleState::Cancelled {
                reason: TransferCancelReason::PeerDisconnected,
            })
        {
            return Err(TransferError::InvalidTransportLifecycle);
        }
        let offset = session.accept_resume_checkpoint(checkpoint)?;
        verify_resume_checkpoint(&self.source_path, &self.manifest, checkpoint)?;
        self.acknowledged_bytes = offset;
        self.state = if offset == self.manifest.byte_length {
            TransportLifecycleState::Complete
        } else {
            TransportLifecycleState::Transferring
        };
        Ok(offset)
    }

    pub fn cancel(
        &mut self,
        reason: TransferCancelReason,
    ) -> Result<PeerWireMessage, TransferError> {
        if matches!(
            self.state,
            TransportLifecycleState::Complete | TransportLifecycleState::Cancelled { .. }
        ) {
            return Err(TransferError::InvalidTransportLifecycle);
        }
        self.state = TransportLifecycleState::Cancelled { reason };
        Ok(PeerWireMessage::Cancel {
            transfer_id: self.manifest.transfer_id.clone(),
            reason,
        })
    }
}

impl EncryptedTransferReceiver {
    pub fn new(
        writer: ReceiveWriter,
        session: &TransferSession,
        codec: EncryptedChunkCodec,
    ) -> Result<Self, TransferError> {
        if !matches!(session.state, TransferState::Transferring { .. })
            || writer.manifest() != &session.manifest
            || codec.transfer_id != session.manifest.transfer_id
            || codec.total_plaintext_bytes != session.manifest.byte_length
        {
            return Err(TransferError::InvalidState);
        }
        Ok(Self {
            writer,
            codec,
            state: TransportLifecycleState::Transferring,
        })
    }

    pub fn state(&self) -> TransportLifecycleState {
        self.state
    }

    pub fn resume_checkpoint(&self) -> ResumeCheckpoint {
        self.writer.resume_checkpoint()
    }

    pub fn accept(
        &mut self,
        message: &PeerWireMessage,
    ) -> Result<Option<PeerWireMessage>, TransferError> {
        if self.state != TransportLifecycleState::Transferring {
            return Err(TransferError::InvalidTransportLifecycle);
        }
        match message {
            PeerWireMessage::EncryptedChunk(chunk) => {
                let persisted_bytes = self.writer.write_encrypted_chunk(&self.codec, chunk)?;
                if persisted_bytes == self.writer.manifest.byte_length {
                    self.state = TransportLifecycleState::Complete;
                }
                Ok(Some(PeerWireMessage::DurableAck {
                    transfer_id: self.writer.manifest.transfer_id.clone(),
                    persisted_bytes,
                }))
            }
            PeerWireMessage::Cancel {
                transfer_id,
                reason,
            } if transfer_id == &self.writer.manifest.transfer_id => {
                self.state = TransportLifecycleState::Cancelled { reason: *reason };
                Ok(None)
            }
            _ => Err(TransferError::InvalidWireMessage),
        }
    }

    pub fn into_writer(self) -> Result<ReceiveWriter, TransferError> {
        if self.state != TransportLifecycleState::Complete {
            return Err(TransferError::InvalidTransportLifecycle);
        }
        Ok(self.writer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataSanitizationReport {
    pub removed_segments: u32,
    pub output_bytes: u64,
    pub sha256_hex: String,
}

/// Opaque proof that a JPEG was rewritten by the privacy sanitizer.
///
/// The path is intentionally private so callers cannot label an arbitrary
/// source file with a stripping policy through the safe manifest builder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanitizedJpeg {
    path: PathBuf,
    pub report: MetadataSanitizationReport,
}

impl SanitizedJpeg {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Rewrites a JPEG without metadata-bearing application segments.
///
/// ICC APP2 and Adobe APP14 color interpretation segments are retained. EXIF,
/// XMP, IPTC, comments and unknown APP segments are removed. Pixel entropy is
/// copied byte-for-byte after SOS.
pub fn sanitize_jpeg_for_transfer(
    source: &Path,
    destination: &Path,
    policy: MetadataPolicy,
) -> Result<SanitizedJpeg, TransferError> {
    if policy != MetadataPolicy::StripDeviceAndLocation {
        return Err(TransferError::UnsupportedMetadataPolicy);
    }
    if source == destination || destination.exists() {
        return Err(TransferError::DestinationExists);
    }
    let input = fs::read(source).map_err(io_error)?;
    let (output, removed_segments) = sanitized_jpeg_bytes(&input)?;
    let parent = destination
        .parent()
        .ok_or(TransferError::UnsafeReceivePath)?;
    fs::create_dir_all(parent).map_err(io_error)?;
    let temporary = destination.with_extension("sanitize-partial");
    reject_symlink(&temporary)?;
    {
        let mut file = File::create(&temporary).map_err(io_error)?;
        file.write_all(&output).map_err(io_error)?;
        file.sync_all().map_err(io_error)?;
    }
    fs::rename(&temporary, destination).map_err(io_error)?;
    Ok(SanitizedJpeg {
        path: destination.to_owned(),
        report: MetadataSanitizationReport {
            removed_segments,
            output_bytes: output.len() as u64,
            sha256_hex: hex::encode(Sha256::digest(&output)),
        },
    })
}

fn sanitized_jpeg_bytes(input: &[u8]) -> Result<(Vec<u8>, u32), TransferError> {
    if input.get(..2) != Some(&[0xff, 0xd8]) || !input.ends_with(&[0xff, 0xd9]) {
        return Err(TransferError::UnsupportedMediaForSanitization);
    }
    let mut output = Vec::with_capacity(input.len());
    output.extend_from_slice(&input[..2]);
    let mut cursor = 2_usize;
    let mut removed = 0_u32;
    while cursor < input.len() {
        if input[cursor] != 0xff {
            return Err(TransferError::MalformedJpeg);
        }
        let marker_start = cursor;
        while cursor < input.len() && input[cursor] == 0xff {
            cursor += 1;
        }
        let marker = *input.get(cursor).ok_or(TransferError::MalformedJpeg)?;
        cursor += 1;
        if marker == 0xda {
            output.extend_from_slice(&input[marker_start..]);
            return Ok((output, removed));
        }
        if marker == 0xd9 {
            output.extend_from_slice(&input[marker_start..cursor]);
            return Ok((output, removed));
        }
        if marker == 0x01 || (0xd0..=0xd7).contains(&marker) {
            output.extend_from_slice(&input[marker_start..cursor]);
            continue;
        }
        let length_bytes = input
            .get(cursor..cursor + 2)
            .ok_or(TransferError::MalformedJpeg)?;
        let segment_length = u16::from_be_bytes([length_bytes[0], length_bytes[1]]) as usize;
        if segment_length < 2 {
            return Err(TransferError::MalformedJpeg);
        }
        let segment_end = cursor
            .checked_add(segment_length)
            .filter(|end| *end <= input.len())
            .ok_or(TransferError::MalformedJpeg)?;
        let payload = &input[cursor + 2..segment_end];
        let is_app = (0xe0..=0xef).contains(&marker);
        let retain_standard_app0 =
            marker == 0xe0 && (payload.starts_with(b"JFIF\0") || payload.starts_with(b"JFXX\0"));
        let retain_color_segment = (marker == 0xe2 && payload.starts_with(b"ICC_PROFILE\0"))
            || (marker == 0xee && payload.starts_with(b"Adobe"));
        let strip = marker == 0xfe || (is_app && !retain_standard_app0 && !retain_color_segment);
        if strip {
            removed += 1;
        } else {
            output.extend_from_slice(&input[marker_start..segment_end]);
        }
        cursor = segment_end;
    }
    Err(TransferError::MalformedJpeg)
}

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
#[serde(tag = "selection", rename_all = "snake_case")]
pub enum AssetSelection {
    Original,
    Derivatives { resource_ids: Vec<String> },
    OriginalAndDerivatives { resource_ids: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferResource {
    pub resource_id: String,
    pub role: TransferResourceRole,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derivative_provenance: Option<DerivativeProvenance>,
    pub manifest: TransferManifest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum TransferResourceRole {
    Original,
    Derivative { purpose: DerivativePurpose },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetTransferManifest {
    pub schema_version: u16,
    pub source_asset_id: String,
    pub media_type: CapturedMediaType,
    pub capture: CaptureMetadata,
    pub selection: AssetSelection,
    pub resources: Vec<TransferResource>,
}

/// Coordinates dependency-safe receipt of an Original + Derivative bundle.
pub struct BundleReceiveCoordinator {
    bundle: AssetTransferManifest,
    local_asset_id: String,
    source_to_local: BTreeMap<String, String>,
    completed_resources: BTreeSet<String>,
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
        if !safe_media_token(&self.transfer_id) {
            return Err(TransferError::InvalidManifest("transfer_id is invalid"));
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

impl AssetTransferManifest {
    pub fn from_captured_asset(
        asset: &CapturedAsset,
        selection: AssetSelection,
        transfer_prefix: &str,
        chunk_size: u32,
        metadata_policy: MetadataPolicy,
    ) -> Result<Self, TransferError> {
        if asset.state != camera_core::AssetState::Finalized || !safe_media_token(transfer_prefix) {
            return Err(TransferError::InvalidSourceAsset);
        }
        if metadata_policy != MetadataPolicy::Preserve {
            return Err(TransferError::MetadataSanitizationRequired);
        }
        let selected_derivatives = match &selection {
            AssetSelection::Original => Vec::new(),
            AssetSelection::Derivatives { resource_ids }
            | AssetSelection::OriginalAndDerivatives { resource_ids } => {
                if resource_ids.is_empty() {
                    return Err(TransferError::InvalidSelection);
                }
                let mut unique = std::collections::BTreeSet::new();
                let mut selected = Vec::new();
                for resource_id in resource_ids {
                    if !unique.insert(resource_id) {
                        return Err(TransferError::InvalidSelection);
                    }
                    selected.push(
                        asset
                            .derivatives
                            .iter()
                            .find(|item| &item.resource_id == resource_id)
                            .ok_or(TransferError::InvalidSelection)?,
                    );
                }
                selected
            }
        };
        let include_original = matches!(
            &selection,
            AssetSelection::Original | AssetSelection::OriginalAndDerivatives { .. }
        );
        let mut resources = Vec::new();
        if include_original {
            resources.push(resource_transfer(
                &asset.original_resource_id,
                TransferResourceRole::Original,
                &asset.original.path,
                format!("{transfer_prefix}-original"),
                asset.media_type,
                chunk_size,
                metadata_policy,
                None,
            )?);
        }
        for (index, derivative) in selected_derivatives.into_iter().enumerate() {
            resources.push(resource_transfer(
                &derivative.resource_id,
                TransferResourceRole::Derivative {
                    purpose: derivative.purpose,
                },
                &derivative.resource.path,
                format!("{transfer_prefix}-derivative-{index}"),
                asset.media_type,
                chunk_size,
                metadata_policy,
                Some(derivative.provenance.clone()),
            )?);
        }
        Ok(Self {
            schema_version: PROTOCOL_VERSION,
            source_asset_id: asset.id.clone(),
            media_type: asset.media_type,
            capture: asset.capture.clone(),
            selection,
            resources,
        })
    }

    /// Builds an Original-only transfer from sanitizer-proven JPEG bytes.
    pub fn from_sanitized_jpeg_original(
        asset: &CapturedAsset,
        sanitized: &SanitizedJpeg,
        transfer_prefix: &str,
        chunk_size: u32,
    ) -> Result<Self, TransferError> {
        if asset.state != camera_core::AssetState::Finalized
            || asset.media_type != CapturedMediaType::Photo
            || !safe_media_token(transfer_prefix)
        {
            return Err(TransferError::InvalidSourceAsset);
        }
        let probed = probe_media_resource(sanitized.path(), CapturedMediaType::Photo)
            .map_err(|_| TransferError::SanitizedSourceMismatch)?;
        if probed.pixel_width != asset.original.pixel_width
            || probed.pixel_height != asset.original.pixel_height
        {
            return Err(TransferError::SanitizedSourceMismatch);
        }
        let resource = resource_transfer(
            &asset.original_resource_id,
            TransferResourceRole::Original,
            sanitized.path(),
            format!("{transfer_prefix}-original"),
            asset.media_type,
            chunk_size,
            MetadataPolicy::StripDeviceAndLocation,
            None,
        )?;
        if resource.manifest.byte_length != sanitized.report.output_bytes
            || resource.manifest.sha256_hex != sanitized.report.sha256_hex
        {
            return Err(TransferError::SanitizedSourceMismatch);
        }
        Ok(Self {
            schema_version: PROTOCOL_VERSION,
            source_asset_id: asset.id.clone(),
            media_type: asset.media_type,
            capture: asset.capture.clone(),
            selection: AssetSelection::Original,
            resources: vec![resource],
        })
    }
}

impl BundleReceiveCoordinator {
    pub fn new(
        bundle: AssetTransferManifest,
        local_asset_id: String,
    ) -> Result<Self, TransferError> {
        if bundle.schema_version != PROTOCOL_VERSION
            || !safe_media_token(&local_asset_id)
            || !matches!(
                bundle.selection,
                AssetSelection::OriginalAndDerivatives { .. }
            )
        {
            return Err(TransferError::InvalidBundle);
        }
        let mut resource_ids = BTreeSet::new();
        let mut transfer_ids = BTreeSet::new();
        let mut derivative_ids = BTreeSet::new();
        let mut derivative_parents = Vec::new();
        let mut originals = 0_usize;
        let mut derivatives = 0_usize;
        for resource in &bundle.resources {
            resource.manifest.validate()?;
            let resource_media_type = match resource.manifest.media_type {
                SharedMediaType::Photo => CapturedMediaType::Photo,
                SharedMediaType::Video => CapturedMediaType::Video,
            };
            if resource_media_type != bundle.media_type {
                return Err(TransferError::InvalidBundle);
            }
            if !resource_ids.insert(resource.resource_id.clone())
                || !transfer_ids.insert(resource.manifest.transfer_id.clone())
            {
                return Err(TransferError::InvalidBundle);
            }
            match resource.role {
                TransferResourceRole::Original => {
                    originals += 1;
                    if resource.derivative_provenance.is_some() {
                        return Err(TransferError::InvalidBundle);
                    }
                }
                TransferResourceRole::Derivative { .. } => {
                    derivatives += 1;
                    derivative_ids.insert(resource.resource_id.clone());
                    derivative_parents.push((
                        resource.resource_id.clone(),
                        resource
                            .derivative_provenance
                            .as_ref()
                            .ok_or(TransferError::MissingDerivativeProvenance)?
                            .parent_resource_id
                            .clone(),
                    ));
                }
            }
        }
        if originals != 1 || derivatives == 0 {
            return Err(TransferError::InvalidBundle);
        }
        let declared_derivatives = match &bundle.selection {
            AssetSelection::OriginalAndDerivatives { resource_ids } => {
                resource_ids.iter().cloned().collect::<BTreeSet<_>>()
            }
            _ => return Err(TransferError::InvalidBundle),
        };
        if declared_derivatives.len() != derivatives || declared_derivatives != derivative_ids {
            return Err(TransferError::InvalidBundle);
        }
        if derivative_parents
            .iter()
            .any(|(_, parent)| !resource_ids.contains(parent))
        {
            return Err(TransferError::InvalidBundle);
        }
        let mut resolvable = bundle
            .resources
            .iter()
            .filter(|resource| resource.role == TransferResourceRole::Original)
            .map(|resource| resource.resource_id.clone())
            .collect::<BTreeSet<_>>();
        while resolvable.len() < resource_ids.len() {
            let before = resolvable.len();
            for (child, parent) in &derivative_parents {
                if resolvable.contains(parent) {
                    resolvable.insert(child.clone());
                }
            }
            if resolvable.len() == before {
                return Err(TransferError::InvalidBundle);
            }
        }
        Ok(Self {
            bundle,
            local_asset_id,
            source_to_local: BTreeMap::new(),
            completed_resources: BTreeSet::new(),
        })
    }

    pub fn source_to_local_resource_ids(&self) -> &BTreeMap<String, String> {
        &self.source_to_local
    }

    pub fn prepare_original_receive(
        &self,
        captures_root: &Path,
        invitation: Invitation,
        available_bytes: u64,
        safety_reserve_bytes: u64,
    ) -> Result<(TransferSession, IndexedOriginalReceive), TransferError> {
        let resource = self
            .bundle
            .resources
            .iter()
            .find(|resource| resource.role == TransferResourceRole::Original)
            .cloned()
            .ok_or(TransferError::InvalidBundle)?;
        if self.completed_resources.contains(&resource.resource_id) {
            return Err(TransferError::BundleResourceAlreadyFinalized);
        }
        let session = TransferSession::new(invitation, resource.manifest.clone())?;
        let receive = IndexedOriginalReceive::create_or_resume_with_asset_id(
            captures_root,
            resource,
            self.bundle.capture.clone(),
            self.local_asset_id.clone(),
            available_bytes,
            safety_reserve_bytes,
        )?;
        Ok((session, receive))
    }

    pub fn mark_original_finalized(&mut self, asset: &CapturedAsset) -> Result<(), TransferError> {
        if asset.id != self.local_asset_id || asset.state != camera_core::AssetState::Finalized {
            return Err(TransferError::BundleFinalizationMismatch);
        }
        let source_id = self
            .bundle
            .resources
            .iter()
            .find(|resource| resource.role == TransferResourceRole::Original)
            .map(|resource| resource.resource_id.clone())
            .ok_or(TransferError::InvalidBundle)?;
        self.source_to_local
            .insert(source_id.clone(), asset.original_resource_id.clone());
        self.completed_resources.insert(source_id);
        Ok(())
    }

    pub fn prepare_derivative_receive(
        &self,
        captures_root: &Path,
        source_resource_id: &str,
        parent_asset: CapturedAsset,
        invitation: Invitation,
        available_bytes: u64,
        safety_reserve_bytes: u64,
    ) -> Result<(TransferSession, IndexedDerivativeReceive), TransferError> {
        if self.completed_resources.contains(source_resource_id) {
            return Err(TransferError::BundleResourceAlreadyFinalized);
        }
        let (index, source) = self
            .bundle
            .resources
            .iter()
            .enumerate()
            .find(|(_, resource)| resource.resource_id == source_resource_id)
            .ok_or(TransferError::InvalidSelection)?;
        if !matches!(source.role, TransferResourceRole::Derivative { .. }) {
            return Err(TransferError::InvalidSelection);
        }
        let mut resource = source.clone();
        let provenance = resource
            .derivative_provenance
            .as_mut()
            .ok_or(TransferError::MissingDerivativeProvenance)?;
        provenance.parent_resource_id = self
            .source_to_local
            .get(&provenance.parent_resource_id)
            .cloned()
            .ok_or(TransferError::BundleDependencyNotFinalized)?;
        resource.resource_id = format!("{}:peer-{index}", self.local_asset_id);
        let session = TransferSession::new(invitation, resource.manifest.clone())?;
        let receive = IndexedDerivativeReceive::create_or_resume(
            captures_root,
            resource,
            parent_asset,
            available_bytes,
            safety_reserve_bytes,
        )?;
        Ok((session, receive))
    }

    pub fn mark_derivative_finalized(
        &mut self,
        source_resource_id: &str,
        asset: &CapturedAsset,
    ) -> Result<(), TransferError> {
        if asset.id != self.local_asset_id || asset.state != camera_core::AssetState::Finalized {
            return Err(TransferError::BundleFinalizationMismatch);
        }
        let (index, source) = self
            .bundle
            .resources
            .iter()
            .enumerate()
            .find(|(_, resource)| resource.resource_id == source_resource_id)
            .ok_or(TransferError::InvalidSelection)?;
        if !matches!(source.role, TransferResourceRole::Derivative { .. }) {
            return Err(TransferError::InvalidSelection);
        }
        let local_id = format!("{}:peer-{index}", self.local_asset_id);
        if !asset
            .derivatives
            .iter()
            .any(|derivative| derivative.resource_id == local_id)
        {
            return Err(TransferError::BundleFinalizationMismatch);
        }
        self.source_to_local
            .insert(source_resource_id.to_owned(), local_id);
        self.completed_resources
            .insert(source_resource_id.to_owned());
        Ok(())
    }
}

fn resource_transfer(
    resource_id: &str,
    role: TransferResourceRole,
    path: &Path,
    transfer_id: String,
    media_type: CapturedMediaType,
    chunk_size: u32,
    metadata_policy: MetadataPolicy,
    derivative_provenance: Option<DerivativeProvenance>,
) -> Result<TransferResource, TransferError> {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or(TransferError::InvalidSourceAsset)?
        .to_owned();
    let mut file = File::open(path).map_err(io_error)?;
    let byte_length = file.metadata().map_err(io_error)?.len();
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(io_error)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let manifest = TransferManifest {
        schema_version: PROTOCOL_VERSION,
        transfer_id,
        media_type: match media_type {
            CapturedMediaType::Photo => SharedMediaType::Photo,
            CapturedMediaType::Video => SharedMediaType::Video,
        },
        file_name,
        byte_length,
        chunk_size,
        sha256_hex: hex::encode(hasher.finalize()),
        metadata_policy,
    };
    manifest.validate()?;
    Ok(TransferResource {
        resource_id: resource_id.to_owned(),
        role,
        derivative_provenance,
        manifest,
    })
}

pub fn write_wire_message(
    writer: &mut impl Write,
    message: &PeerWireMessage,
) -> Result<(), TransferError> {
    let (kind, payload) = match message {
        PeerWireMessage::EncryptedChunk(chunk) => {
            if !safe_media_token(&chunk.transfer_id)
                || chunk.ciphertext.len() > MAX_CHUNK_BYTES as usize + 16
            {
                return Err(TransferError::InvalidWireMessage);
            }
            let mut payload = Vec::with_capacity(chunk.ciphertext.len() + 64);
            encode_wire_string(&mut payload, &chunk.transfer_id)?;
            payload.extend_from_slice(&chunk.offset.to_be_bytes());
            payload.extend_from_slice(&chunk.plaintext_bytes.to_be_bytes());
            payload.extend_from_slice(&chunk.nonce);
            payload.extend_from_slice(&(chunk.ciphertext.len() as u32).to_be_bytes());
            payload.extend_from_slice(&chunk.ciphertext);
            (1_u8, payload)
        }
        PeerWireMessage::ResumeCheckpoint(checkpoint) => {
            if !safe_media_token(&checkpoint.transfer_id)
                || checkpoint.prefix_sha256.len() != 64
                || !checkpoint
                    .prefix_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(TransferError::InvalidWireMessage);
            }
            let mut payload = Vec::with_capacity(160);
            encode_wire_string(&mut payload, &checkpoint.transfer_id)?;
            payload.extend_from_slice(&checkpoint.persisted_bytes.to_be_bytes());
            payload.extend_from_slice(checkpoint.prefix_sha256.as_bytes());
            (2_u8, payload)
        }
        PeerWireMessage::DurableAck {
            transfer_id,
            persisted_bytes,
        } => {
            if !safe_media_token(transfer_id) {
                return Err(TransferError::InvalidWireMessage);
            }
            let mut payload = Vec::with_capacity(144);
            encode_wire_string(&mut payload, transfer_id)?;
            payload.extend_from_slice(&persisted_bytes.to_be_bytes());
            (3_u8, payload)
        }
        PeerWireMessage::Cancel {
            transfer_id,
            reason,
        } => {
            if !safe_media_token(transfer_id) {
                return Err(TransferError::InvalidWireMessage);
            }
            let mut payload = Vec::with_capacity(136);
            encode_wire_string(&mut payload, transfer_id)?;
            payload.push(match reason {
                TransferCancelReason::User => 1,
                TransferCancelReason::Timeout => 2,
                TransferCancelReason::PeerDisconnected => 3,
            });
            (4_u8, payload)
        }
    };
    if payload.len() > MAX_WIRE_FRAME_BYTES {
        return Err(TransferError::WireFrameTooLarge);
    }
    writer.write_all(&WIRE_MAGIC).map_err(io_error)?;
    writer.write_all(&[kind]).map_err(io_error)?;
    writer
        .write_all(&(payload.len() as u32).to_be_bytes())
        .map_err(io_error)?;
    writer.write_all(&payload).map_err(io_error)?;
    writer.flush().map_err(io_error)
}

pub fn read_wire_message(reader: &mut impl Read) -> Result<PeerWireMessage, TransferError> {
    let mut header = [0_u8; 9];
    reader.read_exact(&mut header).map_err(io_error)?;
    if header[..4] != WIRE_MAGIC {
        return Err(TransferError::InvalidWireMessage);
    }
    let payload_length = u32::from_be_bytes(header[5..9].try_into().unwrap()) as usize;
    if payload_length > MAX_WIRE_FRAME_BYTES {
        return Err(TransferError::WireFrameTooLarge);
    }
    let mut payload = vec![0_u8; payload_length];
    reader.read_exact(&mut payload).map_err(io_error)?;
    let mut cursor = 0_usize;
    let transfer_id = decode_wire_string(&payload, &mut cursor)?;
    let message = match header[4] {
        1 => {
            let offset = take_u64(&payload, &mut cursor)?;
            let plaintext_bytes = take_u32(&payload, &mut cursor)?;
            let nonce_slice = take_wire(&payload, &mut cursor, 12)?;
            let mut nonce = [0_u8; 12];
            nonce.copy_from_slice(nonce_slice);
            let ciphertext_length = take_u32(&payload, &mut cursor)? as usize;
            if ciphertext_length != plaintext_bytes as usize + 16
                || ciphertext_length > MAX_CHUNK_BYTES as usize + 16
            {
                return Err(TransferError::InvalidWireMessage);
            }
            let ciphertext = take_wire(&payload, &mut cursor, ciphertext_length)?.to_vec();
            PeerWireMessage::EncryptedChunk(EncryptedChunk {
                transfer_id,
                offset,
                plaintext_bytes,
                nonce,
                ciphertext,
            })
        }
        2 => {
            let persisted_bytes = take_u64(&payload, &mut cursor)?;
            let hash = take_wire(&payload, &mut cursor, 64)?;
            let prefix_sha256 = std::str::from_utf8(hash)
                .map_err(|_| TransferError::InvalidWireMessage)?
                .to_owned();
            if !prefix_sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                return Err(TransferError::InvalidWireMessage);
            }
            PeerWireMessage::ResumeCheckpoint(ResumeCheckpoint {
                transfer_id,
                persisted_bytes,
                prefix_sha256,
            })
        }
        3 => PeerWireMessage::DurableAck {
            transfer_id,
            persisted_bytes: take_u64(&payload, &mut cursor)?,
        },
        4 => PeerWireMessage::Cancel {
            transfer_id,
            reason: match *take_wire(&payload, &mut cursor, 1)?
                .first()
                .ok_or(TransferError::InvalidWireMessage)?
            {
                1 => TransferCancelReason::User,
                2 => TransferCancelReason::Timeout,
                3 => TransferCancelReason::PeerDisconnected,
                _ => return Err(TransferError::InvalidWireMessage),
            },
        },
        _ => return Err(TransferError::InvalidWireMessage),
    };
    if cursor != payload.len() {
        return Err(TransferError::InvalidWireMessage);
    }
    Ok(message)
}

fn encode_wire_string(output: &mut Vec<u8>, value: &str) -> Result<(), TransferError> {
    let length = u16::try_from(value.len()).map_err(|_| TransferError::InvalidWireMessage)?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn decode_wire_string(input: &[u8], cursor: &mut usize) -> Result<String, TransferError> {
    let length = take_u16(input, cursor)? as usize;
    let value = std::str::from_utf8(take_wire(input, cursor, length)?)
        .map_err(|_| TransferError::InvalidWireMessage)?
        .to_owned();
    if !safe_media_token(&value) {
        return Err(TransferError::InvalidWireMessage);
    }
    Ok(value)
}

fn take_wire<'a>(
    input: &'a [u8],
    cursor: &mut usize,
    length: usize,
) -> Result<&'a [u8], TransferError> {
    let end = cursor
        .checked_add(length)
        .filter(|end| *end <= input.len())
        .ok_or(TransferError::InvalidWireMessage)?;
    let value = &input[*cursor..end];
    *cursor = end;
    Ok(value)
}

fn take_u16(input: &[u8], cursor: &mut usize) -> Result<u16, TransferError> {
    Ok(u16::from_be_bytes(
        take_wire(input, cursor, 2)?.try_into().unwrap(),
    ))
}

fn take_u32(input: &[u8], cursor: &mut usize) -> Result<u32, TransferError> {
    Ok(u32::from_be_bytes(
        take_wire(input, cursor, 4)?.try_into().unwrap(),
    ))
}

fn take_u64(input: &[u8], cursor: &mut usize) -> Result<u64, TransferError> {
    Ok(u64::from_be_bytes(
        take_wire(input, cursor, 8)?.try_into().unwrap(),
    ))
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

    pub fn accept_resume_checkpoint(
        &self,
        checkpoint: &ResumeCheckpoint,
    ) -> Result<u64, TransferError> {
        self.require_state(|state| matches!(state, TransferState::Transferring { .. }))?;
        if !self.negotiated_resume
            || checkpoint.transfer_id != self.manifest.transfer_id
            || checkpoint.persisted_bytes > self.manifest.byte_length
            || checkpoint.prefix_sha256.len() != 64
            || !checkpoint
                .prefix_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(TransferError::InvalidResumeCheckpoint);
        }
        Ok(checkpoint.persisted_bytes)
    }

    pub fn finalize_receive(&mut self, writer: ReceiveWriter) -> Result<PathBuf, TransferError> {
        self.require_state(|state| matches!(state, TransferState::Verifying))?;
        if writer.manifest() != &self.manifest {
            return Err(TransferError::ManifestMismatch);
        }
        let completion = match writer.finish() {
            Ok(completion) => completion,
            Err(error @ TransferError::ContentVerificationFailed) => {
                self.state = TransferState::Failed {
                    reason: "content verification failed".into(),
                };
                return Err(error);
            }
            Err(error) => return Err(error),
        };
        self.finalize(completion.byte_length, &completion.sha256_hex)?;
        Ok(completion.path)
    }

    pub fn finalize_indexed_original(
        &mut self,
        receive: IndexedOriginalReceive,
    ) -> Result<CapturedAsset, TransferError> {
        self.require_state(|state| matches!(state, TransferState::Verifying))?;
        if receive.writer.manifest() != &self.manifest {
            return Err(TransferError::ManifestMismatch);
        }
        let record_id = self.manifest.transfer_id.clone();
        let incomplete_path = receive.writer.incomplete_path.clone();
        let completion = match receive.writer.finish() {
            Ok(completion) => completion,
            Err(error) => {
                let _ = receive.media_index.record_failed(
                    record_id,
                    receive.media_type,
                    incomplete_path,
                    error.to_string(),
                );
                self.state = TransferState::Failed {
                    reason: error.to_string(),
                };
                return Err(error);
            }
        };
        let resource = match probe_media_resource(&completion.path, receive.media_type) {
            Ok(resource) => resource,
            Err(error) => {
                let message = error.to_string();
                let _ = receive.media_index.record_failed(
                    record_id,
                    receive.media_type,
                    &completion.path,
                    &message,
                );
                self.state = TransferState::Failed {
                    reason: message.clone(),
                };
                return Err(TransferError::MediaValidation(message));
            }
        };
        let asset = match CapturedAsset::from_probed_resource(
            receive.asset_id,
            receive.media_type,
            resource,
            completion.path.clone(),
            receive.capture,
        ) {
            Ok(asset) => asset,
            Err(error) => {
                let message = error.to_string();
                let _ = receive.media_index.record_failed(
                    record_id,
                    receive.media_type,
                    &completion.path,
                    &message,
                );
                self.state = TransferState::Failed {
                    reason: message.clone(),
                };
                return Err(TransferError::MediaValidation(message));
            }
        };
        if let Err(error) = receive.media_index.persist_finalized(&asset) {
            let message = error.to_string();
            let rollback = fs::rename(&completion.path, &completion.rollback_path);
            let failed_path = if rollback.is_ok() {
                completion.rollback_path
            } else {
                completion.path
            };
            let _ = receive.media_index.record_failed(
                record_id,
                receive.media_type,
                failed_path,
                &message,
            );
            self.state = TransferState::Failed {
                reason: message.clone(),
            };
            return Err(TransferError::MediaIndex(message));
        }
        if record_id != asset.id {
            let _ = receive.media_index.cleanup_recoverable(&record_id);
        }
        self.finalize(completion.byte_length, &completion.sha256_hex)?;
        Ok(asset)
    }

    pub fn finalize_indexed_derivative(
        &mut self,
        receive: IndexedDerivativeReceive,
    ) -> Result<CapturedAsset, TransferError> {
        self.require_state(|state| matches!(state, TransferState::Verifying))?;
        if receive.writer.manifest() != &self.manifest {
            return Err(TransferError::ManifestMismatch);
        }
        let IndexedDerivativeReceive {
            writer,
            media_index,
            media_type,
            mut parent_asset,
            resource_id,
            purpose,
            provenance,
        } = receive;
        let record_id = self.manifest.transfer_id.clone();
        let incomplete_path = writer.incomplete_path.clone();
        let completion = match writer.finish() {
            Ok(completion) => completion,
            Err(error) => {
                let _ = media_index.record_failed(
                    &record_id,
                    media_type,
                    incomplete_path,
                    error.to_string(),
                );
                self.state = TransferState::Failed {
                    reason: error.to_string(),
                };
                return Err(error);
            }
        };
        let mut resource = match probe_media_resource(&completion.path, media_type) {
            Ok(resource) => resource,
            Err(error) => {
                return self.fail_completed_derivative(
                    &media_index,
                    media_type,
                    &record_id,
                    completion,
                    error.to_string(),
                );
            }
        };
        resource.path = completion.path.clone();
        if let Err(error) = parent_asset.add_derivative(resource_id, purpose, resource, provenance)
        {
            return self.fail_completed_derivative(
                &media_index,
                media_type,
                &record_id,
                completion,
                error.to_string(),
            );
        }
        if let Err(error) = media_index.persist_finalized(&parent_asset) {
            return self.fail_completed_derivative(
                &media_index,
                media_type,
                &record_id,
                completion,
                error.to_string(),
            );
        }
        // The completed parent manifest is authoritative. A stale recoverable
        // record is safe to reconcile later and must not roll back valid media.
        let _ = media_index.cleanup_recoverable(&record_id);
        self.finalize(completion.byte_length, &completion.sha256_hex)?;
        Ok(parent_asset)
    }

    fn fail_completed_derivative<T>(
        &mut self,
        media_index: &MediaIndex,
        media_type: CapturedMediaType,
        record_id: &str,
        completion: ReceiveCompletion,
        message: String,
    ) -> Result<T, TransferError> {
        let rollback = fs::rename(&completion.path, &completion.rollback_path);
        let failed_path = if rollback.is_ok() {
            completion.rollback_path
        } else {
            completion.path
        };
        let _ = media_index.record_failed(record_id, media_type, failed_path, &message);
        self.state = TransferState::Failed {
            reason: message.clone(),
        };
        Err(TransferError::MediaValidation(message))
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ReceiveLedger {
    schema_version: u16,
    manifest: TransferManifest,
    persisted_bytes: u64,
}

pub struct ReceiveWriter {
    manifest: TransferManifest,
    incomplete_path: PathBuf,
    ledger_path: PathBuf,
    destination_path: PathBuf,
    file: File,
    persisted_bytes: u64,
    hasher: Sha256,
}

pub struct IndexedOriginalReceive {
    writer: ReceiveWriter,
    media_index: MediaIndex,
    media_type: CapturedMediaType,
    capture: CaptureMetadata,
    asset_id: String,
}

pub struct IndexedDerivativeReceive {
    writer: ReceiveWriter,
    media_index: MediaIndex,
    media_type: CapturedMediaType,
    parent_asset: CapturedAsset,
    resource_id: String,
    purpose: DerivativePurpose,
    provenance: DerivativeProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReceiveCompletion {
    path: PathBuf,
    rollback_path: PathBuf,
    byte_length: u64,
    sha256_hex: String,
}

impl IndexedOriginalReceive {
    pub fn create_or_resume(
        captures_root: &Path,
        resource: TransferResource,
        capture: CaptureMetadata,
        available_bytes: u64,
        safety_reserve_bytes: u64,
    ) -> Result<Self, TransferError> {
        let asset_id = resource.manifest.transfer_id.clone();
        Self::create_or_resume_with_asset_id(
            captures_root,
            resource,
            capture,
            asset_id,
            available_bytes,
            safety_reserve_bytes,
        )
    }

    pub fn create_or_resume_with_asset_id(
        captures_root: &Path,
        resource: TransferResource,
        capture: CaptureMetadata,
        asset_id: String,
        available_bytes: u64,
        safety_reserve_bytes: u64,
    ) -> Result<Self, TransferError> {
        if resource.role != TransferResourceRole::Original {
            return Err(TransferError::InvalidSelection);
        }
        if resource.derivative_provenance.is_some() || !safe_media_token(&asset_id) {
            return Err(TransferError::InvalidSourceAsset);
        }
        let manifest = resource.manifest;
        let media_type = match manifest.media_type {
            SharedMediaType::Photo => CapturedMediaType::Photo,
            SharedMediaType::Video => CapturedMediaType::Video,
        };
        let writer = ReceiveWriter::create_or_resume(
            captures_root,
            manifest,
            available_bytes,
            safety_reserve_bytes,
        )?;
        let media_index = MediaIndex::new(captures_root);
        media_index
            .record_incomplete(
                writer.manifest.transfer_id.clone(),
                media_type,
                writer.incomplete_path.clone(),
            )
            .map_err(|error| TransferError::MediaIndex(error.to_string()))?;
        Ok(Self {
            writer,
            media_index,
            media_type,
            capture,
            asset_id,
        })
    }

    pub fn persisted_bytes(&self) -> u64 {
        self.writer.persisted_bytes()
    }

    pub fn write_chunk(&mut self, offset: u64, bytes: &[u8]) -> Result<u64, TransferError> {
        self.writer.write_chunk(offset, bytes)
    }
}

impl IndexedDerivativeReceive {
    pub fn create_or_resume(
        captures_root: &Path,
        resource: TransferResource,
        parent_asset: CapturedAsset,
        available_bytes: u64,
        safety_reserve_bytes: u64,
    ) -> Result<Self, TransferError> {
        let TransferResourceRole::Derivative { purpose } = resource.role else {
            return Err(TransferError::InvalidSelection);
        };
        let provenance = resource
            .derivative_provenance
            .ok_or(TransferError::MissingDerivativeProvenance)?;
        let parent_exists = provenance.parent_resource_id == parent_asset.original_resource_id
            || parent_asset
                .derivatives
                .iter()
                .any(|item| item.resource_id == provenance.parent_resource_id);
        if parent_asset.state != camera_core::AssetState::Finalized || !parent_exists {
            return Err(TransferError::DerivativeParentMismatch);
        }
        let manifest = resource.manifest;
        let media_type = match manifest.media_type {
            SharedMediaType::Photo => CapturedMediaType::Photo,
            SharedMediaType::Video => CapturedMediaType::Video,
        };
        if media_type != parent_asset.media_type {
            return Err(TransferError::DerivativeParentMismatch);
        }
        let writer = ReceiveWriter::create_or_resume(
            captures_root,
            manifest,
            available_bytes,
            safety_reserve_bytes,
        )?;
        let media_index = MediaIndex::new(captures_root);
        media_index
            .record_incomplete(
                writer.manifest.transfer_id.clone(),
                media_type,
                writer.incomplete_path.clone(),
            )
            .map_err(|error| TransferError::MediaIndex(error.to_string()))?;
        Ok(Self {
            writer,
            media_index,
            media_type,
            parent_asset,
            resource_id: resource.resource_id,
            purpose,
            provenance,
        })
    }

    pub fn persisted_bytes(&self) -> u64 {
        self.writer.persisted_bytes()
    }

    pub fn write_chunk(&mut self, offset: u64, bytes: &[u8]) -> Result<u64, TransferError> {
        self.writer.write_chunk(offset, bytes)
    }
}

impl ReceiveWriter {
    pub fn create_or_resume(
        root: &Path,
        manifest: TransferManifest,
        available_bytes: u64,
        safety_reserve_bytes: u64,
    ) -> Result<Self, TransferError> {
        manifest.validate()?;
        fs::create_dir_all(root).map_err(io_error)?;
        let incomplete_dir = root.join(".incomplete").join("peer-transfer");
        fs::create_dir_all(&incomplete_dir).map_err(io_error)?;
        ensure_managed_directory(root, &incomplete_dir)?;

        let incomplete_path = incomplete_dir.join(format!("{}.part", manifest.transfer_id));
        let ledger_path = incomplete_dir.join(format!("{}.json", manifest.transfer_id));
        let destination_path = root.join(&manifest.file_name);
        reject_symlink(&incomplete_path)?;
        reject_symlink(&ledger_path)?;
        if destination_path.exists() {
            return Err(TransferError::DestinationExists);
        }

        let ledger = if ledger_path.exists() {
            let bytes = fs::read(&ledger_path).map_err(io_error)?;
            let ledger: ReceiveLedger =
                serde_json::from_slice(&bytes).map_err(|_| TransferError::CorruptResumeLedger)?;
            if ledger.schema_version != PROTOCOL_VERSION || ledger.manifest != manifest {
                return Err(TransferError::ManifestMismatch);
            }
            Some(ledger)
        } else {
            None
        };

        let persisted_bytes = ledger.as_ref().map_or(0, |value| value.persisted_bytes);
        let remaining = manifest.byte_length.saturating_sub(persisted_bytes);
        if available_bytes < remaining.saturating_add(safety_reserve_bytes) {
            return Err(TransferError::InsufficientStorage);
        }

        let mut file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&incomplete_path)
            .map_err(io_error)?;
        let file_length = file.metadata().map_err(io_error)?.len();
        if file_length != persisted_bytes || persisted_bytes > manifest.byte_length {
            return Err(TransferError::CorruptResumeLedger);
        }

        let mut hasher = Sha256::new();
        file.seek(SeekFrom::Start(0)).map_err(io_error)?;
        let mut buffer = [0_u8; 64 * 1024];
        let mut rehashed = 0_u64;
        while rehashed < persisted_bytes {
            let limit = (persisted_bytes - rehashed).min(buffer.len() as u64) as usize;
            let count = file.read(&mut buffer[..limit]).map_err(io_error)?;
            if count == 0 {
                return Err(TransferError::CorruptResumeLedger);
            }
            hasher.update(&buffer[..count]);
            rehashed += count as u64;
        }
        file.seek(SeekFrom::Start(persisted_bytes))
            .map_err(io_error)?;

        let mut writer = Self {
            manifest,
            incomplete_path,
            ledger_path,
            destination_path,
            file,
            persisted_bytes,
            hasher,
        };
        if ledger.is_none() {
            writer.persist_ledger()?;
        }
        Ok(writer)
    }

    pub fn resume_checkpoint(&self) -> ResumeCheckpoint {
        ResumeCheckpoint {
            transfer_id: self.manifest.transfer_id.clone(),
            persisted_bytes: self.persisted_bytes,
            prefix_sha256: hex::encode(self.hasher.clone().finalize()),
        }
    }

    pub fn write_encrypted_chunk(
        &mut self,
        codec: &EncryptedChunkCodec,
        chunk: &EncryptedChunk,
    ) -> Result<u64, TransferError> {
        if codec.transfer_id != self.manifest.transfer_id
            || codec.total_plaintext_bytes != self.manifest.byte_length
        {
            return Err(TransferError::ManifestMismatch);
        }
        let plaintext = codec.decrypt(chunk)?;
        self.write_chunk(chunk.offset, &plaintext)
    }

    pub fn manifest(&self) -> &TransferManifest {
        &self.manifest
    }

    pub fn persisted_bytes(&self) -> u64 {
        self.persisted_bytes
    }

    /// Writes exactly the next contiguous chunk and returns a durable ACK.
    pub fn write_chunk(&mut self, offset: u64, bytes: &[u8]) -> Result<u64, TransferError> {
        if offset != self.persisted_bytes || bytes.is_empty() {
            return Err(TransferError::InvalidChunk);
        }
        let next = offset
            .checked_add(bytes.len() as u64)
            .ok_or(TransferError::InvalidChunk)?;
        if next > self.manifest.byte_length || bytes.len() as u32 > self.manifest.chunk_size {
            return Err(TransferError::InvalidChunk);
        }
        self.file.write_all(bytes).map_err(io_error)?;
        self.file.sync_data().map_err(io_error)?;
        self.hasher.update(bytes);
        self.persisted_bytes = next;
        self.persist_ledger()?;
        Ok(self.persisted_bytes)
    }

    fn persist_ledger(&mut self) -> Result<(), TransferError> {
        let ledger = ReceiveLedger {
            schema_version: PROTOCOL_VERSION,
            manifest: self.manifest.clone(),
            persisted_bytes: self.persisted_bytes,
        };
        let bytes = serde_json::to_vec(&ledger).map_err(|_| TransferError::LedgerSerialization)?;
        let temporary = self.ledger_path.with_extension("json.tmp");
        {
            let mut file = File::create(&temporary).map_err(io_error)?;
            file.write_all(&bytes).map_err(io_error)?;
            file.sync_all().map_err(io_error)?;
        }
        fs::rename(&temporary, &self.ledger_path).map_err(io_error)
    }

    fn finish(self) -> Result<ReceiveCompletion, TransferError> {
        if self.persisted_bytes != self.manifest.byte_length {
            return Err(TransferError::IncompleteReceive);
        }
        self.file.sync_all().map_err(io_error)?;
        let sha256_hex = hex::encode(self.hasher.finalize());
        if !sha256_hex.eq_ignore_ascii_case(&self.manifest.sha256_hex) {
            return Err(TransferError::ContentVerificationFailed);
        }
        if self.destination_path.exists() {
            return Err(TransferError::DestinationExists);
        }
        fs::rename(&self.incomplete_path, &self.destination_path).map_err(io_error)?;
        fs::remove_file(&self.ledger_path).map_err(io_error)?;
        Ok(ReceiveCompletion {
            path: self.destination_path,
            rollback_path: self.incomplete_path,
            byte_length: self.persisted_bytes,
            sha256_hex,
        })
    }
}

pub fn verify_resume_checkpoint(
    source: &Path,
    manifest: &TransferManifest,
    checkpoint: &ResumeCheckpoint,
) -> Result<(), TransferError> {
    manifest.validate()?;
    if checkpoint.transfer_id != manifest.transfer_id
        || checkpoint.persisted_bytes > manifest.byte_length
        || checkpoint.prefix_sha256.len() != 64
        || !checkpoint
            .prefix_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(TransferError::InvalidResumeCheckpoint);
    }
    let mut file = File::open(source).map_err(io_error)?;
    if file.metadata().map_err(io_error)?.len() != manifest.byte_length {
        return Err(TransferError::ManifestMismatch);
    }
    let mut remaining = checkpoint.persisted_bytes;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        let limit = remaining.min(buffer.len() as u64) as usize;
        let count = file.read(&mut buffer[..limit]).map_err(io_error)?;
        if count == 0 {
            return Err(TransferError::InvalidResumeCheckpoint);
        }
        hasher.update(&buffer[..count]);
        remaining -= count as u64;
    }
    if !hex::encode(hasher.finalize()).eq_ignore_ascii_case(&checkpoint.prefix_sha256) {
        return Err(TransferError::ResumePrefixMismatch);
    }
    Ok(())
}

fn ensure_managed_directory(root: &Path, directory: &Path) -> Result<(), TransferError> {
    let root = root.canonicalize().map_err(io_error)?;
    let directory = directory.canonicalize().map_err(io_error)?;
    if directory.starts_with(root) {
        Ok(())
    } else {
        Err(TransferError::UnsafeReceivePath)
    }
}

fn reject_symlink(path: &Path) -> Result<(), TransferError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(TransferError::UnsafeReceivePath),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error(error)),
    }
}

fn validate_invitation(invitation: &Invitation) -> Result<(), TransferError> {
    if !safe_token(&invitation.invitation_id) || !safe_token(&invitation.sender.ephemeral_id) {
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

fn safe_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn safe_media_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn io_error(error: std::io::Error) -> TransferError {
    TransferError::Io(error.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferError {
    UnsupportedVersion(u16),
    InvalidManifest(&'static str),
    InvalidInvitation,
    UnsafeFileName,
    InvitationExpired,
    ConfirmationMismatch,
    InvalidEphemeralKey,
    KeyAgreementContextMismatch,
    KeyDerivationFailed,
    SecureRandomFailed,
    NoCompatibleProtocol,
    NoHighBandwidthTransport,
    NoCompatibleChunkSize,
    InvalidAcknowledgement,
    InvalidChunk,
    InvalidEncryptedChunk,
    ChunkAuthenticationFailed,
    InvalidWireMessage,
    WireFrameTooLarge,
    InvalidTransportTimeout,
    InvalidTransportLifecycle,
    ManifestMismatch,
    CorruptResumeLedger,
    InvalidResumeCheckpoint,
    ResumePrefixMismatch,
    LedgerSerialization,
    InsufficientStorage,
    IncompleteReceive,
    DestinationExists,
    UnsafeReceivePath,
    ContentVerificationFailed,
    InvalidState,
    InvalidSourceAsset,
    InvalidSelection,
    InvalidBundle,
    BundleDependencyNotFinalized,
    BundleResourceAlreadyFinalized,
    BundleFinalizationMismatch,
    MissingDerivativeProvenance,
    DerivativeParentMismatch,
    MetadataSanitizationRequired,
    UnsupportedMetadataPolicy,
    UnsupportedMediaForSanitization,
    SanitizedSourceMismatch,
    MalformedJpeg,
    MediaIndex(String),
    MediaValidation(String),
    Io(String),
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
    use imaging_core::{ProfileKind, ProfileSnapshotEntry, RenderProfileSnapshot};

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

    fn receive_fixture(data: &[u8]) -> (PathBuf, TransferSession) {
        static FIXTURE_SEQUENCE: std::sync::atomic::AtomicU64 =
            std::sync::atomic::AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "ufc-peer-receive-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            FIXTURE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        let mut transfer = session();
        transfer.manifest.byte_length = data.len() as u64;
        transfer.manifest.chunk_size = MIN_CHUNK_BYTES;
        transfer.manifest.sha256_hex = hex::encode(Sha256::digest(data));
        transfer.approve("482913", 5_000).unwrap();
        let peers = capabilities(vec![TransportKind::LocalNetwork]);
        transfer.negotiate(&peers, &peers).unwrap();
        (root, transfer)
    }

    #[test]
    fn receive_writer_resumes_rehashes_and_atomically_publishes() {
        let data = vec![0x5a; 30_000];
        let (root, mut transfer) = receive_fixture(&data);
        let mut writer = ReceiveWriter::create_or_resume(
            &root,
            transfer.manifest.clone(),
            u64::MAX,
            DEFAULT_RECEIVE_RESERVE_BYTES,
        )
        .unwrap();
        let first_ack = writer
            .write_chunk(0, &data[..MIN_CHUNK_BYTES as usize])
            .unwrap();
        transfer.acknowledge(first_ack).unwrap();
        drop(writer);

        let mut resumed = ReceiveWriter::create_or_resume(
            &root,
            transfer.manifest.clone(),
            u64::MAX,
            DEFAULT_RECEIVE_RESERVE_BYTES,
        )
        .unwrap();
        assert_eq!(resumed.persisted_bytes(), first_ack);
        let final_ack = resumed
            .write_chunk(first_ack, &data[first_ack as usize..])
            .unwrap();
        transfer.acknowledge(final_ack).unwrap();
        let completed = transfer.finalize_receive(resumed).unwrap();
        assert_eq!(fs::read(&completed).unwrap(), data);
        assert_eq!(transfer.state, TransferState::Finalized);
        assert!(
            !root
                .join(".incomplete/peer-transfer/transfer-1.json")
                .exists()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn encrypted_chunks_authenticate_offsets_and_resume_prefix() {
        let data = (0..30_000)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let (root, mut transfer) = receive_fixture(&data);
        fs::create_dir_all(&root).unwrap();
        let source = root.join("encrypted-source.bin");
        fs::write(&source, &data).unwrap();
        let key = [0x41; 32];
        let nonce_prefix = [0x27; 16];
        let codec = EncryptedChunkCodec::new(key, nonce_prefix, &transfer.manifest).unwrap();
        let mut writer = ReceiveWriter::create_or_resume(
            &root,
            transfer.manifest.clone(),
            u64::MAX,
            DEFAULT_RECEIVE_RESERVE_BYTES,
        )
        .unwrap();
        let first_plaintext = &data[..MIN_CHUNK_BYTES as usize];
        let first = codec.encrypt(0, first_plaintext).unwrap();
        let mut tampered = first.clone();
        tampered.ciphertext[0] ^= 0x80;
        assert!(matches!(
            writer.write_encrypted_chunk(&codec, &tampered),
            Err(TransferError::ChunkAuthenticationFailed)
        ));
        let mut moved = first.clone();
        moved.offset = 1;
        assert!(matches!(
            codec.decrypt(&moved),
            Err(TransferError::ChunkAuthenticationFailed)
        ));
        let first_ack = writer.write_encrypted_chunk(&codec, &first).unwrap();
        transfer.acknowledge(first_ack).unwrap();
        drop(writer);

        let mut resumed = ReceiveWriter::create_or_resume(
            &root,
            transfer.manifest.clone(),
            u64::MAX,
            DEFAULT_RECEIVE_RESERVE_BYTES,
        )
        .unwrap();
        let checkpoint = resumed.resume_checkpoint();
        assert_eq!(checkpoint.persisted_bytes, first_ack);
        transfer.accept_resume_checkpoint(&checkpoint).unwrap();
        verify_resume_checkpoint(&source, &transfer.manifest, &checkpoint).unwrap();
        let mut false_checkpoint = checkpoint.clone();
        false_checkpoint.prefix_sha256 = "00".repeat(32);
        assert!(matches!(
            verify_resume_checkpoint(&source, &transfer.manifest, &false_checkpoint),
            Err(TransferError::ResumePrefixMismatch)
        ));

        let second = codec
            .encrypt(first_ack, &data[first_ack as usize..])
            .unwrap();
        let final_ack = resumed.write_encrypted_chunk(&codec, &second).unwrap();
        transfer.acknowledge(final_ack).unwrap();
        let completed = transfer.finalize_receive(resumed).unwrap();
        assert_eq!(fs::read(completed).unwrap(), data);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn ephemeral_handshake_binds_code_keys_and_transfer_manifest() {
        let mut transfer = session();
        let sender = EphemeralKeyPair::from_secret_bytes([0x11; 32]).unwrap();
        let receiver = EphemeralKeyPair::from_secret_bytes([0x22; 32]).unwrap();
        let sender_public = sender.public_key();
        let receiver_public = receiver.public_key();
        let confirmation_code = sender
            .confirmation_code(
                receiver_public,
                &transfer.invitation.invitation_id,
                &transfer.invitation.sender.ephemeral_id,
                &transfer.manifest,
            )
            .unwrap();
        assert_eq!(
            receiver
                .confirmation_code(
                    sender_public,
                    &transfer.invitation.invitation_id,
                    &transfer.invitation.sender.ephemeral_id,
                    &transfer.manifest,
                )
                .unwrap(),
            confirmation_code
        );
        transfer.invitation.confirmation_code = confirmation_code.clone();
        let sender_secrets = sender
            .agree(
                receiver_public,
                &transfer.invitation,
                &confirmation_code,
                &transfer.manifest,
            )
            .unwrap();
        let receiver_secrets = receiver
            .agree(
                sender_public,
                &transfer.invitation,
                &confirmation_code,
                &transfer.manifest,
            )
            .unwrap();
        let sender_codec = sender_secrets.into_chunk_codec(&transfer.manifest).unwrap();
        let receiver_codec = receiver_secrets
            .into_chunk_codec(&transfer.manifest)
            .unwrap();
        let plaintext = vec![0x5a; MIN_CHUNK_BYTES as usize];
        let frame = sender_codec.encrypt(0, &plaintext).unwrap();
        assert_eq!(receiver_codec.decrypt(&frame).unwrap(), plaintext);

        let wrong_code_key = EphemeralKeyPair::from_secret_bytes([0x33; 32]).unwrap();
        assert!(matches!(
            wrong_code_key.agree(
                sender_public,
                &transfer.invitation,
                "000000",
                &transfer.manifest,
            ),
            Err(TransferError::KeyAgreementContextMismatch)
        ));
        assert!(matches!(
            EphemeralKeyPair::from_secret_bytes([0; 32]),
            Err(TransferError::InvalidEphemeralKey)
        ));

        let mut other_manifest = transfer.manifest.clone();
        other_manifest.transfer_id = "transfer-other".into();
        let other_sender = EphemeralKeyPair::from_secret_bytes([0x44; 32]).unwrap();
        let other_receiver = EphemeralKeyPair::from_secret_bytes([0x55; 32]).unwrap();
        let other_sender_public = other_sender.public_key();
        let other_receiver_public = other_receiver.public_key();
        let other_confirmation = other_sender
            .confirmation_code(
                other_receiver_public,
                &transfer.invitation.invitation_id,
                &transfer.invitation.sender.ephemeral_id,
                &other_manifest,
            )
            .unwrap();
        let mut other_invitation = transfer.invitation.clone();
        other_invitation.confirmation_code = other_confirmation.clone();
        let other_codec = other_receiver
            .agree(
                other_sender_public,
                &other_invitation,
                &other_confirmation,
                &other_manifest,
            )
            .unwrap()
            .into_chunk_codec(&other_manifest)
            .unwrap();
        let other_frame = other_sender
            .agree(
                other_receiver_public,
                &other_invitation,
                &other_confirmation,
                &other_manifest,
            )
            .unwrap()
            .into_chunk_codec(&other_manifest)
            .unwrap()
            .encrypt(0, &vec![0x5a; MIN_CHUNK_BYTES as usize])
            .unwrap();
        assert!(matches!(
            receiver_codec.decrypt(&other_frame),
            Err(TransferError::InvalidEncryptedChunk)
        ));
        assert!(other_codec.decrypt(&other_frame).is_ok());
    }

    #[test]
    fn local_network_transport_frames_encrypted_chunks_and_durable_ack() {
        let generated_a = EphemeralKeyPair::generate().unwrap();
        let generated_b = EphemeralKeyPair::generate().unwrap();
        assert_ne!(generated_a.public_key(), generated_b.public_key());

        let transfer = session();
        let codec = EncryptedChunkCodec::new([0x61; 32], [0x19; 16], &transfer.manifest).unwrap();
        let plaintext = vec![0x7c; MIN_CHUNK_BYTES as usize];
        let encrypted = codec.encrypt(0, &plaintext).unwrap();
        let expected = PeerWireMessage::EncryptedChunk(encrypted.clone());
        if let Ok(listener) = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)) {
            let address = listener.local_addr().unwrap();
            let transfer_id = transfer.manifest.transfer_id.clone();
            let server = std::thread::spawn(move || {
                let (stream, _) = listener.accept().unwrap();
                let mut transport = LocalNetworkTransport::from_stream(stream).unwrap();
                let received = transport.receive().unwrap();
                assert_eq!(received, PeerWireMessage::EncryptedChunk(encrypted));
                transport
                    .send(&PeerWireMessage::DurableAck {
                        transfer_id,
                        persisted_bytes: MIN_CHUNK_BYTES as u64,
                    })
                    .unwrap();
            });
            let mut client = LocalNetworkTransport::connect(address).unwrap();
            client.send(&expected).unwrap();
            assert_eq!(
                client.receive().unwrap(),
                PeerWireMessage::DurableAck {
                    transfer_id: transfer.manifest.transfer_id.clone(),
                    persisted_bytes: MIN_CHUNK_BYTES as u64,
                }
            );
            server.join().unwrap();
        } else {
            let mut bytes = Vec::new();
            write_wire_message(&mut bytes, &expected).unwrap();
            assert_eq!(
                read_wire_message(&mut std::io::Cursor::new(bytes)).unwrap(),
                expected
            );
        }

        let mut oversized = Vec::new();
        oversized.extend_from_slice(&WIRE_MAGIC);
        oversized.push(1);
        oversized.extend_from_slice(&((MAX_WIRE_FRAME_BYTES + 1) as u32).to_be_bytes());
        assert!(matches!(
            read_wire_message(&mut std::io::Cursor::new(oversized)),
            Err(TransferError::WireFrameTooLarge)
        ));
        let checkpoint = PeerWireMessage::ResumeCheckpoint(ResumeCheckpoint {
            transfer_id: transfer.manifest.transfer_id,
            persisted_bytes: MIN_CHUNK_BYTES as u64,
            prefix_sha256: "ab".repeat(32),
        });
        let mut bytes = Vec::new();
        write_wire_message(&mut bytes, &checkpoint).unwrap();
        assert_eq!(
            read_wire_message(&mut std::io::Cursor::new(bytes)).unwrap(),
            checkpoint
        );
        let cancel = PeerWireMessage::Cancel {
            transfer_id: "transfer-1".into(),
            reason: TransferCancelReason::Timeout,
        };
        let mut bytes = Vec::new();
        write_wire_message(&mut bytes, &cancel).unwrap();
        assert_eq!(
            read_wire_message(&mut std::io::Cursor::new(bytes)).unwrap(),
            cancel
        );
    }

    #[test]
    fn transport_lifecycle_resumes_after_disconnect_and_finalizes() {
        let data = (0..30_000)
            .map(|index| (index % 239) as u8)
            .collect::<Vec<_>>();
        let (root, mut session) = receive_fixture(&data);
        fs::create_dir_all(&root).unwrap();
        let source_path = root.join("lifecycle-source.bin");
        fs::write(&source_path, &data).unwrap();
        let sender_codec =
            EncryptedChunkCodec::new([0x71; 32], [0x29; 16], &session.manifest).unwrap();
        let receiver_codec =
            EncryptedChunkCodec::new([0x71; 32], [0x29; 16], &session.manifest).unwrap();
        let cancel_codec =
            EncryptedChunkCodec::new([0x72; 32], [0x30; 16], &session.manifest).unwrap();
        let mut cancelled_sender =
            EncryptedTransferSender::open(&source_path, &session, cancel_codec).unwrap();
        assert!(matches!(
            cancelled_sender.cancel(TransferCancelReason::User).unwrap(),
            PeerWireMessage::Cancel {
                reason: TransferCancelReason::User,
                ..
            }
        ));
        assert!(matches!(
            cancelled_sender.next_chunk(),
            Err(TransferError::InvalidTransportLifecycle)
        ));
        let mut sender =
            EncryptedTransferSender::open(&source_path, &session, sender_codec).unwrap();
        let writer = ReceiveWriter::create_or_resume(
            &root,
            session.manifest.clone(),
            u64::MAX,
            DEFAULT_RECEIVE_RESERVE_BYTES,
        )
        .unwrap();
        let mut receiver =
            EncryptedTransferReceiver::new(writer, &session, receiver_codec).unwrap();

        let first = sender.next_chunk().unwrap().unwrap();
        let first_ack = receiver.accept(&first).unwrap().unwrap();
        let first_offset = sender.accept_ack(&first_ack).unwrap();
        sender.mark_disconnected();
        let checkpoint = receiver.resume_checkpoint();
        assert_eq!(checkpoint.persisted_bytes, first_offset);
        drop(receiver);

        sender
            .resume_from_checkpoint(&session, &checkpoint)
            .unwrap();
        let resumed_writer = ReceiveWriter::create_or_resume(
            &root,
            session.manifest.clone(),
            u64::MAX,
            DEFAULT_RECEIVE_RESERVE_BYTES,
        )
        .unwrap();
        let resumed_codec =
            EncryptedChunkCodec::new([0x71; 32], [0x29; 16], &session.manifest).unwrap();
        let mut receiver =
            EncryptedTransferReceiver::new(resumed_writer, &session, resumed_codec).unwrap();
        while let Some(chunk) = sender.next_chunk().unwrap() {
            let ack = receiver.accept(&chunk).unwrap().unwrap();
            sender.accept_ack(&ack).unwrap();
        }
        assert_eq!(sender.state(), TransportLifecycleState::Complete);
        assert_eq!(receiver.state(), TransportLifecycleState::Complete);
        session.acknowledge(data.len() as u64).unwrap();
        let completed = session
            .finalize_receive(receiver.into_writer().unwrap())
            .unwrap();
        assert_eq!(fs::read(completed).unwrap(), data);

        assert!(matches!(
            sender.cancel(TransferCancelReason::User),
            Err(TransferError::InvalidTransportLifecycle)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn receive_writer_rejects_insufficient_space_before_creating_partial_file() {
        let data = vec![0x11; 20_000];
        let (root, transfer) = receive_fixture(&data);
        let result = ReceiveWriter::create_or_resume(
            &root,
            transfer.manifest,
            data.len() as u64 + DEFAULT_RECEIVE_RESERVE_BYTES - 1,
            DEFAULT_RECEIVE_RESERVE_BYTES,
        );
        assert!(matches!(result, Err(TransferError::InsufficientStorage)));
        assert!(
            !root
                .join(".incomplete/peer-transfer/transfer-1.part")
                .exists()
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn receive_hash_mismatch_keeps_partial_and_never_finalizes() {
        let data = vec![0x22; 20_000];
        let (root, mut transfer) = receive_fixture(&data);
        transfer.manifest.sha256_hex = "00".repeat(32);
        let mut writer = ReceiveWriter::create_or_resume(
            &root,
            transfer.manifest.clone(),
            u64::MAX,
            DEFAULT_RECEIVE_RESERVE_BYTES,
        )
        .unwrap();
        let first = writer
            .write_chunk(0, &data[..MIN_CHUNK_BYTES as usize])
            .unwrap();
        let ack = writer.write_chunk(first, &data[first as usize..]).unwrap();
        transfer.acknowledge(ack).unwrap();
        assert!(matches!(
            transfer.finalize_receive(writer),
            Err(TransferError::ContentVerificationFailed)
        ));
        assert!(matches!(transfer.state, TransferState::Failed { .. }));
        assert!(
            root.join(".incomplete/peer-transfer/transfer-1.part")
                .exists()
        );
        assert!(!root.join("UFC-photo.jpg").exists());
        fs::remove_dir_all(root).unwrap();
    }

    fn jpeg_fixture() -> Vec<u8> {
        let mut jpeg = vec![0xff, 0xd8];
        let exif = [
            b'E', b'x', b'i', b'f', 0, 0, b'I', b'I', 42, 0, 8, 0, 0, 0, 2, 0, 0x12, 0x01, 3, 0, 1,
            0, 0, 0, 1, 0, 0, 0, 0x69, 0x87, 4, 0, 1, 0, 0, 0, 38, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0x01,
            0xa0, 3, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
        ];
        jpeg.extend([0xff, 0xe1]);
        jpeg.extend(((exif.len() + 2) as u16).to_be_bytes());
        jpeg.extend(exif);
        jpeg.extend([
            0xff, 0xc0, 0, 17, 8, 4, 0, 6, 0, 3, 1, 0x11, 0, 2, 0x11, 0, 3, 0x11, 0, 0xff, 0xd9,
        ]);
        jpeg
    }

    fn capture_metadata() -> CaptureMetadata {
        CaptureMetadata {
            device_id: "remote-camera".into(),
            selected_format: camera_core::SelectedCaptureFormat {
                width: 1536,
                height: 1024,
                fps: camera_core::RationalRate {
                    numerator: 1,
                    denominator: 1,
                },
            },
        }
    }

    fn derivative_provenance(parent_resource_id: String) -> DerivativeProvenance {
        DerivativeProvenance {
            parent_resource_id,
            render_snapshot: RenderProfileSnapshot {
                schema_version: 1,
                pipeline_id: "peer-film-preview".into(),
                pipeline_sha256: "1".repeat(64),
                profiles: vec![ProfileSnapshotEntry {
                    id: "film.synthetic".into(),
                    kind: ProfileKind::Film,
                    profile_version: "1.0.0".into(),
                    content_sha256: "2".repeat(64),
                }],
                snapshot_sha256: "3".repeat(64),
            },
            engine_version: "film-core/0.1.0+peer-test".into(),
            seed: 42,
        }
    }

    #[test]
    fn indexed_original_receive_moves_incomplete_to_finalized_media() {
        let data = jpeg_fixture();
        let (root, mut transfer) = receive_fixture(&data);
        let resource = TransferResource {
            resource_id: "remote:original".into(),
            role: TransferResourceRole::Original,
            derivative_provenance: None,
            manifest: transfer.manifest.clone(),
        };
        let mut receive = IndexedOriginalReceive::create_or_resume(
            &root,
            resource,
            capture_metadata(),
            u64::MAX,
            DEFAULT_RECEIVE_RESERVE_BYTES,
        )
        .unwrap();
        let initial = MediaIndex::new(&root).list().unwrap();
        assert_eq!(initial.len(), 1);
        assert_eq!(initial[0].state, camera_core::AssetState::Incomplete);

        let ack = receive.write_chunk(0, &data).unwrap();
        transfer.acknowledge(ack).unwrap();
        let asset = transfer.finalize_indexed_original(receive).unwrap();
        assert_eq!(asset.state, camera_core::AssetState::Finalized);
        let final_entries = MediaIndex::new(&root).list().unwrap();
        assert_eq!(final_entries.len(), 1);
        assert_eq!(final_entries[0].state, camera_core::AssetState::Finalized);
        assert_eq!(final_entries[0].asset.as_ref(), Some(&asset));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn indexed_original_receive_rejects_derivative_role() {
        let data = jpeg_fixture();
        let (root, transfer) = receive_fixture(&data);
        let resource = TransferResource {
            resource_id: "remote:processed".into(),
            role: TransferResourceRole::Derivative {
                purpose: DerivativePurpose::Processed,
            },
            derivative_provenance: None,
            manifest: transfer.manifest,
        };
        assert!(matches!(
            IndexedOriginalReceive::create_or_resume(
                &root,
                resource,
                capture_metadata(),
                u64::MAX,
                DEFAULT_RECEIVE_RESERVE_BYTES,
            ),
            Err(TransferError::InvalidSelection)
        ));
        assert!(!root.exists());
    }

    #[test]
    fn indexed_derivative_receive_preserves_provenance_on_parent_asset() {
        let data = jpeg_fixture();
        let (source_root, _) = receive_fixture(b"derivative-source");
        fs::create_dir_all(&source_root).unwrap();
        let source_original_path = source_root.join("original.jpg");
        let source_derivative_path = source_root.join("processed.jpg");
        fs::write(&source_original_path, &data).unwrap();
        fs::write(&source_derivative_path, &data).unwrap();
        let mut source_asset = CapturedAsset::from_probed_resource(
            "shared-asset".into(),
            CapturedMediaType::Photo,
            probe_media_resource(&source_original_path, CapturedMediaType::Photo).unwrap(),
            source_original_path,
            capture_metadata(),
        )
        .unwrap();
        let provenance = derivative_provenance(source_asset.original_resource_id.clone());
        source_asset
            .add_derivative(
                "shared-asset:processed".into(),
                DerivativePurpose::Processed,
                probe_media_resource(&source_derivative_path, CapturedMediaType::Photo).unwrap(),
                provenance.clone(),
            )
            .unwrap();
        let bundle = AssetTransferManifest::from_captured_asset(
            &source_asset,
            AssetSelection::Derivatives {
                resource_ids: vec!["shared-asset:processed".into()],
            },
            "derivative-share",
            MIN_CHUNK_BYTES,
            MetadataPolicy::Preserve,
        )
        .unwrap();
        let transfer_resource = bundle.resources.into_iter().next().unwrap();
        assert_eq!(
            transfer_resource.derivative_provenance.as_ref(),
            Some(&provenance)
        );

        let (receive_root, _) = receive_fixture(b"derivative-receive");
        fs::create_dir_all(&receive_root).unwrap();
        let local_original_path = receive_root.join("local-original.jpg");
        fs::write(&local_original_path, &data).unwrap();
        let parent_asset = CapturedAsset::from_probed_resource(
            "shared-asset".into(),
            CapturedMediaType::Photo,
            probe_media_resource(&local_original_path, CapturedMediaType::Photo).unwrap(),
            local_original_path,
            capture_metadata(),
        )
        .unwrap();
        MediaIndex::new(&receive_root)
            .persist_finalized(&parent_asset)
            .unwrap();

        let mut transfer = session();
        transfer.manifest = transfer_resource.manifest.clone();
        transfer.approve("482913", 5_000).unwrap();
        let peers = capabilities(vec![TransportKind::LocalNetwork]);
        transfer.negotiate(&peers, &peers).unwrap();
        let mut receive = IndexedDerivativeReceive::create_or_resume(
            &receive_root,
            transfer_resource,
            parent_asset,
            u64::MAX,
            DEFAULT_RECEIVE_RESERVE_BYTES,
        )
        .unwrap();
        let ack = receive.write_chunk(0, &data).unwrap();
        transfer.acknowledge(ack).unwrap();
        let finalized = transfer.finalize_indexed_derivative(receive).unwrap();
        assert_eq!(finalized.derivatives.len(), 1);
        assert_eq!(finalized.derivatives[0].provenance, provenance);
        assert_eq!(
            finalized.derivatives[0].resource_id,
            "shared-asset:processed"
        );
        let indexed = MediaIndex::new(&receive_root).list().unwrap();
        assert_eq!(indexed.len(), 1);
        assert_eq!(indexed[0].asset.as_ref(), Some(&finalized));
        fs::remove_dir_all(source_root).unwrap();
        fs::remove_dir_all(receive_root).unwrap();
    }

    #[test]
    fn bundle_coordinator_finalizes_original_before_mapped_derivative() {
        let data = jpeg_fixture();
        let (source_root, _) = receive_fixture(b"bundle-source");
        fs::create_dir_all(&source_root).unwrap();
        let original_path = source_root.join("bundle-original.jpg");
        let derivative_path = source_root.join("bundle-processed.jpg");
        fs::write(&original_path, &data).unwrap();
        fs::write(&derivative_path, &data).unwrap();
        let mut source_asset = CapturedAsset::from_probed_resource(
            "source-bundle".into(),
            CapturedMediaType::Photo,
            probe_media_resource(&original_path, CapturedMediaType::Photo).unwrap(),
            original_path,
            capture_metadata(),
        )
        .unwrap();
        source_asset
            .add_derivative(
                "source-bundle:processed".into(),
                DerivativePurpose::Processed,
                probe_media_resource(&derivative_path, CapturedMediaType::Photo).unwrap(),
                derivative_provenance(source_asset.original_resource_id.clone()),
            )
            .unwrap();
        let bundle = AssetTransferManifest::from_captured_asset(
            &source_asset,
            AssetSelection::OriginalAndDerivatives {
                resource_ids: vec!["source-bundle:processed".into()],
            },
            "bundle-share",
            MIN_CHUNK_BYTES,
            MetadataPolicy::Preserve,
        )
        .unwrap();
        let mut coordinator =
            BundleReceiveCoordinator::new(bundle, "received-bundle".into()).unwrap();
        let (receive_root, _) = receive_fixture(b"bundle-receive");

        assert!(matches!(
            coordinator.prepare_derivative_receive(
                &receive_root,
                "source-bundle:processed",
                source_asset.clone(),
                session().invitation,
                u64::MAX,
                DEFAULT_RECEIVE_RESERVE_BYTES,
            ),
            Err(TransferError::BundleDependencyNotFinalized)
        ));

        let (mut original_session, mut original_receive) = coordinator
            .prepare_original_receive(
                &receive_root,
                session().invitation,
                u64::MAX,
                DEFAULT_RECEIVE_RESERVE_BYTES,
            )
            .unwrap();
        original_session.approve("482913", 5_000).unwrap();
        let peers = capabilities(vec![TransportKind::LocalNetwork]);
        original_session.negotiate(&peers, &peers).unwrap();
        let ack = original_receive.write_chunk(0, &data).unwrap();
        original_session.acknowledge(ack).unwrap();
        let parent = original_session
            .finalize_indexed_original(original_receive)
            .unwrap();
        coordinator.mark_original_finalized(&parent).unwrap();

        let (mut derivative_session, mut derivative_receive) = coordinator
            .prepare_derivative_receive(
                &receive_root,
                "source-bundle:processed",
                parent,
                session().invitation,
                u64::MAX,
                DEFAULT_RECEIVE_RESERVE_BYTES,
            )
            .unwrap();
        derivative_session.approve("482913", 5_000).unwrap();
        derivative_session.negotiate(&peers, &peers).unwrap();
        let ack = derivative_receive.write_chunk(0, &data).unwrap();
        derivative_session.acknowledge(ack).unwrap();
        let completed = derivative_session
            .finalize_indexed_derivative(derivative_receive)
            .unwrap();
        coordinator
            .mark_derivative_finalized("source-bundle:processed", &completed)
            .unwrap();

        assert_eq!(completed.id, "received-bundle");
        assert_eq!(completed.derivatives.len(), 1);
        assert_eq!(
            completed.derivatives[0].provenance.parent_resource_id,
            completed.original_resource_id
        );
        assert_eq!(
            coordinator
                .source_to_local_resource_ids()
                .get("source-bundle:original")
                .map(String::as_str),
            Some("received-bundle:original")
        );
        assert_eq!(
            coordinator
                .source_to_local_resource_ids()
                .get("source-bundle:processed")
                .map(String::as_str),
            Some("received-bundle:peer-1")
        );
        let indexed = MediaIndex::new(&receive_root).list().unwrap();
        assert_eq!(indexed.len(), 1);
        assert_eq!(indexed[0].asset.as_ref(), Some(&completed));
        fs::remove_dir_all(source_root).unwrap();
        fs::remove_dir_all(receive_root).unwrap();
    }

    #[test]
    fn asset_transfer_manifest_hashes_original_and_rejects_unknown_derivative() {
        let data = jpeg_fixture();
        let (root, _) = receive_fixture(&data);
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.jpg");
        fs::write(&source, &data).unwrap();
        let resource = probe_media_resource(&source, CapturedMediaType::Photo).unwrap();
        let asset = CapturedAsset::from_probed_resource(
            "source-asset".into(),
            CapturedMediaType::Photo,
            resource,
            source,
            capture_metadata(),
        )
        .unwrap();
        let bundle = AssetTransferManifest::from_captured_asset(
            &asset,
            AssetSelection::Original,
            "share-1",
            MIN_CHUNK_BYTES,
            MetadataPolicy::Preserve,
        )
        .unwrap();
        assert_eq!(bundle.resources.len(), 1);
        assert_eq!(
            bundle.resources[0].manifest.sha256_hex,
            hex::encode(Sha256::digest(&data))
        );
        assert!(matches!(
            AssetTransferManifest::from_captured_asset(
                &asset,
                AssetSelection::Derivatives {
                    resource_ids: vec!["missing".into()]
                },
                "share-2",
                MIN_CHUNK_BYTES,
                MetadataPolicy::Preserve,
            ),
            Err(TransferError::InvalidSelection)
        ));
        assert!(matches!(
            AssetTransferManifest::from_captured_asset(
                &asset,
                AssetSelection::Original,
                "share-3",
                MIN_CHUNK_BYTES,
                MetadataPolicy::StripLocation,
            ),
            Err(TransferError::MetadataSanitizationRequired)
        ));
        let _ = fs::remove_dir_all(root);
    }

    fn jpeg_segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut segment = vec![0xff, marker];
        segment.extend(((payload.len() + 2) as u16).to_be_bytes());
        segment.extend(payload);
        segment
    }

    #[test]
    fn jpeg_sanitizer_removes_private_metadata_and_preserves_color_profile() {
        let (root, _) = receive_fixture(b"sanitizer");
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.jpg");
        let destination = root.join("sanitized.jpg");
        let fixture = jpeg_fixture();
        let mut input = fixture[..2].to_vec();
        input.extend(jpeg_segment(0xe2, b"ICC_PROFILE\0profile-data"));
        input.extend(jpeg_segment(0xed, b"IPTC private location"));
        input.extend(jpeg_segment(0xfe, b"device serial comment"));
        input.extend_from_slice(&fixture[2..]);
        fs::write(&source, &input).unwrap();

        let original_resource = probe_media_resource(&source, CapturedMediaType::Photo).unwrap();
        let asset = CapturedAsset::from_probed_resource(
            "private-source".into(),
            CapturedMediaType::Photo,
            original_resource,
            source.clone(),
            capture_metadata(),
        )
        .unwrap();
        let sanitized = sanitize_jpeg_for_transfer(
            &source,
            &destination,
            MetadataPolicy::StripDeviceAndLocation,
        )
        .unwrap();
        let output = fs::read(&destination).unwrap();
        assert!(
            output
                .windows(b"ICC_PROFILE\0".len())
                .any(|part| part == b"ICC_PROFILE\0")
        );
        assert!(!output.windows(4).any(|part| part == b"Exif"));
        assert!(!output.windows(4).any(|part| part == b"IPTC"));
        assert!(!output.windows(6).any(|part| part == b"serial"));
        assert_eq!(sanitized.report.removed_segments, 3);
        assert_eq!(
            sanitized.report.sha256_hex,
            hex::encode(Sha256::digest(&output))
        );
        let resource = probe_media_resource(&destination, CapturedMediaType::Photo).unwrap();
        assert_eq!((resource.pixel_width, resource.pixel_height), (1536, 1024));
        assert!(!resource.orientation_explicit);
        let bundle = AssetTransferManifest::from_sanitized_jpeg_original(
            &asset,
            &sanitized,
            "private-share",
            MIN_CHUNK_BYTES,
        )
        .unwrap();
        assert_eq!(bundle.resources.len(), 1);
        assert_eq!(
            bundle.resources[0].manifest.metadata_policy,
            MetadataPolicy::StripDeviceAndLocation
        );
        assert_eq!(
            bundle.resources[0].manifest.sha256_hex,
            sanitized.report.sha256_hex
        );

        fs::write(&destination, jpeg_fixture()).unwrap();
        assert!(matches!(
            AssetTransferManifest::from_sanitized_jpeg_original(
                &asset,
                &sanitized,
                "mutated-share",
                MIN_CHUNK_BYTES,
            ),
            Err(TransferError::SanitizedSourceMismatch)
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn jpeg_sanitizer_rejects_location_only_policy_and_malformed_input() {
        let (root, _) = receive_fixture(b"sanitizer-errors");
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source.jpg");
        fs::write(&source, jpeg_fixture()).unwrap();
        assert!(matches!(
            sanitize_jpeg_for_transfer(
                &source,
                &root.join("location-only.jpg"),
                MetadataPolicy::StripLocation,
            ),
            Err(TransferError::UnsupportedMetadataPolicy)
        ));
        fs::write(&source, [0xff, 0xd8, 0xff, 0xe1, 0, 20, 1, 2]).unwrap();
        let malformed = root.join("malformed.jpg");
        assert!(matches!(
            sanitize_jpeg_for_transfer(&source, &malformed, MetadataPolicy::StripDeviceAndLocation,),
            Err(TransferError::UnsupportedMediaForSanitization) | Err(TransferError::MalformedJpeg)
        ));
        assert!(!malformed.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
