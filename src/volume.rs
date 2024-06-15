use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(target_os = "macos")]
use std::path::Path;

#[cfg(target_os = "macos")]
use std::str::FromStr;

#[cfg(target_os = "macos")]
use std::os::unix::prelude::OsStrExt;

#[cfg(target_os = "macos")]
#[derive(Serialize, Deserialize, Debug)]
pub struct Volume {
    #[serde(rename = "_name")]
    name: String,
    bsd_name: String,
    file_system: String,
    mount_point: PathBuf,
    physical_drive: PhysicalDrive,
}

#[cfg(target_os = "macos")]
#[derive(Serialize, Deserialize, Debug)]
pub struct PhysicalDrive {
    device_name: String,
    // #[serde(deserialize_with = "deserialize_yes_or_no")]
    // is_internal_disk: bool,
    media_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    medium_type: Option<String>,
    partition_map_type: String,
    protocol: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    smart_status: Option<String>,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Deserialize)]
struct SystemProfile {
    #[serde(rename = "SPStorageDataType")]
    volumes: Vec<Volume>,
}

#[cfg(target_os = "macos")]
impl Volume {
    pub fn volume_for_path(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Err(anyhow::anyhow!("Path {} does not exist", path.display()));
        }
        let mount_point: anyhow::Result<String> = unsafe {
            let mut buffer = std::mem::zeroed::<libc::statfs>();
            let r = libc::statfs(
                path.as_os_str().as_bytes().as_ptr() as *const i8,
                &mut buffer,
            );
            if r != 0 {
                panic!("Failed to statfs: {}", r);
            }
            Ok(buffer.mount_on_name())
        };
        let mount_point = PathBuf::from_str(&mount_point?)?;
        let system_profile_json = std::process::Command::new("system_profiler")
            .args(["-json", "SPStorageDataType"])
            .output()?
            .stdout;
        let system_profile: SystemProfile = serde_json::from_slice(&system_profile_json)?;
        let volume = system_profile
            .volumes
            .into_iter()
            .find(|volume| volume.mount_point == mount_point)
            .ok_or(anyhow::anyhow!(
                "Failed to find volume for path {}",
                path.display()
            ))?;
        Ok(volume)
    }
}

#[cfg(target_os = "macos")]
trait StatFSStuff {
    fn mount_on_name(&self) -> String;
}

#[cfg(target_os = "macos")]
impl StatFSStuff for libc::statfs {
    fn mount_on_name(&self) -> String {
        unsafe {
            String::from_utf8_lossy(std::ffi::CStr::from_ptr(self.f_mntonname.as_ptr()).to_bytes())
                .to_string()
        }
    }
}

// MARK: Linux

#[cfg(target_os = "linux")]
#[derive(Serialize, Deserialize, Debug)]
pub struct Volume {}

#[cfg(target_os = "linux")]
impl Volume {
    pub fn volume_for_path(_: &PathBuf) -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("Not implemented."))
    }
}

// MARK: Windows

#[cfg(target_os = "windows")]
#[derive(Serialize, Deserialize, Debug)]
pub struct Volume {}

#[cfg(target_os = "windows")]
impl Volume {
    pub fn volume_for_path(_: &PathBuf) -> anyhow::Result<Self> {
        return Err(anyhow::anyhow!("Not implemented."));
    }
}
