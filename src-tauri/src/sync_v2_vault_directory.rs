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
    token: Vec<usize>,
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
            let mut token = vec![0usize; (length as usize).div_ceil(std::mem::size_of::<usize>())];
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
            Ok(Self { token })
        }
    }
    fn sid(&self) -> PSID {
        unsafe { (*(self.token.as_ptr() as *const TOKEN_USER)).User.Sid }
    }
    fn descriptor(&self) -> Result<NativeMemory> {
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
    fn verify(&self, file: &File) -> Result<()> {
        unsafe {
            let mut owner = std::ptr::null_mut();
            let mut acl = std::ptr::null_mut();
            let mut sd = std::ptr::null_mut();
            if GetSecurityInfo(
                file.as_raw_handle() as _,
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &mut owner,
                std::ptr::null_mut(),
                &mut acl,
                std::ptr::null_mut(),
                &mut sd,
            ) != 0
            {
                return Err(Refusal::Storage);
            }
            let _owned = NativeMemory(sd);
            if owner.is_null() || acl.is_null() || EqualSid(owner, self.sid()) == 0 {
                return Err(Refusal::Invalid);
            }
            let mut control = 0;
            let mut revision = 0;
            if GetSecurityDescriptorControl(sd, &mut control, &mut revision) == 0
                || control & SE_DACL_PROTECTED == 0
            {
                return Err(Refusal::Invalid);
            }
            let mut info: ACL_SIZE_INFORMATION = std::mem::zeroed();
            if GetAclInformation(
                acl,
                (&mut info as *mut ACL_SIZE_INFORMATION).cast(),
                std::mem::size_of_val(&info) as u32,
                AclSizeInformation,
            ) == 0
                || info.AceCount != 2
            {
                return Err(Refusal::Invalid);
            }
            let mut system = [0usize; 16];
            let mut length = std::mem::size_of_val(&system) as u32;
            if CreateWellKnownSid(
                WinLocalSystemSid,
                std::ptr::null_mut(),
                system.as_mut_ptr().cast(),
                &mut length,
            ) == 0
            {
                return Err(Refusal::Storage);
            }
            let mut user_seen = false;
            let mut system_seen = false;
            for index in 0..2 {
                let mut ptr = std::ptr::null_mut();
                if GetAce(acl, index, &mut ptr) == 0 || ptr.is_null() {
                    return Err(Refusal::Invalid);
                }
                let header = &*(ptr as *const ACE_HEADER);
                if header.AceType != 0
                    || header.AceFlags != 3
                    || (header.AceSize as usize) < std::mem::size_of::<ACCESS_ALLOWED_ACE>()
                {
                    return Err(Refusal::Invalid);
                }
                let ace = &*(ptr as *const ACCESS_ALLOWED_ACE);
                if ace.Mask != FILE_ALL_ACCESS {
                    return Err(Refusal::Invalid);
                }
                let sid = std::ptr::addr_of!(ace.SidStart) as PSID;
                if EqualSid(sid, self.sid()) != 0 && !user_seen {
                    user_seen = true;
                } else if EqualSid(sid, system.as_mut_ptr().cast()) != 0 && !system_seen {
                    system_seen = true;
                } else {
                    return Err(Refusal::Invalid);
                }
            }
            if !user_seen || !system_seen {
                return Err(Refusal::Invalid);
            }
            Ok(())
        }
    }
}
fn locked_chain(path: &Path) -> Result<Vec<File>> {
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
        if partial.has_root() {
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
pub(super) struct PrivateVaultDirectory {
    path: PathBuf,
    held: Vec<File>,
    profile: Profile,
}
impl PrivateVaultDirectory {
    pub(super) fn create(path: &Path) -> Result<Self> {
        let parent = path.parent().ok_or(Refusal::Invalid)?;
        // Pin every existing ancestor before creating a restricted leaf directory.
        let mut held = locked_chain(parent)?;
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
        // SAFETY: buffers/descriptor remain alive through the synchronous API call.
        if unsafe { CreateDirectoryW(wide.as_ptr(), &attributes) } == 0 {
            return Err(Refusal::Storage);
        }
        let mut leaf = locked_chain(path)?;
        let file = leaf.pop().ok_or(Refusal::Invalid)?;
        profile.verify(&file)?;
        held.push(file);
        Ok(Self {
            path: path.to_owned(),
            held,
            profile,
        })
    }
    pub(super) fn open(path: &Path) -> Result<Self> {
        let held = locked_chain(path)?;
        let profile = Profile::current()?;
        profile.verify(held.last().ok_or(Refusal::Invalid)?)?;
        Ok(Self {
            path: path.to_owned(),
            held,
            profile,
        })
    }
    fn destination(&self, id: Uuid) -> Result<PathBuf> {
        if id.is_nil() {
            return Err(Refusal::Invalid);
        }
        for directory in &self.held {
            let m = directory.metadata().map_err(|_| Refusal::Storage)?;
            if !m.is_dir() || m.file_attributes() & 0x400 != 0 {
                return Err(Refusal::Invalid);
            }
        }
        self.profile
            .verify(self.held.last().ok_or(Refusal::Invalid)?)?;
        Ok(self.path.join(format!("{id}.vault")))
    }
    pub(super) fn create_generation(&self, id: Uuid, archive: &Archive) -> Result<ExpectedBundle> {
        sync_v2_vault_storage::create_local(&self.destination(id)?, archive)
    }
    pub(super) fn open_generation(&self, id: Uuid, pin: ExpectedBundle) -> Result<Archive> {
        sync_v2_vault_storage::open_local(&self.destination(id)?, pin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zeroize::Zeroizing;
    #[test]
    fn restrictive_root_reopens_and_pins_ancestors_without_overwriting() {
        let parent =
            std::env::temp_dir().join(format!("sitedatum-private-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&parent).unwrap();
        let path = parent.join("vault");
        let dir = PrivateVaultDirectory::create(&path).unwrap();
        assert!(PrivateVaultDirectory::create(&path).is_err());
        assert!(std::fs::rename(&parent, parent.with_extension("moved")).is_err());
        assert!(OpenOptions::new()
            .read(true)
            .write(true)
            .share_mode(3)
            .custom_flags(0x02000000 | 0x00200000)
            .open(&parent)
            .is_err());
        let id = Uuid::new_v4();
        let o = Uuid::new_v4();
        let w = Uuid::new_v4();
        let a = Archive::new(o, w, 1, vec![(1, Zeroizing::new([7; 32]))]).unwrap();
        let pin = dir.create_generation(id, &a).unwrap();
        assert!(dir.create_generation(id, &a).is_err());
        assert!(dir.open_generation(id, pin).is_ok());
        drop(dir);
        let reopened = PrivateVaultDirectory::open(&path).unwrap();
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
        assert!(PrivateVaultDirectory::open(&path).is_err());
        assert!(path.is_dir());
        std::fs::remove_dir(&path).unwrap();
        assert!(PrivateVaultDirectory::open(Path::new("relative")).is_err());
        assert!(PrivateVaultDirectory::open(Path::new(r"\\server\share\vault")).is_err());
        let prefix = format!("sitedatum-nul-test-{}", Uuid::new_v4());
        let invalid = std::env::temp_dir().join(format!("{prefix}\0suffix"));
        assert!(PrivateVaultDirectory::create(&invalid).is_err());
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
        assert!(PrivateVaultDirectory::open(&alias).is_err());
        assert!(PrivateVaultDirectory::create(&alias.join("vault")).is_err());
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
        let dir = PrivateVaultDirectory::create(&path).unwrap();
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
        assert!(dir.destination(Uuid::new_v4()).is_err());
        assert!(PrivateVaultDirectory::open(&path).is_err());
        drop(handle);
        drop(dir);
        std::fs::remove_dir(&path).unwrap();
        std::fs::remove_dir(&parent).unwrap();
    }
}
