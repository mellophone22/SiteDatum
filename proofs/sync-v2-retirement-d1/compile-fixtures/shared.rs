#[path = "../authority_roles.rs"]
mod roles;
use roles::*;
struct Read;
struct Write;
static FICTIONAL_KEY: [u8; 32] = [7; 32];
impl ReadBackend for Read {
    type Error = ();
    fn select(&self, _: [u8; 16], _: [u8; 16], _: u32) -> Result<&[u8; 32], ()> {
        Ok(&FICTIONAL_KEY)
    }
}
impl WriteBackend for Write {
    type Error = ();
    fn current(&self, _: [u8; 16], _: [u8; 16]) -> Result<(u32, &[u8; 32]), ()> {
        Ok((2, &FICTIONAL_KEY))
    }
}
fn needs_writer(_: &impl WriteKeys) {}
fn needs_reader(_: &impl ReadKeys) {}
