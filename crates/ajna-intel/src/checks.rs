// Known device-compromise artifact catalogs.
//
// Platform shells report raw facts (paths that exist, libraries loaded,
// build properties); this module holds the reference lists those facts are
// matched against. Lists are additive product data — extending them is a
// data change, not a logic change.

/// iOS jailbreak filesystem artifacts (classic + rootless).
pub const IOS_JAILBREAK_ARTIFACTS: &[&str] = &[
    "/Applications/Cydia.app",
    "/Applications/Sileo.app",
    "/Library/MobileSubstrate/MobileSubstrate.dylib",
    "/bin/bash",
    "/usr/sbin/sshd",
    "/etc/apt",
    "/private/var/lib/apt",
    "/usr/bin/ssh",
    "/var/jb",
];

/// Android root artifacts (su binaries, managers, Magisk).
pub const ANDROID_ROOT_ARTIFACTS: &[&str] = &[
    "/system/bin/su",
    "/system/xbin/su",
    "/sbin/su",
    "/su/bin/su",
    "/system/app/Superuser.apk",
    "/system/app/SuperSU.apk",
    "/data/adb/magisk",
    "/system/bin/.ext/.su",
];

/// Case-insensitive substrings identifying instrumentation / hooking
/// frameworks in loaded library names.
pub const HOOKING_LIBRARY_MARKERS: &[&str] = &[
    "frida",
    "substrate",
    "cycript",
    "xposed",
    "libriru",
    "zygisk",
    "shadowhook",
];

/// Android build properties whose values indicate an emulator.
/// Matched as (property key, case-insensitive value substring).
pub const EMULATOR_PROPERTIES: &[(&str, &str)] = &[
    ("ro.kernel.qemu", "1"),
    ("ro.hardware", "goldfish"),
    ("ro.hardware", "ranchu"),
    ("ro.product.model", "sdk"),
    ("ro.product.model", "emulator"),
    ("ro.product.manufacturer", "genymotion"),
];

/// Property marking a debuggable OS build.
pub const DEBUGGABLE_PROPERTY: (&str, &str) = ("ro.debuggable", "1");

pub fn is_known_root_artifact(path: &str) -> bool {
    ANDROID_ROOT_ARTIFACTS.contains(&path)
}

pub fn is_known_jailbreak_artifact(path: &str) -> bool {
    IOS_JAILBREAK_ARTIFACTS.contains(&path)
}

pub fn hooking_marker_in(library_name: &str) -> Option<&'static str> {
    let lowered = library_name.to_ascii_lowercase();
    HOOKING_LIBRARY_MARKERS
        .iter()
        .copied()
        .find(|marker| lowered.contains(marker))
}

pub fn is_emulator_property(key: &str, value: &str) -> bool {
    let lowered = value.to_ascii_lowercase();
    EMULATOR_PROPERTIES
        .iter()
        .any(|(prop_key, marker)| *prop_key == key && lowered.contains(marker))
}
