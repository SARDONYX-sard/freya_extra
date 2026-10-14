//! Run with: `cargo run -p freya_extra --example mod_table --profile release`

mod app;
mod model;
mod state;
mod table;
mod toolbar;
mod widgets;

use freya::prelude::*;
use freya_extra::{acrylic, themes};

fn root() -> impl IntoElement {
    use_init_theme(themes::liquid_glass::theme);
    crate::state::provide_state();

    crate::app::App
}

fn main() {
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new(root)
                .with_transparency(true)
                .with_background(Color::TRANSPARENT)
                .with_window_handle(blur_hook)
                .with_title("Mods Table")
                .with_size(1300.0, 850.0),
        ),
    );
}

fn blur_hook(window: &mut freya::winit::window::Window) {
    #[cfg(target_os = "windows")]
    {
        use freya::winit::{
            platform::windows::WindowExtWindows as _, raw_window_handle::RawWindowHandle,
        };

        let Ok(handle) = (unsafe { window.window_handle_any_thread() }) else {
            return;
        };

        let hwnd = match handle.as_raw() {
            RawWindowHandle::Win32(handle) => match handle.hwnd.get().try_into() {
                Ok(address) => address,
                Err(_) => return,
            },
            RawWindowHandle::WinRt(handle) => handle.core_window.as_ptr() as usize,
            _ => return,
        };

        acrylic::register_window(hwnd);

        if let Err(error) = acrylic::enable() {
            eprintln!("Failed to enable acrylic backdrop: {error}");
        }
    }
}
