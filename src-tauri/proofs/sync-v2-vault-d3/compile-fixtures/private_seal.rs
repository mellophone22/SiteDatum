mod codec;
mod sync_v2_record_codec { pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991; }
fn main() { let _ = codec::Pin::from_verified_seal(1,[0;32]); }
