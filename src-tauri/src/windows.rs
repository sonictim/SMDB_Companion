use std::fs;
use std::path::{Path, PathBuf};

/// Detects the correct Windows drive letter for a given volume name
/// by checking all available drives (A: through Z:)
pub fn detect_drive_letter(volume_name: &str) -> Option<char> {
    // Check drives A through Z
    for drive_letter in 'A'..='Z' {
        let drive_path = format!("{}:\\", drive_letter);

        // Check if the drive exists and is accessible
        if let Ok(metadata) = fs::metadata(&drive_path) {
            if metadata.is_dir() {
                // Try to get the volume label for this drive
                if let Some(label) = get_volume_label(drive_letter) {
                    // Case-insensitive comparison of volume names
                    if label.to_lowercase() == volume_name.to_lowercase() {
                        return Some(drive_letter);
                    }
                }

                // Fallback: check if a folder with the volume name exists at the root
                let volume_folder_path = format!("{}:\\{}", drive_letter, volume_name);
                if Path::new(&volume_folder_path).exists() {
                    return Some(drive_letter);
                }
            }
        }
    }

    None
}

/// Gets the volume label for a given drive letter on Windows
pub fn get_volume_label(drive_letter: char) -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use winapi::um::fileapi::GetVolumeInformationW;
        use winapi::um::winnt::WCHAR;

        let drive_path = format!("{}:\\", drive_letter);
        let mut drive_path_wide: Vec<u16> = drive_path
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let mut volume_name_buffer: [WCHAR; 256] = [0; 256];
        let mut file_system_buffer: [WCHAR; 256] = [0; 256];
        let mut volume_serial_number: u32 = 0;
        let mut maximum_component_length: u32 = 0;
        let mut file_system_flags: u32 = 0;

        let result = unsafe {
            GetVolumeInformationW(
                drive_path_wide.as_ptr(),
                volume_name_buffer.as_mut_ptr(),
                volume_name_buffer.len() as u32,
                &mut volume_serial_number,
                &mut maximum_component_length,
                &mut file_system_flags,
                file_system_buffer.as_mut_ptr(),
                file_system_buffer.len() as u32,
            )
        };

        if result != 0 {
            // Find the null terminator
            let end = volume_name_buffer
                .iter()
                .position(|&x| x == 0)
                .unwrap_or(volume_name_buffer.len());
            let volume_name = OsString::from_wide(&volume_name_buffer[..end]);
            volume_name.into_string().ok()
        } else {
            None
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        println!("Getting volume label for drive: {}", drive_letter);
        // For non-Windows platforms, return None
        None
    }
}

/// Converts a macOS-style path from the database to a proper Windows path
///
/// # Arguments
/// * `macos_path` - The macOS-style path from the database (e.g., "/Volumes/AVS SFX/AVS SFX/Mindful Audio")
/// * `drive_letter` - The Windows drive letter to use (e.g., 'F')
///
/// # Returns
/// * `Option<PathBuf>` - The converted Windows path, or None if conversion fails
pub fn convert_macos_path_to_windows(macos_path: &str, drive_letter: char) -> Option<PathBuf> {
    // Normalize the path separators and remove leading slash
    let normalized_path = macos_path
        .replace('/', "\\")
        .trim_start_matches('\\')
        .to_string();

    // Check if this is a /Volumes/ path
    if normalized_path.to_lowercase().starts_with("volumes\\") {
        // Remove the "Volumes\" prefix
        let without_volumes = normalized_path
            .strip_prefix("volumes\\")
            .or_else(|| normalized_path.strip_prefix("Volumes\\"))?;

        // Find the first backslash to separate volume name from the rest of the path
        if let Some(first_slash_pos) = without_volumes.find('\\') {
            let _volume_name = &without_volumes[..first_slash_pos];
            let remaining_path = &without_volumes[first_slash_pos + 1..];

            // Construct the Windows path
            let windows_path = format!("{}:\\{}", drive_letter, remaining_path);
            Some(PathBuf::from(windows_path))
        } else {
            // If there's no path after the volume name, just return the drive root
            Some(PathBuf::from(format!("{}:\\", drive_letter)))
        }
    } else {
        // If it's not a /Volumes/ path, treat it as a relative path from the drive root
        let windows_path = format!("{}:\\{}", drive_letter, normalized_path);
        Some(PathBuf::from(windows_path))
    }
}

/// Attempts to automatically convert a macOS-style path to Windows by detecting the drive
///
/// # Arguments
/// * `macos_path` - The macOS-style path from the database
///
/// # Returns
/// * `Option<PathBuf>` - The converted Windows path, or None if conversion fails
pub fn auto_convert_macos_path_to_windows(macos_path: &str) -> Option<PathBuf> {
    // Extract volume name from the macOS path
    let volume_name = extract_volume_name(macos_path)?;

    // Try to detect the drive letter for this volume
    let drive_letter = detect_drive_letter(&volume_name)?;

    // Convert the path using the detected drive letter
    convert_macos_path_to_windows(macos_path, drive_letter)
}

/// Extracts the volume name from a macOS-style path
///
/// # Arguments
/// * `macos_path` - The macOS-style path (e.g., "/Volumes/AVS SFX/AVS SFX/Mindful Audio")
///
/// # Returns
/// * `Option<String>` - The volume name, or None if extraction fails
fn extract_volume_name(macos_path: &str) -> Option<String> {
    let normalized_path = macos_path.trim_start_matches('/');

    if normalized_path.to_lowercase().starts_with("volumes/") {
        let without_volumes = normalized_path
            .strip_prefix("volumes/")
            .or_else(|| normalized_path.strip_prefix("Volumes/"))?;

        // Find the first slash to get just the volume name
        if let Some(first_slash_pos) = without_volumes.find('/') {
            Some(without_volumes[..first_slash_pos].to_string())
        } else {
            // If there's no slash, the entire remaining part is the volume name
            Some(without_volumes.to_string())
        }
    } else {
        None
    }
}

/// Validates that a converted Windows path actually exists
///
/// # Arguments
/// * `windows_path` - The Windows path to validate
///
/// # Returns
/// * `bool` - True if the path exists, false otherwise
pub fn validate_windows_path(windows_path: &Path) -> bool {
    windows_path.exists()
}

/// Comprehensive function to convert and validate a macOS path to Windows
///
/// # Arguments
/// * `macos_path` - The macOS-style path from the database
///
/// # Returns
/// * `Option<PathBuf>` - A validated Windows path, or None if conversion/validation fails
pub fn convert_and_validate_path(macos_path: &str) -> Option<PathBuf> {
    if let Some(windows_path) = auto_convert_macos_path_to_windows(macos_path) {
        if validate_windows_path(&windows_path) {
            Some(windows_path)
        } else {
            // If the auto-detected path doesn't exist, try all drives manually
            if let Some(_volume_name) = extract_volume_name(macos_path) {
                for drive_letter in 'A'..='Z' {
                    if let Some(test_path) = convert_macos_path_to_windows(macos_path, drive_letter)
                    {
                        if validate_windows_path(&test_path) {
                            return Some(test_path);
                        }
                    }
                }
            }
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_volume_name() {
        assert_eq!(
            extract_volume_name("/Volumes/AVS SFX/AVS SFX/Mindful Audio"),
            Some("AVS SFX".to_string())
        );

        assert_eq!(
            extract_volume_name("/Volumes/MyDrive/folder/file.txt"),
            Some("MyDrive".to_string())
        );

        assert_eq!(
            extract_volume_name("/Volumes/USB Drive"),
            Some("USB Drive".to_string())
        );

        assert_eq!(extract_volume_name("/Users/test/file.txt"), None);
    }

    #[test]
    fn test_convert_macos_path_to_windows() {
        let result = convert_macos_path_to_windows("/Volumes/AVS SFX/AVS SFX/Mindful Audio", 'F');
        assert_eq!(result, Some(PathBuf::from("F:\\AVS SFX\\Mindful Audio")));

        let result = convert_macos_path_to_windows("/Volumes/MyDrive/folder/file.txt", 'E');
        assert_eq!(result, Some(PathBuf::from("E:\\folder\\file.txt")));
    }
}
