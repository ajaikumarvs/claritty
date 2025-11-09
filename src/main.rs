use eframe::egui;
use nix::errno::Errno;
use std::{
    ffi::{CStr, CString}, os::fd::{AsRawFd, OwnedFd}
};
use sysinfo::System;

fn main() {
    //TODO : Optimize later with C String literals - https://doc.rust-lang.org/edition-guide/rust-2021/c-string-literals.html
    let fd = unsafe {

        
        match nix::pty::forkpty(None, None).unwrap() {
            nix::pty::ForkptyResult::Parent { master, child: _ } => master,
            nix::pty::ForkptyResult::Child => {
                let shell_name = CStr::from_bytes_with_nul(b"zsh\0")
                    .expect("This should always have a null terminator");
                
                std::env::remove_var("PROMPT_COMMAND");
                std::env::set_var("PS1", "$ ");
                nix::unistd::execvp::<CString>(shell_name, &[]).unwrap();
                return;
            }
        }
    };

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Claritty",
        native_options,
        Box::new(move |cc| Ok(Box::new(Claritty::new(cc, fd)))),
    )
    .expect("eframe failed");
}
struct Claritty {
    buf: Vec<u8>,
    fd: OwnedFd,
    // Performance metrics
    frame_times: Vec<f32>, // Rolling window of frame times for FPS calculation
    last_frame_time: std::time::Instant, // Timestamp of last frame for delta calculation
    system: System,        // System info for CPU/RAM monitoring
    pid: sysinfo::Pid,     // Current process ID
    total_cores: usize,    // Total CPU cores available
}

impl Claritty {
    fn new(_cc: &eframe::CreationContext<'_>, fd: OwnedFd) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        let flags = nix::fcntl::fcntl(&fd, nix::fcntl::FcntlArg::F_GETFL).unwrap();
        let mut flags = nix::fcntl::OFlag::from_bits_truncate(flags);
        flags.insert(nix::fcntl::OFlag::O_NONBLOCK);
        nix::fcntl::fcntl(&fd, nix::fcntl::FcntlArg::F_SETFL(flags)).unwrap();
        
        let mut system = System::new_all();
        system.refresh_all();
        let total_cores = system.cpus().len();

        Claritty {
            buf: Vec::new(),
            fd,
            frame_times: Vec::with_capacity(60),
            last_frame_time: std::time::Instant::now(),
            system,
            pid: sysinfo::Pid::from_u32(std::process::id()),
            total_cores,
        }
    }
}

impl eframe::App for Claritty {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        let mut buf = vec![0u8; 4096];
        match nix::unistd::read(&self.fd, &mut buf) {
            Ok(read_size) => {
                self.buf.extend_from_slice(&buf[0..read_size]);
            }
            Err(e) => {
                if e != Errno::EAGAIN{
                println!("Failed to read : {e}");
                }
            }
        }
        
        // Track frame times
        let now = std::time::Instant::now();
        let frame_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;

        self.frame_times.push(frame_time);
        if self.frame_times.len() > 60 {
            self.frame_times.remove(0);
        }

        // Calculate FPS from average frame time
        let avg_frame_time = if !self.frame_times.is_empty() {
            self.frame_times.iter().sum::<f32>() / self.frame_times.len() as f32
        } else {
            0.0
        };
        let fps = if avg_frame_time > 0.0 {
            1.0 / avg_frame_time
        } else {
            0.0
        };

        // Get CPU, RAM, and thread usage
        self.system.refresh_all();
        let (cpu_usage, ram_usage, thread_count) =
            if let Some(process) = self.system.process(self.pid) {
                (
                    process.cpu_usage(),
                    process.memory() as f64 / 1024.0 / 1024.0,
                    process.tasks().map(|t| t.len()).unwrap_or(0),
                )
            } else {
                (0.0, 0.0, 0)
            };

        // Calculate cores used (cpu_usage is percentage of one core, so divide by 100)
        let cores_used = cpu_usage / 100.0;

       

        egui::CentralPanel::default().show(ctx, |ui| {
            unsafe {
                //FIX : Unsafe code
                ui.label(std::str::from_utf8_unchecked(&self.buf));
            }
        });

        // Performance metrics overlay in top-right corner
        egui::Area::new("fps_overlay".into())
            .fixed_pos(egui::pos2(ctx.viewport_rect().width() - 200.0, 5.0))
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(format!(
                            "{:.1} FPS ({:.2}ms)",
                            fps,
                            avg_frame_time * 1000.0
                        ))
                        .monospace()
                        .color(egui::Color32::WHITE)
                        .background_color(egui::Color32::from_black_alpha(180)),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "CPU: {:.1}% ({:.2}/{} cores)",
                            cpu_usage, cores_used, self.total_cores
                        ))
                        .monospace()
                        .color(egui::Color32::WHITE)
                        .background_color(egui::Color32::from_black_alpha(180)),
                    );
                    ui.label(
                        egui::RichText::new(format!("RAM: {:.1} MB", ram_usage))
                            .monospace()
                            .color(egui::Color32::WHITE)
                            .background_color(egui::Color32::from_black_alpha(180)),
                    );
                    ui.label(
                        egui::RichText::new(format!("Threads: {}", thread_count))
                            .monospace()
                            .color(egui::Color32::WHITE)
                            .background_color(egui::Color32::from_black_alpha(180)),
                    );
                });
            });

        // 4. Request continuous repaints for smooth FPS updates
        ctx.request_repaint();
    }
}
