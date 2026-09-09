from pathlib import Path

cargo = Path("Cargo.toml")
c = cargo.read_text()
marker = 'serde_json = "1.0"\n'
target_dep = "\n[target.'cfg(windows)'.dependencies]\nwinapi = { version = \"0.3.9\", features = [\"winuser\"] }\n"
if "[target.'cfg(windows)'.dependencies]" not in c:
    if marker not in c:
        raise SystemExit("Cargo marker not found")
    c = c.replace(marker, marker + target_dep, 1)
cargo.write_text(c)

path = Path("src/bin/client.rs")
text = path.read_text()

import_marker = 'use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};\n'
imports = '''use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[cfg(target_os = "windows")]
use std::ptr;
#[cfg(target_os = "windows")]
use winapi::shared::windef::HWND;
#[cfg(target_os = "windows")]
use winapi::um::winuser::{
    FindWindowW, GetMonitorInfoW, MonitorFromWindow, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE,
    GWL_STYLE, HWND_TOP, MONITORINFO, MONITOR_DEFAULTTONEAREST, SWP_FRAMECHANGED, SWP_SHOWWINDOW,
    WS_EX_APPWINDOW, WS_POPUP, WS_VISIBLE,
};
'''
if imports not in text:
    if import_marker not in text:
        raise SystemExit("import marker not found")
    text = text.replace(import_marker, imports, 1)

old_conf = '''        high_dpi: true,
        fullscreen: false,
        window_resizable: true,
        ..Default::default()
'''
new_conf = '''        high_dpi: true,
        fullscreen: false,
        window_resizable: false,
        ..Default::default()
'''
if old_conf not in text:
    raise SystemExit("window config pattern not found")
text = text.replace(old_conf, new_conf, 1)

window_block = '''fn window_conf() -> Conf {
    Conf {
        window_title: "Maze Wars 3D - Multiplayer FPS".to_string(),
        window_width: 1180,
        window_height: 820,
        high_dpi: true,
        fullscreen: false,
        window_resizable: false,
        ..Default::default()
    }
}
'''
helper = window_block + '''
#[cfg(target_os = "windows")]
fn force_fullscreen() {
    unsafe {
        let title: Vec<u16> = "Maze Wars 3D - Multiplayer FPS"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let hwnd: HWND = FindWindowW(ptr::null(), title.as_ptr());
        if hwnd.is_null() {
            return;
        }

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return;
        }

        SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_POPUP | WS_VISIBLE) as isize);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, WS_EX_APPWINDOW as isize);
        let rect = info.rcMonitor;
        SetWindowPos(
            hwnd,
            HWND_TOP,
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
            SWP_FRAMECHANGED | SWP_SHOWWINDOW,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn force_fullscreen() {
    set_fullscreen(true);
}
'''
if "fn force_fullscreen()" not in text:
    if window_block not in text:
        raise SystemExit("window_conf function not found")
    text = text.replace(window_block, helper, 1)

old_main = '''    set_cursor_grab(false);
    show_mouse(true);

    // Create a normal resizable Windows window first. With miniquad 0.4.6,
    // startup fullscreen can leave a decorated fixed-size window. Apply
    // fullscreen only after the native window has completed one frame.
    clear_background(Color::new(0.008, 0.011, 0.02, 1.0));
    screen = match screen {
        AppScreen::Connect(connect) => update_connect(connect),
        AppScreen::Game(game) => update_game(game),
        AppScreen::Editor(editor) => update_editor(editor),
    };
    next_frame().await;
    set_fullscreen(true);
    let mut fullscreen = true;

    loop {
        if is_key_pressed(KeyCode::F11) {
            fullscreen = !fullscreen;
            set_fullscreen(fullscreen);
        }
        clear_background(Color::new(0.008, 0.011, 0.02, 1.0));
'''
new_main = '''    set_cursor_grab(false);
    show_mouse(true);
    force_fullscreen();

    loop {
        clear_background(Color::new(0.008, 0.011, 0.02, 1.0));
'''
if old_main not in text:
    raise SystemExit("main fullscreen block not found")
text = text.replace(old_main, new_main, 1)

old_cursor = '''    let cursor = if active && ((get_time() * 2.0) as i32 % 2 == 0) {
        "_"
    } else {
        ""
    };
    draw_text(
        format!("{value}{cursor}"),
'''
new_cursor = '''    draw_text(
        value,
'''
if old_cursor not in text:
    raise SystemExit("field cursor block not found")
text = text.replace(old_cursor, new_cursor, 1)

path.write_text(text)
