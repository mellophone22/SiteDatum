//! Dormant private vault root. No application root is created automatically.
//! Native ACL validation and no-delete ancestor handles; no permissions repair.
#![allow(dead_code)]
use super::sync_v2_vault_file_prototype::{Archive, Refusal};
use super::sync_v2_vault_storage::{self, ExpectedBundle};
use std::{
    fs::{File, OpenOptions},
    os::windows::{
        ffi::OsStrExt,
        fs::{MetadataExt, OpenOptionsExt},
        io::AsRawHandle,
    },
    path::{Component, Path, PathBuf, Prefix},
};
use uuid::Uuid;
use windows_sys::Win32::{
    Foundation::{CloseHandle, LocalFree},
    Security::{Authorization::*, *},
    Storage::FileSystem::{CreateDirectoryW, FILE_ALL_ACCESS},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};
type Result<T> = std::result::Result<T, Refusal>;
struct NativeMemory(*mut core::ffi::c_void);
impl Drop for NativeMemory {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                LocalFree(self.0);
            }
        }
    }
}
struct Profile {
    token: Box<[usize]>,
    length: usize,
}
impl Profile {
    fn current() -> Result<Self> {
        unsafe {
            let mut handle = std::ptr::null_mut();
            if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) == 0 {
                return Err(Refusal::Storage);
            }
            let mut length = 0;
            GetTokenInformation(handle, TokenUser, std::ptr::null_mut(), 0, &mut length);
            if length < std::mem::size_of::<TOKEN_USER>() as u32 || length > 16384 {
                CloseHandle(handle);
                return Err(Refusal::Invalid);
            }
            // usize allocation supplies TOKEN_USER alignment, unlike a Vec<u8> cast.
            let mut token = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())]
                .into_boxed_slice();
            let success = GetTokenInformation(
                handle,
                TokenUser,
                token.as_mut_ptr().cast(),
                length,
                &mut length,
            );
            CloseHandle(handle);
            if success == 0 {
                return Err(Refusal::Storage);
            }
            if length < std::mem::size_of::<TOKEN_USER>() as u32
                || length as usize > token.len() * std::mem::size_of::<usize>()
            {
                return Err(Refusal::Invalid);
            }
            Ok(Self {
                token,
                length: length as usize,
            })
        }
    }
    fn sid(&self) -> PSID {
        // SAFETY: TOKEN_USER is aligned and initialized by Windows. Its SID points
        // into this immutable boxed allocation, which is never resized or moved.
        unsafe { (*(self.token.as_ptr() as *const TOKEN_USER)).User.Sid }
    }
    fn descriptor(&self) -> Result<NativeMemory> {
        self.sid_bytes()?; // Bound the immutable token SID before native conversion.
        unsafe {
            let mut ptr = std::ptr::null_mut();
            if ConvertSidToStringSidW(self.sid(), &mut ptr) == 0 || ptr.is_null() {
                return Err(Refusal::Storage);
            }
            let owned = NativeMemory(ptr.cast());
            let mut length = 0;
            while length < 256 && *ptr.add(length) != 0 {
                length += 1;
            }
            if length == 256 {
                return Err(Refusal::Invalid);
            }
            let sid = String::from_utf16(std::slice::from_raw_parts(ptr, length))
                .map_err(|_| Refusal::Invalid)?;
            drop(owned);
            let text = format!("O:{sid}D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;{sid})");
            let wide: Vec<u16> = text.encode_utf16().chain(Some(0)).collect();
            let mut sd = std::ptr::null_mut();
            if ConvertStringSecurityDescriptorToSecurityDescriptorW(
                wide.as_ptr(),
                1,
                &mut sd,
                std::ptr::null_mut(),
            ) == 0
            {
                return Err(Refusal::Storage);
            }
            Ok(NativeMemory(sd))
        }
    }
    fn sid_bytes(&self) -> Result<&[u8]> {
        let base = self.token.as_ptr() as usize;
        let offset = (self.sid() as usize)
            .checked_sub(base)
            .ok_or(Refusal::Invalid)?;
        // SAFETY: only the initialized, bounded native output is exposed.
        let bytes =
            unsafe { std::slice::from_raw_parts(self.token.as_ptr().cast::<u8>(), self.length) };
        bounded_sid(bytes.get(offset..).ok_or(Refusal::Invalid)?)
    }
    fn verify(&self, file: &File) -> Result<()> {
        let mut length = 0u32;
        let information = OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION;
        // Query a caller-owned buffer size; never trust lengths read from a native SD.
        unsafe {
            GetKernelObjectSecurity(
                file.as_raw_handle() as _,
                information,
                std::ptr::null_mut(),
                0,
                &mut length,
            );
        }
        if !(20..=65536).contains(&length) {
            return Err(Refusal::Invalid);
        }
        let capacity = length;
        let mut storage = vec![0usize; (capacity as usize).div_ceil(std::mem::size_of::<usize>())]
            .into_boxed_slice();
        // SAFETY: aligned, initialized caller-owned allocation; capacity is exact.
        if unsafe {
            GetKernelObjectSecurity(
                file.as_raw_handle() as _,
                information,
                storage.as_mut_ptr().cast(),
                capacity,
                &mut length,
            )
        } == 0
        {
            return Err(Refusal::Storage);
        }
        if length > capacity || length < 20 {
            return Err(Refusal::Invalid);
        }
        // SAFETY: length was checked against the caller-owned initialized capacity.
        let bytes =
            unsafe { std::slice::from_raw_parts(storage.as_ptr().cast::<u8>(), length as usize) };
        verify_descriptor(bytes, self.sid_bytes()?)
    }
}
// No native SID/ACE comparison runs on untrusted pointers. Every field is read
// from a Rust slice bounded by the caller-owned descriptor allocation.
fn bounded_sid(bytes: &[u8]) -> Result<&[u8]> {
    if bytes.len() < 8 || bytes[0] != 1 || bytes[1] > 15 {
        return Err(Refusal::Invalid);
    }
    bytes
        .get(..8 + 4 * bytes[1] as usize)
        .ok_or(Refusal::Invalid)
}
fn verify_descriptor(bytes: &[u8], user: &[u8]) -> Result<()> {
    const SYSTEM: &[u8] = &[1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0];
    if bytes.len() < 20 || bytes[0] != 1 {
        return Err(Refusal::Invalid);
    }
    let control = u16::from_le_bytes([bytes[2], bytes[3]]);
    if control & 0x9004 != 0x9004 {
        return Err(Refusal::Invalid);
    }
    let offset = |i| -> Result<usize> {
        let n =
            u32::from_le_bytes(bytes[i..i + 4].try_into().map_err(|_| Refusal::Invalid)?) as usize;
        if n < 20 || n % 4 != 0 || n >= bytes.len() {
            Err(Refusal::Invalid)
        } else {
            Ok(n)
        }
    };
    if bounded_sid(&bytes[offset(4)?..])? != user {
        return Err(Refusal::Invalid);
    }
    let raw = &bytes[offset(16)?..];
    if raw.len() < 8 || ![2, 4].contains(&raw[0]) || raw[1] != 0 || raw[6..8] != [0, 0] {
        return Err(Refusal::Invalid);
    }
    let size = u16::from_le_bytes([raw[2], raw[3]]) as usize;
    let acl = raw
        .get(..size)
        .filter(|a| a.len() >= 8)
        .ok_or(Refusal::Invalid)?;
    if u16::from_le_bytes([acl[4], acl[5]]) != 2 {
        return Err(Refusal::Invalid);
    }
    let mut position = 8;
    let (mut user_seen, mut system_seen) = (false, false);
    for _ in 0..2 {
        let head = acl.get(position..position + 4).ok_or(Refusal::Invalid)?;
        let size = u16::from_le_bytes([head[2], head[3]]) as usize;
        if head[0] != 0 || head[1] != 3 || size < 16 || size % 4 != 0 {
            return Err(Refusal::Invalid);
        }
        let ace = acl.get(position..position + size).ok_or(Refusal::Invalid)?;
        if u32::from_le_bytes(ace[4..8].try_into().unwrap()) != FILE_ALL_ACCESS {
            return Err(Refusal::Invalid);
        }
        let sid = bounded_sid(&ace[8..])?;
        if sid.len() + 8 != size {
            return Err(Refusal::Invalid);
        }
        if sid == user && !user_seen {
            user_seen = true;
        } else if sid == SYSTEM && !system_seen {
            system_seen = true;
        } else {
            return Err(Refusal::Invalid);
        }
        position += size;
    }
    if !user_seen || !system_seen || position != acl.len() {
        return Err(Refusal::Invalid);
    }
    Ok(())
}
fn locked_chain(boundary: &Path, path: &Path) -> Result<Vec<File>> {
    if !boundary.is_absolute() || !path.starts_with(boundary) {
        return Err(Refusal::Invalid);
    }
    if !path.is_absolute() {
        return Err(Refusal::Invalid);
    }
    let mut partial = PathBuf::new();
    let mut held = Vec::new();
    for component in path.components() {
        match component {
            Component::Prefix(p)
                if matches!(p.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) =>
            {
                partial.push(p.as_os_str())
            }
            Component::RootDir => partial.push(component.as_os_str()),
            Component::Normal(part) => {
                let text = part.to_str().ok_or(Refusal::Invalid)?;
                if text.contains([':', '\0']) || text.ends_with(['.', ' ']) {
                    return Err(Refusal::Invalid);
                }
                partial.push(part);
            }
            _ => return Err(Refusal::Invalid),
        }
        if partial.has_root() && partial.starts_with(boundary) {
            let f = OpenOptions::new()
                .read(true)
                .share_mode(1)
                .custom_flags(0x02000000 | 0x00200000)
                .open(&partial)
                .map_err(|_| Refusal::Storage)?;
            let m = f.metadata().map_err(|_| Refusal::Storage)?;
            if !m.is_dir() || m.file_attributes() & 0x400 != 0 {
                return Err(Refusal::Invalid);
            }
            held.push(f);
            if held.len() > 64 {
                return Err(Refusal::Invalid);
            }
        }
    }
    if held.is_empty() {
        return Err(Refusal::Invalid);
    }
    Ok(held)
}
fn identity(file: &File) -> Result<(u32, u32, u32)> {
    let mut info = std::mem::MaybeUninit::<
        windows_sys::Win32::Storage::FileSystem::BY_HANDLE_FILE_INFORMATION,
    >::uninit();
    // SAFETY: valid open handle and writable structure; read only after success.
    if unsafe {
        windows_sys::Win32::Storage::FileSystem::GetFileInformationByHandle(
            file.as_raw_handle() as _,
            info.as_mut_ptr(),
        )
    } == 0
    {
        return Err(Refusal::Storage);
    }
    let info = unsafe { info.assume_init() };
    Ok((
        info.dwVolumeSerialNumber,
        info.nFileIndexHigh,
        info.nFileIndexLow,
    ))
}
pub(super) struct PrivateVaultDirectory {
    path: PathBuf,
    boundary: PathBuf,
    boundary_identity: (u32, u32, u32),
    root_identity: (u32, u32, u32),
    profile: Profile,
}
impl PrivateVaultDirectory {
    // Boundary is a caller-configured trusted application-data directory. We do
    // not claim to defend replacement of its ancestors by an administrator.
    pub(super) fn create(boundary: &Path, path: &Path) -> Result<Self> {
        let parent = path.parent().ok_or(Refusal::Invalid)?;
        let mut held = locked_chain(boundary, parent)?;
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or(Refusal::Invalid)?;
        if name.contains([':', '\0']) || name.ends_with(['.', ' ']) {
            return Err(Refusal::Invalid);
        }
        let profile = Profile::current()?;
        let descriptor = profile.descriptor()?;
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: 0,
        };
        if unsafe { CreateDirectoryW(wide.as_ptr(), &attributes) } == 0 {
            return Err(Refusal::Storage);
        }
        let leaf = OpenOptions::new()
            .read(true)
            .share_mode(1)
            .custom_flags(0x02000000 | 0x00200000)
            .open(path)
            .map_err(|_| Refusal::Storage)?;
        let m = leaf.metadata().map_err(|_| Refusal::Storage)?;
        if !m.is_dir() || m.file_attributes() & 0x400 != 0 {
            return Err(Refusal::Invalid);
        }
        profile.verify(&leaf)?;
        held.push(leaf);
        Self::remember(boundary, path, profile, &held)
    }
    fn remember(boundary: &Path, path: &Path, profile: Profile, held: &[File]) -> Result<Self> {
        Ok(Self {
            path: path.to_owned(),
            boundary: boundary.to_owned(),
            boundary_identity: identity(held.first().ok_or(Refusal::Invalid)?)?,
            root_identity: identity(held.last().ok_or(Refusal::Invalid)?)?,
            profile,
        })
    }
    pub(super) fn open(boundary: &Path, path: &Path) -> Result<Self> {
        let held = locked_chain(boundary, path)?;
        let profile = Profile::current()?;
        profile.verify(held.last().ok_or(Refusal::Invalid)?)?;
        Self::remember(boundary, path, profile, &held)
    }
    fn with_destination<T>(
        &self,
        id: Uuid,
        operation: impl FnOnce(&Path) -> Result<T>,
    ) -> Result<T> {
        if id.is_nil() {
            return Err(Refusal::Invalid);
        }
        // Lease acquired only for this operation, including storage's reopen.
        let held = locked_chain(&self.boundary, &self.path)?;
        if identity(held.first().ok_or(Refusal::Invalid)?)? != self.boundary_identity
            || identity(held.last().ok_or(Refusal::Invalid)?)? != self.root_identity
        {
            return Err(Refusal::Invalid);
        }
        self.profile.verify(held.last().ok_or(Refusal::Invalid)?)?;
        let result = operation(&self.path.join(format!("{id}.vault")));
        drop(held);
        result
    }
    pub(super) fn create_generation(&self, id: Uuid, archive: &Archive) -> Result<ExpectedBundle> {
        self.with_destination(id, |path| {
            sync_v2_vault_storage::create_local(path, archive)
        })
    }
    pub(super) fn open_generation(&self, id: Uuid, pin: ExpectedBundle) -> Result<Archive> {
        self.with_destination(id, |path| sync_v2_vault_storage::open_local(path, pin))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeroize::Zeroizing;

    fn fictional_descriptor() -> (Vec<u8>, Vec<u8>) {
        let user = vec![1, 1, 0, 0, 0, 0, 0, 5, 42, 0, 0, 0];
        let system = [1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0];
        let mut sd = vec![0; 20];
        sd[0] = 1;
        sd[2..4].copy_from_slice(&0x9004u16.to_le_bytes());
        sd[4..8].copy_from_slice(&20u32.to_le_bytes());
        sd.extend_from_slice(&user);
        let offset = sd.len() as u32;
        sd[16..20].copy_from_slice(&offset.to_le_bytes());
        sd.extend_from_slice(&[2, 0, 48, 0, 2, 0, 0, 0]);
        for sid in [&user[..], &system[..]] {
            sd.extend_from_slice(&[0, 3, 20, 0]);
            sd.extend_from_slice(&FILE_ALL_ACCESS.to_le_bytes());
            sd.extend_from_slice(sid);
        }
        (sd, user)
    }
    #[test]
    fn malformed_ace_owner_sid_and_acl_lengths_are_bounded_before_comparison() {
        let (valid, user) = fictional_descriptor();
        assert!(verify_descriptor(&valid, &user).is_ok());
        for end in 0..valid.len() {
            assert_eq!(
                verify_descriptor(&valid[..end], &user),
                Err(Refusal::Invalid)
            );
        }
        for (offset, value) in [
            (62, 12),
            (69, 15),
            (34, 255),
            (36, 3),
            (32, 1),
            (41, 0),
            (40, 1),
            (44, 0),
        ] {
            let mut malformed = valid.clone();
            malformed[offset] = value;
            assert_eq!(verify_descriptor(&malformed, &user), Err(Refusal::Invalid));
        }
        let mut owner = valid.clone();
        owner[21] = 15;
        assert_eq!(verify_descriptor(&owner, &user), Err(Refusal::Invalid));
        let mut undersized = valid.clone();
        undersized[62..64].copy_from_slice(&12u16.to_le_bytes());
        undersized[69] = 15;
        assert_eq!(verify_descriptor(&undersized, &user), Err(Refusal::Invalid));
    }
    #[test]
    fn truncated_hostile_ace_at_guard_page_boundary_refuses_without_native_sid_read() {
        use windows_sys::Win32::System::Memory::*;
        let (mut sd, user) = fictional_descriptor();
        sd[62..64].copy_from_slice(&12u16.to_le_bytes());
        sd[69] = 15;
        sd.truncate(72);
        sd[34..36].copy_from_slice(&40u16.to_le_bytes());
        // Place the last malformed ACE against an inaccessible page. Any read
        // beyond the actual caller-owned buffer would fault this test process.
        unsafe {
            let base = VirtualAlloc(
                std::ptr::null(),
                8192,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            );
            assert!(!base.is_null());
            let mut old = 0;
            assert_ne!(
                VirtualProtect(
                    (base as *mut u8).add(4096).cast(),
                    4096,
                    PAGE_NOACCESS,
                    &mut old
                ),
                0
            );
            let start = (base as *mut u8).add(4096 - sd.len());
            std::ptr::copy_nonoverlapping(sd.as_ptr(), start, sd.len());
            let result = verify_descriptor(std::slice::from_raw_parts(start, sd.len()), &user);
            assert_ne!(VirtualFree(base, 0, MEM_RELEASE), 0);
            assert_eq!(result, Err(Refusal::Invalid));
        }
    }
    #[test]
    fn operation_scoped_lease_has_zero_idle_handles_and_releases_after_failure() {
        if let Some(value) = std::env::var_os("SITEDATUM_LEASE_PROBE_PATH") {
            let target = PathBuf::from(value);
            assert_eq!(target.parent(), Some(std::env::temp_dir().as_path()));
            assert!(Uuid::parse_str(
                target
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .strip_prefix("sitedatum-lease-")
                    .unwrap()
            )
            .is_ok());
            let opened = OpenOptions::new()
                .write(true)
                .share_mode(3)
                .custom_flags(0x02000000)
                .open(&target);
            assert_eq!(
                opened.is_ok(),
                std::env::var("SITEDATUM_LEASE_PROBE_EXPECT").unwrap() == "allowed"
            );
            return;
        }
        if std::env::var("SITEDATUM_LEASE_ISOLATED").as_deref() != Ok("1") {
            let output=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","sync_v2_vault_directory::tests::operation_scoped_lease_has_zero_idle_handles_and_releases_after_failure"]).env("SITEDATUM_LEASE_ISOLATED","1").output().unwrap();
            assert!(
                output.status.success(),
                "isolated lease probe failed: {}",
                String::from_utf8_lossy(&output.stdout)
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
            return;
        }
        use windows_sys::Win32::System::Threading::GetProcessHandleCount;
        fn probe(path: &Path, allowed: bool) {
            let output=std::process::Command::new(std::env::current_exe().unwrap()).args(["--exact","sync_v2_vault_directory::tests::operation_scoped_lease_has_zero_idle_handles_and_releases_after_failure"]).env("SITEDATUM_LEASE_PROBE_PATH",path).env("SITEDATUM_LEASE_PROBE_EXPECT",if allowed {"allowed"} else {"blocked"}).output().unwrap();
            assert!(
                output.status.success(),
                "cross-process sharing probe failed"
            );
            assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
        }
        fn count() -> u32 {
            let mut n = 0;
            assert_ne!(
                unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut n) },
                0
            );
            n
        }
        let parent = std::env::temp_dir().join(format!("sitedatum-lease-{}", Uuid::new_v4()));
        std::fs::create_dir(&parent).unwrap();
        let path = parent.join("vault");
        // Warm up Windows' lazy security-provider initialization before measuring
        // directory handle lifetime; those process-global cached handles are not leases.
        let warm = PrivateVaultDirectory::create(&parent, &path).unwrap();
        warm.with_destination(Uuid::new_v4(), |_| Ok(())).unwrap();
        drop(warm);
        probe(&parent, true); // Warm subprocess/pipe runtime caches as well.
        let baseline = count();
        let dir = PrivateVaultDirectory::open(&parent, &path).unwrap();
        let idle = count();
        assert_eq!(idle, baseline);
        probe(&parent, true);
        let result: Result<()> = dir.with_destination(Uuid::new_v4(), |_| {
            assert_eq!(count(), idle + 2);
            probe(&parent, false);
            assert!(OpenOptions::new()
                .write(true)
                .share_mode(3)
                .custom_flags(0x02000000)
                .open(&parent)
                .is_err());
            // Measure rather than assume installer compatibility: child creation
            // works, but Windows also refuses a child rename during this lease.
            let child = parent.join("installer.fixture");
            let moved = parent.join("installer-moved.fixture");
            File::create(&child).unwrap();
            assert!(std::fs::rename(&child, &moved).is_err());
            std::fs::remove_file(&child).unwrap();
            // No volume-root or above-boundary handles are leased.
            let outside = OpenOptions::new()
                .write(true)
                .share_mode(3)
                .custom_flags(0x02000000)
                .open(std::env::temp_dir())
                .unwrap();
            drop(outside);
            Err(Refusal::Storage)
        });
        assert_eq!(result, Err(Refusal::Storage));
        assert_eq!(count(), idle);
        probe(&parent, true);
        let child = parent.join("after.fixture");
        let moved = parent.join("after-moved.fixture");
        File::create(&child).unwrap();
        std::fs::rename(&child, &moved).unwrap();
        std::fs::remove_file(&moved).unwrap();
        let outside = OpenOptions::new()
            .write(true)
            .share_mode(3)
            .custom_flags(0x02000000)
            .open(&parent)
            .unwrap();
        drop(outside);
        drop(dir);
        std::fs::remove_dir(path).unwrap();
        std::fs::remove_dir(parent).unwrap();
    }
    #[test]
    fn restrictive_root_reopens_and_pins_ancestors_without_overwriting() {
        let parent =
            std::env::temp_dir().join(format!("sitedatum-private-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&parent).unwrap();
        let path = parent.join("vault");
        let dir = PrivateVaultDirectory::create(&parent, &path).unwrap();
        assert!(PrivateVaultDirectory::create(&parent, &path).is_err());
        dir.with_destination(Uuid::new_v4(), |_| {
            assert!(std::fs::rename(&parent, parent.with_extension("moved")).is_err());
            assert!(OpenOptions::new()
                .read(true)
                .write(true)
                .share_mode(3)
                .custom_flags(0x02000000 | 0x00200000)
                .open(&parent)
                .is_err());
            Ok(())
        })
        .unwrap();
        let writable = OpenOptions::new()
            .write(true)
            .share_mode(3)
            .custom_flags(0x02000000)
            .open(&parent)
            .unwrap();
        drop(writable);
        let id = Uuid::new_v4();
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let a = Archive::new(o, w, 1, vec![(1, Zeroizing::new([7; 32]))]).unwrap();
        let pin = dir.create_generation(id, &a).unwrap();
        assert!(dir.create_generation(id, &a).is_err());
        assert!(dir.open_generation(id, pin).is_ok());
        drop(dir);
        let reopened = PrivateVaultDirectory::open(path.parent().unwrap(), &path).unwrap();
        assert!(reopened.open_generation(id, pin).is_ok());
        drop(reopened);
        std::fs::remove_file(path.join(format!("{id}.vault"))).unwrap();
        std::fs::remove_dir(&path).unwrap();
        std::fs::remove_dir(&parent).unwrap();
    }
    #[test]
    fn ordinary_inherited_directory_is_refused_and_never_repaired() {
        let path = std::env::temp_dir().join(format!("sitedatum-weak-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        assert!(PrivateVaultDirectory::open(path.parent().unwrap(), &path).is_err());
        assert!(path.is_dir());
        std::fs::remove_dir(&path).unwrap();
        assert!(PrivateVaultDirectory::open(Path::new("relative"), Path::new("relative")).is_err());
        assert!(PrivateVaultDirectory::open(
            Path::new("relative"),
            Path::new(r"\\server\share\vault")
        )
        .is_err());
        let prefix = format!("sitedatum-nul-test-{}", Uuid::new_v4());
        let invalid = std::env::temp_dir().join(format!("{prefix}\0suffix"));
        assert!(PrivateVaultDirectory::create(&std::env::temp_dir(), &invalid).is_err());
        assert!(!std::env::temp_dir().join(prefix).exists());
    }
    #[test]
    fn real_junction_is_refused_as_root_and_ancestor_before_any_write() {
        use windows_sys::Win32::System::IO::DeviceIoControl;
        let parent =
            std::env::temp_dir().join(format!("sitedatum-junction-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&parent).unwrap();
        let target = parent.join("target");
        let alias = parent.join("alias");
        std::fs::create_dir(&target).unwrap();
        std::fs::create_dir(&alias).unwrap();
        let substitute: Vec<u16> = format!(r"\??\{}", target.to_str().unwrap())
            .encode_utf16()
            .collect();
        let printable: Vec<u16> = target.as_os_str().encode_wide().collect();
        let s = (substitute.len() * 2) as u16;
        let p = (printable.len() * 2) as u16;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xA0000003u32.to_le_bytes());
        bytes.extend_from_slice(&(8 + s + p + 4).to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        for n in [0, s, s + 2, p] {
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        for n in substitute
            .into_iter()
            .chain(Some(0))
            .chain(printable)
            .chain(Some(0))
        {
            bytes.extend_from_slice(&n.to_le_bytes());
        }
        let handle = OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(0)
            .custom_flags(0x02000000 | 0x00200000)
            .open(&alias)
            .unwrap();
        let mut returned = 0;
        // SAFETY: test-owned empty directory and bounded mount-point buffer.
        assert_ne!(
            unsafe {
                DeviceIoControl(
                    handle.as_raw_handle() as _,
                    0x000900a4,
                    bytes.as_ptr().cast(),
                    bytes.len() as u32,
                    std::ptr::null_mut(),
                    0,
                    &mut returned,
                    std::ptr::null_mut(),
                )
            },
            0
        );
        drop(handle);
        assert!(std::fs::symlink_metadata(&alias).unwrap().file_attributes() & 0x400 != 0);
        assert!(PrivateVaultDirectory::open(&parent, &alias).is_err());
        assert!(PrivateVaultDirectory::create(&parent, &alias.join("vault")).is_err());
        assert!(!target.join("vault").exists());
        // Remove this exact fictional junction, not its target; no recursion.
        std::fs::remove_dir(&alias).unwrap();
        std::fs::remove_dir(&target).unwrap();
        std::fs::remove_dir(&parent).unwrap();
    }
    #[test]
    fn root_permission_drift_blocks_operations_without_auto_repair() {
        let parent = std::env::temp_dir().join(format!("sitedatum-acl-drift-{}", Uuid::new_v4()));
        std::fs::create_dir(&parent).unwrap();
        let path = parent.join("vault");
        let dir = PrivateVaultDirectory::create(&parent, &path).unwrap();
        // Change ONLY this fictional directory to a permissive test DACL.
        let text: Vec<u16> = "D:P(A;OICI;FA;;;WD)"
            .encode_utf16()
            .chain(Some(0))
            .collect();
        let mut raw = std::ptr::null_mut();
        assert_ne!(
            unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    text.as_ptr(),
                    1,
                    &mut raw,
                    std::ptr::null_mut(),
                )
            },
            0
        );
        let _sd = NativeMemory(raw);
        let mut acl = std::ptr::null_mut();
        let mut present = 0;
        let mut defaulted = 0;
        assert_ne!(
            unsafe { GetSecurityDescriptorDacl(raw, &mut present, &mut acl, &mut defaulted) },
            0
        );
        let handle = OpenOptions::new()
            .access_mode(0x00040000 | 0x00020000)
            .share_mode(3)
            .custom_flags(0x02000000 | 0x00200000)
            .open(&path)
            .unwrap();
        assert_eq!(
            unsafe {
                SetSecurityInfo(
                    handle.as_raw_handle() as _,
                    SE_FILE_OBJECT,
                    DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    acl,
                    std::ptr::null(),
                )
            },
            0
        );
        assert!(dir.with_destination(Uuid::new_v4(), |_| Ok(())).is_err());
        assert!(PrivateVaultDirectory::open(path.parent().unwrap(), &path).is_err());
        drop(handle);
        drop(dir);
        std::fs::remove_dir(&path).unwrap();
        std::fs::remove_dir(&parent).unwrap();
    }
}
