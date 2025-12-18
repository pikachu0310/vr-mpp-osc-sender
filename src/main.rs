#![windows_subsystem = "windows"]

use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use eframe::egui;
use rosc::{OscMessage, OscPacket, OscType, encoder};

#[derive(Default)]
struct AppState {
    interval_ms: u64,
    hold_ms: u64,
    is_sending: bool,
    dest_port: u16,
}

struct OscSenderApp {
    interval_ms: u64,
    hold_ms: u64,
    checked: bool,
    port: u16,
    state: Arc<Mutex<AppState>>,
}

impl OscSenderApp {
    fn new(_: &eframe::CreationContext<'_>) -> Self {
        let state = Arc::new(Mutex::new(AppState {
            interval_ms: 1000,
            hold_ms: 80,
            is_sending: false,
            dest_port: 9000,
        }));

        let cloned_state = state.clone();
        thread::spawn(move || {
            let socket = UdpSocket::bind("0.0.0.0:0").expect("Failed to bind UDP socket");

            let mut prev_sending = false;

            loop {
                let (interval, hold, sending, port) = {
                    let state = cloned_state.lock().unwrap();
                    (
                        state.interval_ms,
                        state.hold_ms,
                        state.is_sending,
                        state.dest_port,
                    )
                };

                if sending {
                    send_click(&socket, port, hold);
                    prev_sending = true;

                    let rest_ms = interval.saturating_sub(hold).max(1);
                    thread::sleep(Duration::from_millis(rest_ms));
                    continue;
                }

                if prev_sending {
                    send_value(&socket, port, 0);
                }

                prev_sending = sending;

                thread::sleep(Duration::from_millis(interval.max(1)));
            }
        });

        Self {
            interval_ms: 1000,
            hold_ms: 80,
            checked: false,
            port: 9000,
            state,
        }
    }

    fn nudge_port(&mut self, delta: i32) {
        let current = self.port as i32;
        let mut next = current + delta;
        if next < 0 {
            next = 0;
        }
        if next > 65534 {
            next = 65534;
        }

        let snapped = (next & !1) as u16;
        if snapped != self.port {
            self.port = snapped;
            let mut s = self.state.lock().unwrap();
            s.dest_port = self.port;
        }
    }
}

fn send_click(socket: &UdpSocket, port: u16, hold_ms: u64) {
    send_value(socket, port, 1);
    thread::sleep(Duration::from_millis(hold_ms.max(1)));
    send_value(socket, port, 0);
}

fn send_value(socket: &UdpSocket, port: u16, value: i32) {
    let msg = OscMessage {
        addr: "/input/UseRight".to_string(),
        args: vec![OscType::Int(value)],
    };
    if let Ok(buf) = encoder::encode(&OscPacket::Message(msg)) {
        let addr = format!("127.0.0.1:{}", port);
        let _ = socket.send_to(&buf, &addr);
    }
}

impl eframe::App for OscSenderApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("OSC Sender");

            if ui
                .add(
                    egui::Slider::new(&mut self.interval_ms, 10..=2000).text("Click interval (ms)"),
                )
                .changed()
            {
                let mut state = self.state.lock().unwrap();
                state.interval_ms = self.interval_ms;
            }

            if ui
                .add(egui::Slider::new(&mut self.hold_ms, 10..=500).text("Hold duration (ms)"))
                .changed()
            {
                let mut state = self.state.lock().unwrap();
                state.hold_ms = self.hold_ms;
            }

            if ui.checkbox(&mut self.checked, "Send OSC").changed() {
                let mut state = self.state.lock().unwrap();
                state.is_sending = self.checked;
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Destination Port:");

                ui.add_enabled_ui(!self.checked, |ui| {
                    if ui.button("−").clicked() {
                        self.nudge_port(-2);
                    }
                    let mut port_str = self.port.to_string();
                    ui.add(
                        egui::TextEdit::singleline(&mut port_str)
                            .hint_text("port")
                            .interactive(false)
                            .desired_width(70.0),
                    );
                    if ui.button("+").clicked() {
                        self.nudge_port(2);
                    }
                });
            });

            let mut display = format!("{}:localhost:{}", self.port, (self.port as u32) + 1);
            ui.label("Quick Launcher OSC setting value");
            if ui
                .add(egui::TextEdit::singleline(&mut display).desired_width(220.0))
                .changed()
            {}
        });

        ctx.request_repaint_after(Duration::from_millis(16));
    }
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([280.0, 160.0]),
        ..Default::default()
    };

    eframe::run_native(
        "OSC Sender",
        options,
        Box::new(|cc| Ok(Box::new(OscSenderApp::new(cc)) as Box<dyn eframe::App>)),
    )
    .unwrap();
}
