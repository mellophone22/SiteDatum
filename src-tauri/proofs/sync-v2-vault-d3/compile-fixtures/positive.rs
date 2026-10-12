mod codec;
mod sync_v2_record_codec { pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991; }
fn main() { assert!(codec::Pin::from_independent_custody(1,[0;32]).is_some()); }
