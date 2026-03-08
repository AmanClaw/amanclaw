use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

pub fn notify_solat(app: &AppHandle, prayer: &str, time: &str, zone: &str) {
    let _ = app.notification()
        .builder()
        .title(&format!("Waktu {} - {}", prayer, zone))
        .body(&format!("Waktu {} telah masuk: {}", prayer, time))
        .show();
}

pub fn notify_user_pending(app: &AppHandle, user_id: &str, platform: &str) {
    let _ = app.notification()
        .builder()
        .title("New User Pending")
        .body(&format!("{} ({}) is waiting for approval", user_id, platform))
        .show();
}

pub fn notify_skill_error(app: &AppHandle, skill: &str, error: &str) {
    let _ = app.notification()
        .builder()
        .title(&format!("Skill Error: {}", skill))
        .body(error)
        .show();
}

pub fn notify_community_joined(app: &AppHandle, name: &str) {
    let _ = app.notification()
        .builder()
        .title("Community Joined")
        .body(&format!("{} has been onboarded", name))
        .show();
}
