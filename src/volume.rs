use serde::{Deserialize, Deserializer, Serialize};

use std::os::unix::prelude::OsStrExt;
use std::path::PathBuf;
use std::str::FromStr;

impl Volume {
    pub fn volume_for_path(path: &PathBuf) -> Self {
        let mount_point = unsafe {
            let mut buffer = std::mem::zeroed::<libc::statfs>();
            let r = libc::statfs(
                path.as_os_str().as_bytes().as_ptr() as *const i8,
                &mut buffer,
            );
            if r != 0 {
                panic!("Failed to statfs: {}", r);
            }
            buffer.mount_on_name()
        };
        let mount_point = PathBuf::from_str(&mount_point).unwrap();
        log::debug!("Mount point: {:?}", mount_point);

        let system_profile_json = std::process::Command::new("system_profiler")
            .args(&["-json", "SPStorageDataType"])
            .output()
            .expect("Failed to execute command.")
            .stdout;
        let system_profile: SystemProfile = serde_json::from_slice(&system_profile_json).unwrap();
        let volume = system_profile
            .volumes
            .into_iter()
            .find(|volume| volume.mount_point == mount_point)
            .unwrap();
        return volume;
    }
}

#[derive(Debug, Deserialize)]
struct SystemProfile {
    #[serde(rename = "SPStorageDataType")]
    volumes: Vec<Volume>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Volume {
    #[serde(rename = "_name")]
    name: String,
    bsd_name: String,
    file_system: String,
    mount_point: PathBuf,
    physical_drive: PhysicalDrive,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PhysicalDrive {
    device_name: String,
    #[serde(deserialize_with = "deserialize_yes_or_no")]
    is_internal_disk: bool,
    media_name: String,
    medium_type: Option<String>,
    partition_map_type: String,
    protocol: String,
    smart_status: Option<String>,
}

fn deserialize_yes_or_no<'a, D: Deserializer<'a>>(deserializer: D) -> Result<bool, D::Error> {
    let s = String::deserialize(deserializer)?;
    match s.as_str() {
        "yes" => Ok(true),
        "no" => Ok(false),
        _ => Err(serde::de::Error::custom(format!(
            "Expected Yes or No, got {}",
            s
        ))),
    }
}
trait StatFSStuff {
    fn fstype_name(&self) -> String;
    fn mount_on_name(&self) -> String;
    fn mount_from_name(&self) -> String;
}

impl StatFSStuff for libc::statfs {
    fn fstype_name(&self) -> String {
        unsafe {
            String::from_utf8_lossy(std::ffi::CStr::from_ptr(self.f_fstypename.as_ptr()).to_bytes())
                .to_string()
        }
    }

    fn mount_on_name(&self) -> String {
        unsafe {
            String::from_utf8_lossy(std::ffi::CStr::from_ptr(self.f_mntonname.as_ptr()).to_bytes())
                .to_string()
        }
    }
    fn mount_from_name(&self) -> String {
        unsafe {
            String::from_utf8_lossy(
                std::ffi::CStr::from_ptr(self.f_mntfromname.as_ptr()).to_bytes(),
            )
            .to_string()
        }
    }
}
