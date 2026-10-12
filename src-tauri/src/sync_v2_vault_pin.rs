//! Trust-marked expected archive metadata. No freshness discovery is performed.
use crate::sync_v2_record_codec::MAX_SAFE_INTEGER;
#[derive(Clone, Copy)]
pub(crate) struct Pin {
    generation: u64,
    digest: [u8; 32],
}
impl Pin {
    // Visible only to the enclosing codec, after successful authenticated sealing.
    pub(super) fn from_verified_seal(generation: u64, digest: [u8; 32]) -> Option<Self> {
        Self::from_independent_custody(generation, digest)
    }
    /// Caller must supply independently retained expected metadata, not a hash
    /// learned from an untrusted file/provider. Bounds do not verify provenance.
    pub(crate) fn from_independent_custody(generation: u64, digest: [u8; 32]) -> Option<Self> {
        if generation == 0 || generation > MAX_SAFE_INTEGER {
            None
        } else {
            Some(Self { generation, digest })
        }
    }
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }
    pub(crate) fn digest(&self) -> [u8; 32] {
        self.digest
    }
}
