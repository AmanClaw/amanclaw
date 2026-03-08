use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App, Manager,
};

pub fn setup_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let quit = MenuItem::with_id(app, "quit", "Quit AmanClaw", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open Dashboard", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("AmanClaw - Bot Manager")
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "quit" => app.exit(0),
                "open" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}
