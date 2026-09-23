// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code, unused_macros)]
#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering::SeqCst},
        Mutex,
    },
    time::{Duration, Instant},
};

use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager,
};

use nvml_wrapper::{enum_wrappers::device::TemperatureSensor::Gpu, Nvml};
use sysinfo::{Networks, System};
use windows_capture::{
    capture::{Context, GraphicsCaptureApiHandler},
    frame::Frame,
    graphics_capture_api::InternalCaptureControl,
    settings::*,
    window::Window,
};

#[derive(Clone, serde::Serialize)]
struct NetworkSpeed {
    net_download_speed: f64,
    net_upload_speed: f64,
}

struct FPSCounter {
    app: AppHandle,
    frame_count: u32,
    last_check: Instant,
    current_hwnd: isize,
}

impl GraphicsCaptureApiHandler for FPSCounter {
    type Flags = AppHandle;
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(context: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let active_win =
            Window::foreground().map_err(|err| format!("Errore: nessuna finestra! {}", err))?;

        Ok(Self {
            app: context.flags,
            frame_count: 0,
            last_check: Instant::now(),
            current_hwnd: active_win.as_raw_hwnd() as isize,
        })
    }

    fn on_frame_arrived(
        &mut self,
        _frame: &mut Frame,
        capture_control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        if let Ok(active_win) = Window::foreground() {
            if active_win.as_raw_hwnd() as isize != self.current_hwnd {
                capture_control.stop();
                return Ok(());
            }
        }

        self.frame_count += 1;

        if self.last_check.elapsed() >= Duration::from_secs(1) {
            let fps = self.frame_count as f64 / self.last_check.elapsed().as_secs_f64();
            let _ = self.app.emit("on_fps_measured", fps);
            self.frame_count = 0;
            self.last_check = Instant::now();
        }

        Ok(())
    }
}

const IS_DEV: bool = cfg!(debug_assertions);

static IS_MEASURING_NETWORK_SPEED: AtomicBool = AtomicBool::new(false);
static IS_MEASURING_FPS: AtomicBool = AtomicBool::new(false);
static SYS: Lazy<Mutex<System>> = Lazy::new(|| Mutex::new(System::new_all()));

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|_app, _args, _cwd| {}))
        .plugin(tauri_plugin_wallpaper::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            let icon = app
                .default_window_icon()
                .cloned()
                .ok_or("Errore: nessuna icona!")?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit SYSTATS", true, None::<&str>)?;
            let tray_menu = Menu::with_items(app, &[&quit_item])?;

            let _ = TrayIconBuilder::new()
                .icon(icon)
                .tooltip("SYSTATS")
                .menu(&tray_menu)
                .on_menu_event(|app, event| {
                    if event.id.as_ref() == "quit" {
                        app.exit(0)
                    }
                })
                .build(app);

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_network_speed_measurement,
            start_fps_measurement,
            get_cpu_stats,
            get_ram_stats,
            get_gpu_stats,
            log,
            devtools
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[tauri::command]
fn start_network_speed_measurement(app: AppHandle) {
    if IS_MEASURING_NETWORK_SPEED.swap(true, SeqCst) {
        return;
    }
    
    std::thread::spawn(move || {
        let mut networks = Networks::new_with_refreshed_list();
        
        let mut last_check = Instant::now();
        
        loop {
            std::thread::sleep(Duration::from_millis(1000));
            networks.refresh(true);

            let now = Instant::now();
            let elapsed_time = now.duration_since(last_check).as_secs_f64();

            let mut received = 0;
            let mut transmitted = 0;

            for (_name, network) in &networks {
                received += network.received();
                transmitted += network.transmitted();
            }

            if elapsed_time > 0.0 {
                let download_speed_mb_s = (received as f64 / elapsed_time) / (1024.0 * 1024.0);
                let upload_speed_mb_s = (transmitted as f64 / elapsed_time) / (1024.0 * 1024.0);

                let _ = app.emit(
                    "on_network_speed_measured",
                    NetworkSpeed {
                        net_download_speed: download_speed_mb_s,
                        net_upload_speed: upload_speed_mb_s,
                    },
                );
            }

            last_check = now;
        }
    });
}

#[tauri::command]
fn start_fps_measurement(app: AppHandle) {
    if IS_MEASURING_FPS.swap(true, SeqCst) {
        return;
    }

    std::thread::spawn(move || loop {
        if let Ok(window) = Window::foreground() {
            let settings = Settings::new(
                window,
                CursorCaptureSettings::WithoutCursor,
                DrawBorderSettings::WithoutBorder,
                SecondaryWindowSettings::Exclude,
                MinimumUpdateIntervalSettings::Custom(Duration::from_millis(1)),
                DirtyRegionSettings::Default,
                ColorFormat::Rgba8,
                app.clone(),
            );

            let _ = FPSCounter::start(settings);
        }

        std::thread::sleep(Duration::from_millis(200));
    });
}

#[tauri::command]
async fn get_cpu_stats() -> HashMap<String, u8> {
    let mut cpu_stats_map = HashMap::new();

    let Ok(mut sys) = SYS.lock() else {
        return cpu_stats_map;
    };

    sys.refresh_cpu_usage();

    cpu_stats_map.insert(
        "cpu_usage".to_string(),
        sys.global_cpu_usage().round() as u8,
    );

    // println!("CPU usage: {:?}%", sys.global_cpu_usage() as u8);
    cpu_stats_map
}

#[tauri::command]
async fn get_ram_stats() -> HashMap<String, u64> {
    let mut ram_stats_map = HashMap::new();

    let Ok(mut sys) = SYS.lock() else {
        return ram_stats_map;
    };

    sys.refresh_memory();

    ram_stats_map.insert(
        "ram".to_string(),
        ((sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0) as u64,
    );

    // println!("{}", ((sys.used_memory() as f64 / sys.total_memory() as f64) * 100.0) as u64);
    ram_stats_map
}

#[tauri::command]
async fn get_gpu_stats() -> HashMap<String, u8> {
    let mut gpu_stats_map = HashMap::new();

    let Ok(nvml) = Nvml::init() else {
        return gpu_stats_map;
    };

    let Ok(device) = nvml.device_by_index(0) else {
        return gpu_stats_map;
    };

    if let Ok(gpu_memory) = device.utilization_rates() {
        gpu_stats_map.insert("gpu_memory".to_string(), gpu_memory.gpu as u8);
    } else {
        gpu_stats_map.insert("gpu_memory".to_string(), 0);
    }

    let gpu_temp = device.temperature(Gpu).unwrap_or(0);
    gpu_stats_map.insert("gpu_temp".to_string(), gpu_temp as u8);

    let gpu_fan_speed_0 = device.fan_speed(0).unwrap_or(0);
    gpu_stats_map.insert("gpu_fan_speed_0".to_string(), gpu_fan_speed_0 as u8);

    let gpu_fan_speed_1 = device.fan_speed(1).unwrap_or(0);
    gpu_stats_map.insert("gpu_fan_speed_1".to_string(), gpu_fan_speed_1 as u8);

    let gpu_fan_speed_2 = device.fan_speed(2).unwrap_or(0);
    gpu_stats_map.insert("gpu_fan_speed_2".to_string(), gpu_fan_speed_2 as u8);

    // println!("{:#?}", gpu_stats_map);
    gpu_stats_map
}

// test functions
#[tauri::command]
async fn log(logs: serde_json::Value) {
    if IS_DEV {
        if logs.is_null() {
            println!("undefined/null");
        } else {
            match serde_json::to_string_pretty(&logs) {
                Ok(pretty) => println!("{}", pretty),
                Err(_e) => println!("{}", logs),
            }
        }
    }
}

#[tauri::command]
fn devtools(app: AppHandle, label: &str) {
    if let Some(_win) = app.get_webview_window(label) {
        #[cfg(debug_assertions)]
        _win.open_devtools();
    }
}
