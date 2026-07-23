// أداة تعرض استهلاك الذاكرة لأي تطبيق مفتوح عند التمرير فوق أيقونته باستخدام UI Automation
#![windows_subsystem = "windows"]

use std::mem::size_of;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::ProcessStatus::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::Accessibility::*;
use windows::Win32::UI::Shell::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::System::Com::*;
use windows::Win32::UI::WindowsAndMessaging::GetClassNameW;

const WM_TRAYICON: u32 = WM_APP + 1;
const HOOK_CHECK_TIMER_ID: usize = 1;

static mut TOOLTIP_HWND: HWND = HWND(std::ptr::null_mut());
static mut MAIN_HWND: HWND = HWND(std::ptr::null_mut());
static mut LAST_PID: u32 = 0;
static mut LAST_TEXT: Vec<u16> = Vec::new();

fn main() -> Result<()> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let instance = GetModuleHandleW(None)?;

        
        let main_class = w!("AppMemoryTooltipMainClass");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(main_wnd_proc),
            hInstance: instance.into(),
            lpszClassName: main_class,
            ..Default::default()
        };
        RegisterClassW(&wc);

        MAIN_HWND = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            main_class,
            w!("AppMemoryTooltipHidden"),
            WINDOW_STYLE(0),
            0, 0, 0, 0,
            None, None, instance, None,
        )?;

        // Tooltip
        let tooltip_class = w!("AppMemoryTooltipPopupClass");
        let wc2 = WNDCLASSW {
            lpfnWndProc: Some(tooltip_wnd_proc),
            hInstance: instance.into(),
            lpszClassName: tooltip_class,
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as *mut _),
            ..Default::default()
        };
        RegisterClassW(&wc2);

        TOOLTIP_HWND = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_LAYERED | WS_EX_NOACTIVATE,
            tooltip_class,
            w!("Tooltip"),
            WS_POPUP,
            0, 0, 210, 65,
            None, None, instance, None,
        )?;
        SetLayeredWindowAttributes(TOOLTIP_HWND, COLORREF(0), 240, LWA_ALPHA)?;

        
        add_tray_icon(MAIN_HWND, instance)?;

        //  مؤقّت يفحص الماوس كل 200 ملي ثانية
        SetTimer(MAIN_HWND, HOOK_CHECK_TIMER_ID, 200, None);

        //  Loop message
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        CoUninitialize();
        Ok(())
    }
}

unsafe extern "system" fn main_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TIMER => {
            if wparam.0 == HOOK_CHECK_TIMER_ID {
                check_cursor_with_uia_and_update();
            }
            LRESULT(0)
        }
        WM_TRAYICON => {
            let event = lparam.0 as u32;
            if event == WM_RBUTTONUP {
                show_tray_menu(hwnd);
            }
            LRESULT(0)
        }
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as u32;
            if id == 1001 {
                PostQuitMessage(0);
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            let mut nid = NOTIFYICONDATAW::default();
            nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1;
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe extern "system" fn tooltip_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rect = RECT::default();
            let _ = GetClientRect(hwnd, &mut rect);

            let brush = CreateSolidBrush(COLORREF(0x00262626));
            FillRect(hdc, &rect, brush);
            let _ = DeleteObject(brush);

            let pen = CreatePen(PS_SOLID, 1, COLORREF(0x00404040));
            let old_pen = SelectObject(hdc, pen);
            let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
            let _ = Rectangle(hdc, 0, 0, rect.right, rect.bottom);
            SelectObject(hdc, old_pen);
            SelectObject(hdc, old_brush);
            let _ = DeleteObject(pen);

            SetBkMode(hdc, TRANSPARENT);

            let full_text = String::from_utf16_lossy(
                &LAST_TEXT[..LAST_TEXT.len().saturating_sub(1)],
            );
            let stats: Vec<&str> = full_text.split('\n').collect();

            let stats_font = CreateFontW(
                16, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0,
                DEFAULT_CHARSET.0 as u32, OUT_DEFAULT_PRECIS.0 as u32, CLIP_DEFAULT_PRECIS.0 as u32,
                CLEARTYPE_QUALITY.0 as u32, FF_DONTCARE.0 as u32,
                w!("Segoe UI"),
            );

            let old_font = SelectObject(hdc, stats_font);
            SetTextColor(hdc, COLORREF(0x00E0E0E0));
            let mut y = 10;
            for line in stats {
                let mut wline = to_wide(line);  // must be mutable
                let mut r2 = RECT { left: 12, top: y, right: rect.right - 10, bottom: y + 20 };
                DrawTextW(
                    hdc,
                    &mut wline,
                    &mut r2,
                    DT_LEFT | DT_SINGLELINE | DT_NOPREFIX,
                );
                y += 20;
            }

            SelectObject(hdc, old_font);
            let _ = DeleteObject(stats_font);
            let _ = EndPaint(hwnd, &ps);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn add_tray_icon(hwnd: HWND, instance: HMODULE) -> Result<()> {
    let mut nid = NOTIFYICONDATAW::default();
    nid.cbSize = size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = LoadIconW(None, IDI_APPLICATION)?;
    let tip = to_wide("App Memory Monitor");
    let len = tip.len().min(nid.szTip.len());
    nid.szTip[..len].copy_from_slice(&tip[..len]);

    Shell_NotifyIconW(NIM_ADD, &nid).ok()?;
    Ok(())
}

unsafe fn show_tray_menu(hwnd: HWND) {
    let menu = CreatePopupMenu().unwrap();
    let _ = AppendMenuW(menu, MF_STRING, 1001, w!("Exit"));

    let mut pt = POINT::default();
    let _ = GetCursorPos(&mut pt);
    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON,
        pt.x,
        pt.y,
        0,
        hwnd,
        None,
    );
    let _ = DestroyMenu(menu);
}

unsafe fn check_cursor_with_uia_and_update() {
    let mut pt = POINT::default();
    if GetCursorPos(&mut pt).is_err() {
        return;
    }

    
    let automation = match CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER) {
        Ok(a) => a,
        Err(_) => { hide_tooltip(); return; }
    };

    let element = match automation.ElementFromPoint(pt) {
        Ok(e) => e,
        Err(_) => { hide_tooltip(); return; }
    };

    
    let mut target_pid: u32 = 0;
    if let Ok(hwnd) = element.CurrentNativeWindowHandle() {
        if hwnd.0 != std::ptr::null_mut() {
            let mut pid = 0;
            GetWindowThreadProcessId(hwnd, Some(&mut pid as *mut u32));
            if pid != 0 {
                target_pid = pid;
            }
        }
    }

    if target_pid == 0 {
        if let Ok(pid) = element.CurrentProcessId() {
            target_pid = pid as u32;
        }
    }

    let current_exe_pid = std::process::id();

    if target_pid == 0 || target_pid == current_exe_pid || is_explorer_process(target_pid) {
        hide_tooltip();
        return;
    }

    if let Some((name, working_set_mb, private_mb)) = get_process_memory_info(target_pid) {
        let text = format!(
            "App: {}\nRAM: {:.1} MB | Priv: {:.1} MB",
            name, working_set_mb, private_mb
        );
        LAST_TEXT = to_wide(&text);
        let _ = SetWindowPos(
            TOOLTIP_HWND,
            HWND_TOPMOST,
            pt.x + 12,
            pt.y - 55,
            210,
            65,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        let _ = InvalidateRect(TOOLTIP_HWND, None, true);
    } else {
        hide_tooltip();
    }
}
unsafe fn is_explorer_process(pid: u32) -> bool {
    if let Some((name, _, _)) = get_process_memory_info(pid) {
        return name.to_lowercase() == "explorer.exe";
    }
    false
}

fn hide_tooltip() {
    unsafe {
        let _ = ShowWindow(TOOLTIP_HWND, SW_HIDE);
    }
}

unsafe fn get_process_memory_info(pid: u32) -> Option<(String, f64, f64)> {
    let process = OpenProcess(
        PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
        false,
        pid,
    )
    .ok()?;

    let mut counters = PROCESS_MEMORY_COUNTERS_EX::default();
    let ok = K32GetProcessMemoryInfo(
        process,
        &mut counters as *mut _ as *mut PROCESS_MEMORY_COUNTERS,
        size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32,
    );

    let name = get_process_name(process).unwrap_or_else(|| "Unknown".to_string());
    let _ = CloseHandle(process);

    if ok.as_bool() {
        let working_set_mb = counters.WorkingSetSize as f64 / (1024.0 * 1024.0);
        let private_mb = counters.PrivateUsage as f64 / (1024.0 * 1024.0);
        Some((name, working_set_mb, private_mb))
    } else {
        None
    }
}

unsafe fn get_process_name(process: HANDLE) -> Option<String> {
    let mut buffer = [0u16; 260];
    let mut size = buffer.len() as u32;
    if QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, PWSTR(buffer.as_mut_ptr()), &mut size)
        .is_ok()
    {
        let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
        let file_name = full_path
            .rsplit('\\')
            .next()
            .unwrap_or(&full_path)
            .to_string();
        Some(file_name)
    } else {
        None
    }
}

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}