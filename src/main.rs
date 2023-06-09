mod color_filtering;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use std::{env, mem, ptr};
use winapi::shared::minwindef::LPVOID;
use winapi::shared::windef::{HGDIOBJ, POINT};
use winapi::um::wingdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, GetDIBits, SelectObject, BITMAPINFOHEADER,
    BI_RGB, DIB_RGB_COLORS, SRCCOPY,
};
use winapi::um::winuser::{
    GetCursorPos, GetDC, GetKeyState, GetSystemMetrics, SendInput, INPUT, INPUT_MOUSE,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, SM_CXSCREEN, SM_CYSCREEN, VK_SHIFT,
};

use crate::color_filtering::{find_head_point, threshold_white};

const YELLOW_TARGET: (f32, f32, f32) = (60.0, 1.0, 1.0);
const PURPLE_TARGET: (f32, f32, f32) = (300.0, 1.0, 1.0);
const RED_TARGET: (f32, f32, f32) = (360.0, 1.0, 1.0);

fn main() {
    let args: Vec<String> = env::args().collect();
    let binding = String::from("Yellow");
    let player_color = args.get(1).unwrap_or(&binding);
    let scan_size = args.get(2).and_then(|arg| arg.parse().ok()).unwrap_or(100);
    let tb_delay = args.get(3).and_then(|arg| arg.parse().ok()).unwrap_or(350);
    let refresh_rate = args.get(4).and_then(|arg| arg.parse().ok()).unwrap_or(60);

    let target = match player_color.as_str() {
        "Purple" => PURPLE_TARGET,
        "Red" => RED_TARGET,
        _ => YELLOW_TARGET,
    };

    let _ = thread::spawn(move || {
        let screen_width = unsafe { GetSystemMetrics(SM_CXSCREEN) };
        let screen_height = unsafe { GetSystemMetrics(SM_CYSCREEN) };

        let h_screen = unsafe { GetDC(ptr::null_mut()) };
        let h_bitmap = unsafe { CreateCompatibleBitmap(h_screen, scan_size, scan_size) };
        let mut screen_data = vec![0u8; (scan_size * scan_size * 4) as usize];
        let h_dc = unsafe { CreateCompatibleDC(h_screen) };

        let middle_screen = POINT {
            x: screen_width / 2,
            y: screen_height / 2,
        };

        let mut bmi: BITMAPINFOHEADER = unsafe { mem::zeroed() };
        bmi.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.biPlanes = 1;
        bmi.biBitCount = 32;
        bmi.biWidth = scan_size;
        bmi.biHeight = -scan_size;
        bmi.biCompression = BI_RGB;
        bmi.biSizeImage = 0;

        let mut previous_data: Option<Vec<u8>> = None;
        let mut last_frame_time = Instant::now();
        let mut tb_cd = Instant::now();

        loop {
            while !is_shift_key_pressed() || !is_movement_keys_pressed() || tb_cd.elapsed() < Duration::from_millis(tb_delay) {
                thread::yield_now();
            }

            // Wait for next VBI
            let vbi_duration = get_vertical_blank_duration(refresh_rate);
            let elapsed = last_frame_time.elapsed();
            if elapsed < vbi_duration {
                thread::sleep(vbi_duration - elapsed);
            }

            let old_obj = unsafe { SelectObject(h_dc, h_bitmap as HGDIOBJ) };
            unsafe {
                BitBlt(
                    h_dc,
                    0,
                    0,
                    scan_size,
                    scan_size,
                    h_screen,
                    middle_screen.x - (scan_size / 2),
                    middle_screen.y - (scan_size / 2),
                    SRCCOPY,
                )
            };

            unsafe { SelectObject(h_dc, old_obj) };
            unsafe {
                GetDIBits(
                    h_dc,
                    h_bitmap,
                    0,
                    scan_size.try_into().unwrap(),
                    screen_data.as_mut_ptr() as LPVOID,
                    &mut bmi as *mut _ as *mut _,
                    DIB_RGB_COLORS,
                )
            };

            let data = threshold_white(&mut screen_data, target);

            if let Some(ref previous_frame) = previous_data {
                if data == *previous_frame {
                    continue;
                }
            }

            previous_data = Some(data.clone());
            last_frame_time = Instant::now();

            if let Some(head_point) = find_head_point(&data) {
                if head_point.0 > 47 && head_point.0 < 53 && head_point.1 < 50 && head_point.1 > 35 {
                    send_left_click();
                    tb_cd = Instant::now();
                }
            }
        }
    });

    println!(
        "Player Color: {}\nSearch Size: {}\nTriggerbot Delay: {}\nRefreshrate: {}",
        player_color, scan_size, tb_delay, refresh_rate
    );

    let running = Arc::new(AtomicBool::new(true));

    let running_signal = Arc::clone(&running);
    ctrlc::set_handler(move || {
        running_signal.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_secs(1));
    }
}

fn get_vertical_blank_duration(refresh_rate: u32) -> Duration {
    let frame_duration = Duration::from_secs(1) / refresh_rate;
    let vbi_duration = frame_duration / 2;

    vbi_duration - Duration::from_micros(100)
}

fn is_movement_keys_pressed() -> bool {
    (unsafe { GetKeyState(b'A' as i32) } as i32 & 0x8000) == 0
        && (unsafe { GetKeyState(b'W' as i32) } as i32 & 0x8000) == 0
        && (unsafe { GetKeyState(b'D' as i32) } as i32 & 0x8000) == 0
        && (unsafe { GetKeyState(b'S' as i32) } as i32 & 0x8000) == 0
}

fn is_shift_key_pressed() -> bool {
    (unsafe { GetKeyState(VK_SHIFT) } as i32 & 0x8000) != 0
}

fn send_left_click() {
    unsafe {
        let mut cursor_pos: POINT = POINT { x: 0, y: 0 };
        GetCursorPos(&mut cursor_pos);

        let mut input: INPUT = std::mem::zeroed();
        input.type_ = INPUT_MOUSE;
        let mut flags = MOUSEEVENTF_LEFTDOWN;
        input.u.mi_mut().dwFlags = flags;
        SendInput(
            1,
            &mut input as *mut INPUT,
            std::mem::size_of::<INPUT>() as i32,
        );

        flags = MOUSEEVENTF_LEFTUP;
        input.u.mi_mut().dwFlags = flags;
        SendInput(
            1,
            &mut input as *mut INPUT,
            std::mem::size_of::<INPUT>() as i32,
        );
    }
}
