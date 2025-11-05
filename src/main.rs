use eframe::egui;
use std::{ffi::{CStr, CString}, fs::File, io::Read, os::fd::OwnedFd};

fn main() {
    //TODO : Optimize later with C String literals - https://doc.rust-lang.org/edition-guide/rust-2021/c-string-literals.html
   let fd = unsafe {
    match nix::pty::forkpty(None, None).unwrap() {
        nix::pty::ForkptyResult::Parent { master, child: _ } => master,
        nix::pty::ForkptyResult::Child => {
            let shell_name = CStr::from_bytes_with_nul(b"ash\0").expect("This should always have a null terminator");
            nix::unistd::execvp::<CString>(shell_name, &[]).unwrap();
            return;
        }
    }
    };



    let native_options = eframe::NativeOptions::default();
    eframe::run_native("Claritty", native_options, Box::new(move|cc| Ok(Box::new(Claritty::new(cc, fd))))).expect("eframe failed");
}
struct Claritty {
    buf: Vec<u8>,
    fd: File
    
}

impl Claritty {
    fn new(_cc: &eframe::CreationContext<'_>, fd: OwnedFd) -> Self {
        // Customize egui here with cc.egui_ctx.set_fonts and cc.egui_ctx.set_visuals.
        // Restore app state using cc.storage (requires the "persistence" feature).
        // Use the cc.gl (a glow::Context) to create graphics shaders and buffers that you can use
        // for e.g. egui::PaintCallback.
        Claritty{
            buf: Vec::new(),
            fd: fd.into(),
        }
    }
}

impl eframe::App for Claritty {
   fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
       let mut buf = vec![0u8; 4096];
       match self.fd.read(&mut buf) {
            Ok(read_size) => {
                self.buf.extend_from_slice(&buf[0..read_size]);
            }
            Err(e) => {
                println!("Failed to read : {e}");
            }
       }

       egui::CentralPanel::default().show(ctx, |ui| {
           unsafe{
            //FIX : Unsafe code
           ui.label(std::str::from_utf8_unchecked(&self.buf));
           }
           
       });
   }
}