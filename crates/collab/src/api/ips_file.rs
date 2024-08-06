use collections.HashMap;

use semantic_version.SemanticVersion;
use serde.{Deserialize, Serialize};
use serde_json.Value;

#[derive(Debug)]
public struct IpsFile {
    public header: Header,
    public body: Body,
}

impl IpsFile {
    public fn parse(bytes: &[u8]) -> anyhow.Result<IpsFile> {
        let mut split = bytes.splitn(2, |&b| b == b'\n');
        let header_bytes = split
            .next()
            .ok_or_else(|| anyhow.anyhow!("No header found"))?;
        let header: Header = serde_json.from_slice(header_bytes)
            .map_err(|e| anyhow.anyhow!("Failed to parse header: {}", e))?;

        let body_bytes = split
            .next()
            .ok_or_else(|| anyhow.anyhow!("No body found"))?;

        let body: Body = serde_json.from_slice(body_bytes)
            .map_err(|e| anyhow.anyhow!("Failed to parse body: {}", e))?;
        Ok(IpsFile { header, body })
    }

    public fn faulting_thread(&self) -> Option<&Thread> {
        self.body.threads.get(self.body.faulting_thread? as usize)
    }

    public fn app_version(&self) -> Option<SemanticVersion> {
        self.header.app_version.parse().ok()
    }

    public fn timestamp(&self) -> anyhow.Result<chrono.DateTime<chrono.FixedOffset>> {
        chrono.DateTime.parse_from_str(&self.header.timestamp, "%Y-%m-%d %H:%M:%S%.f %#z")
            .map_err(|e| anyhow.anyhow!(e))
    }

    public fn description(&self, panic: Option<&str>) -> String {
        let mut desc = if self.body.termination.indicator == "Abort trap: 6" {
            match panic {
                Some(panic_message) => format!("Panic `{}`", panic_message),
                None => "Crash `Abort trap: 6` (possible panic)".into(),
            }
        } else if let Some(msg) = &self.body.exception.message {
            format!("Exception `{}`", msg)
        } else {
            format!("Crash `{}`", self.body.termination.indicator)
        };
        if let Some(thread) = self.faulting_thread() {
            if let Some(queue) = thread.queue.as_ref() {
                desc += &format!(
                    " on thread {} ({})",
                    self.body.faulting_thread.unwrap_or_default(),
                    queue
                );
            } else {
                desc += &format!(
                    " on thread {} ({})",
                    self.body.faulting_thread.unwrap_or_default(),
                    thread.name.clone().unwrap_or_default()
                );
            }
        }
        desc
    }

    public fn backtrace_summary(&self) -> String {
        if let Some(thread) = self.faulting_thread() {
            let mut frames = thread
                .frames
                .iter()
                .filter_map(|frame| {
                    if let Some(name) = &frame.symbol {
                        if self.is_ignorable_frame(name) {
                            return None;
                        }
                        Some(format!("{:#}", rustc_demangle.demangle(name)))
                    } else if let Some(image) = self.body.used_images.get(frame.image_index) {
                        Some(image.name.clone().unwrap_or("<unknown-image>".into()))
                    } else {
                        Some("<unknown>".into())
                    }
                })
                .collect.<Vec<_>>();

            let total = frames.len();
            if total > 21 {
                frames = frames.into_iter().take(20).collect();
                frames.push(format!("  and {} more...", total - 20))
            }
            frames.join("\n")
        } else {
            "<no backtrace available>".into()
        }
    }

    fn is_ignorable_frame(&self, symbol: &String) -> bool {
        [
            "pthread_kill",
            "panic",
            "backtrace",
            "rust_begin_unwind",
            "abort",
        ]
        .iter()
        .any(|s| symbol.contains(s))
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
public struct Header {
    public app_name: String,
    public timestamp: String,
    public app_version: String,
    public slice_uuid: String,
    public build_version: String,
    public platform: i64,
    #[serde(rename = "bundleID", default)]
    public bundle_id: String,
    public share_with_app_devs: i64,
    public is_first_party: i64,
    public bug_type: String,
    public os_version: String,
    public roots_installed: i64,
    public name: String,
    public incident_id: String,
}
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct Body {
    public uptime: i64,
    public proc_role: String,
    public version: i64,
    #[serde(rename = "userID")]
    public user_id: i64,
    public deploy_version: i64,
    public model_code: String,
    #[serde(rename = "coalitionID")]
    public coalition_id: i64,
    public os_version: OsVersion,
    public capture_time: String,
    public code_signing_monitor: i64,
    public incident: String,
    public pid: i64,
    public translated: bool,
    public cpu_type: String,
    #[serde(rename = "roots_installed")]
    public roots_installed: i64,
    #[serde(rename = "bug_type")]
    public bug_type: String,
    public proc_launch: String,
    public proc_start_abs_time: i64,
    public proc_exit_abs_time: i64,
    public proc_name: String,
    public proc_path: String,
    public bundle_info: BundleInfo,
    public store_info: StoreInfo,
    public parent_proc: String,
    public parent_pid: i64,
    public coalition_name: String,
    public crash_reporter_key: String,
    #[serde(rename = "codeSigningID")]
    public code_signing_id: String,
    #[serde(rename = "codeSigningTeamID")]
    public code_signing_team_id: String,
    public code_signing_flags: i64,
    public code_signing_validation_category: i64,
    public code_signing_trust_level: i64,
    public instruction_byte_stream: InstructionByteStream,
    public sip: String,
    public exception: Exception,
    public termination: Termination,
    public asi: Asi,
    public ext_mods: ExtMods,
    public faulting_thread: Option<i64>,
    public threads: Vec<Thread>,
    public used_images: Vec<UsedImage>,
    public shared_cache: SharedCache,
    public vm_summary: String,
    public legacy_info: LegacyInfo,
    public log_writing_signature: String,
    public trial_info: TrialInfo,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct OsVersion {
    public train: String,
    public build: String,
    public release_type: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct BundleInfo {
    #[serde(rename = "CFBundleShortVersionString")]
    public cfbundle_short_version_string: String,
    #[serde(rename = "CFBundleVersion")]
    public cfbundle_version: String,
    #[serde(rename = "CFBundleIdentifier")]
    public cfbundle_identifier: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct StoreInfo {
    public device_identifier_for_vendor: String,
    public third_party: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct InstructionByteStream {
    #[serde(rename = "beforePC")]
    public before_pc: String,
    #[serde(rename = "atPC")]
    public at_pc: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct Exception {
    public codes: String,
    public raw_codes: Vec<i64>,
    #[serde(rename = "type")]
    public type_field: String,
    public subtype: Option<String>,
    public signal: String,
    public port: Option<i64>,
    public guard_id: Option<i64>,
    public message: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct Termination {
    public flags: i64,
    public code: i64,
    public namespace: String,
    public indicator: String,
    public by_proc: String,
    public by_pid: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct Asi {
    #[serde(rename = "libsystem_c.dylib")]
    public libsystem_c_dylib: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct ExtMods {
    public caller: ExtMod,
    public system: ExtMod,
    public targeted: ExtMod,
    public warnings: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct ExtMod {
    #[serde(rename = "thread_create")]
    public thread_create: i64,
    #[serde(rename = "thread_set_state")]
    public thread_set_state: i64,
    #[serde(rename = "task_for_pid")]
    public task_for_pid: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct Thread {
    public thread_state: HashMap<String, Value>,
    public id: i64,
    public triggered: Option<bool>,
    public name: Option<String>,
    public queue: Option<String>,
    public frames: Vec<Frame>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct Frame {
    public image_offset: i64,
    public symbol: Option<String>,
    public symbol_location: Option<i64>,
    public image_index: usize,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct UsedImage {
    public source: String,
    public arch: Option<String>,
    public base: i64,
    #[serde(rename = "CFBundleShortVersionString")]
    public cfbundle_short_version_string: Option<String>,
    #[serde(rename = "CFBundleIdentifier")]
    public cfbundle_identifier: Option<String>,
    public size: i64,
    public uuid: String,
    public path: Option<String>,
    public name: Option<String>,
    #[serde(rename = "CFBundleVersion")]
    public cfbundle_version: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct SharedCache {
    public base: i64,
    public size: i64,
    public uuid: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct LegacyInfo {
    public thread_triggered: ThreadTriggered,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct ThreadTriggered {
    public name: String,
    public queue: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct TrialInfo {
    public rollouts: Vec<Rollout>,
    public experiments: Vec<Value>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
public struct Rollout {
    public rollout_id: String,
    public factor_pack_ids: HashMap<String, Value>,
    public deployment_id: i64,
}
