use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static HWND: AtomicUsize = AtomicUsize::new(0);
static ENABLED: AtomicBool = AtomicBool::new(false);

const GRADIENT_COLOR: u32 = 0x30606060;

/// Registers the native window handle.
///
/// Call this from `WindowConfig::with_window_handle`.
pub fn register_window(hwnd: usize) {
    HWND.store(hwnd, Ordering::Release);
}

/// Returns whether Acrylic is currently enabled.
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::Acquire)
}

/// Enables Acrylic for the registered window.
pub fn enable() -> Result<(), &'static str> {
    set_enabled(true)
}

/// Disables Acrylic for the registered window.
pub fn disable() -> Result<(), &'static str> {
    set_enabled(false)
}

/// Toggles Acrylic and returns the resulting state.
pub fn toggle() -> Result<bool, &'static str> {
    let enabled = !is_enabled();
    set_enabled(enabled)?;
    Ok(enabled)
}

fn set_enabled(enabled: bool) -> Result<(), &'static str> {
    let hwnd = HWND.load(Ordering::Acquire);

    if hwnd == 0 {
        return Err("Window handle has not been registered");
    }

    #[cfg(target_os = "windows")]
    {
        let hwnd = core::ptr::with_exposed_provenance_mut(hwnd);
        unsafe {
            if enabled {
                windows_acrylic::enable(hwnd, GRADIENT_COLOR)?;
            } else {
                windows_acrylic::disable(hwnd)?;
            }
        }
        ENABLED.store(enabled, Ordering::Release);
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
        Err("Acrylic is only supported on Windows")
    }
}

#[cfg(target_os = "windows")]
mod windows_acrylic {
    use core::{
        ffi::{c_char, c_int, c_uint, c_void},
        mem,
    };

    #[link(name = "kernel32.dll", kind = "raw-dylib", modifiers = "+verbatim")]
    unsafe extern "system" {
        // https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-loadlibrarya
        fn LoadLibraryA(name: *const c_char) -> *mut c_void;

        // https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-getprocaddress
        fn GetProcAddress(library: *mut c_void, name: *const c_char) -> *mut c_void;

        // https://learn.microsoft.com/en-us/windows/win32/api/libloaderapi/nf-libloaderapi-freelibrary
        fn FreeLibrary(library: *mut c_void) -> c_int;
    }

    #[repr(C)]
    struct AccentPolicy {
        accent_state: c_uint,
        accent_flags: c_uint,
        gradient_color: c_uint,
        animation_id: c_uint,
    }

    /// https://learn.microsoft.com/en-us/windows/win32/dwm/windowcompositionattribdata
    #[repr(C)]
    struct WindowCompositionAttributeData {
        attribute: c_int,
        data: *mut c_void,
        size_of_data: usize,
    }

    /// https://learn.microsoft.com/en-us/windows/win32/dwm/getwindowcompositionattribute
    type SetWindowCompositionAttributeFn = unsafe extern "system" fn(
        hwnd: *mut c_void,
        data: *mut WindowCompositionAttributeData,
    ) -> c_int;

    const WCA_ACCENT_POLICY: c_int = 19;
    const ACCENT_DISABLED: c_uint = 0;
    const ACCENT_ENABLE_ACRYLICBLURBEHIND: c_uint = 4;

    #[inline]
    pub unsafe fn enable(hwnd: *mut c_void, gradient_color: c_uint) -> Result<(), &'static str> {
        unsafe { apply(hwnd, ACCENT_ENABLE_ACRYLICBLURBEHIND, 2, gradient_color) }
    }

    #[inline]
    pub unsafe fn disable(hwnd: *mut c_void) -> Result<(), &'static str> {
        unsafe { apply(hwnd, ACCENT_DISABLED, 0, 0) }
    }

    unsafe fn apply(
        hwnd: *mut c_void,
        accent_state: c_uint,
        accent_flags: c_uint,
        gradient_color: c_uint,
    ) -> Result<(), &'static str> {
        let module = unsafe { LoadLibraryA(c"user32.dll".as_ptr()) };

        if module.is_null() {
            return Err("Failed to load user32.dll");
        }

        let symbol = unsafe { GetProcAddress(module, c"SetWindowCompositionAttribute".as_ptr()) };

        if symbol.is_null() {
            unsafe { FreeLibrary(module) };
            return Err("SetWindowCompositionAttribute is unavailable");
        }

        let set_window_composition_attribute: SetWindowCompositionAttributeFn =
            unsafe { mem::transmute(symbol) };

        let mut policy =
            AccentPolicy { accent_state, accent_flags, gradient_color, animation_id: 0 };

        let mut data = WindowCompositionAttributeData {
            attribute: WCA_ACCENT_POLICY,
            data: (&mut policy as *mut AccentPolicy).cast(),
            size_of_data: mem::size_of::<AccentPolicy>(),
        };

        let succeeded = unsafe { set_window_composition_attribute(hwnd, &mut data) };

        unsafe { FreeLibrary(module) };

        if succeeded == 0 {
            return Err("SetWindowCompositionAttribute failed");
        }

        Ok(())
    }
}
