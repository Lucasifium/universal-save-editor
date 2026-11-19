//! Is this save in a folder a cloud service syncs? A sync can bring an older copy back over an
//! edited save, so the editor says so (quietly) before and after saving.

use std::path::Path;

/// The service that syncs the folder `save` is in, if one seems to: Steam Cloud
/// (`userdata/<id>/<app>/remote`), OneDrive, Dropbox, Google Drive, iCloud Drive or Ubisoft
/// Connect's `savegames`. Judged from the path alone, so a game's own cloud option elsewhere
/// (Steam Auto-Cloud on Documents, say) isn't seen.
pub fn service_for(save: &Path) -> Option<&'static str> {
    let parts: Vec<String> = save.components().map(|c| c.as_os_str().to_string_lossy().to_lowercase()).collect();
    let has = |n: &str| parts.iter().any(|c| c == n);
    if has("userdata") && has("remote") {
        Some("Steam Cloud")
    } else if parts.iter().any(|c| c.starts_with("onedrive")) {
        Some("OneDrive")
    } else if has("dropbox") {
        Some("Dropbox")
    } else if has("google drive") || has("googledrive") || has("my drive") {
        Some("Google Drive")
    } else if has("iclouddrive") || has("icloud drive") || has("mobile documents") {
        Some("iCloud Drive")
    } else if has("ubisoft game launcher") && has("savegames") {
        Some("Ubisoft Connect")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn finds_the_service_that_syncs_a_folder() {
        let cases = [
            ("C:/Program Files/Steam/userdata/123/4000/remote/slot1.sav", Some("Steam Cloud")),
            ("C:/Users/p/OneDrive/Documents/My Games/G/a.sav", Some("OneDrive")),
            ("C:/Users/p/OneDrive - Work/Documents/a.sav", Some("OneDrive")),
            ("/home/p/Dropbox/saves/a.sav", Some("Dropbox")),
            ("G:/My Drive/saves/a.sav", Some("Google Drive")),
            ("/Users/p/Library/Mobile Documents/com~apple~CloudDocs/a.sav", Some("iCloud Drive")),
            ("C:/Program Files/Ubisoft/Ubisoft Game Launcher/savegames/id/1/a.sav", Some("Ubisoft Connect")),
            ("C:/Users/p/Documents/My Games/G/a.sav", None),
            ("C:/Program Files/Steam/userdata/123/4000/other/a.sav", None),
        ];
        for (p, want) in cases {
            assert_eq!(service_for(&PathBuf::from(p)), want, "{p}");
        }
    }
}
