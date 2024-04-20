use std::{
    error::Error,
    fs::File,
    io::Read,
    num::NonZeroU8,
    path::PathBuf,
    sync::mpsc::{channel, Receiver},
    thread,
    time::Duration,
};

use eframe::{
    egui::{mutex::Mutex, CentralPanel},
    run_native, App,
};

#[derive(Default)]
pub struct Main {
    conductor_path: Option<PathBuf>,
    pub conductor: Option<Conductor>,
    conductor_channel: Option<Receiver<Conductor>>,
}

impl Main {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Default::default()
    }

    pub fn set_conductor_path(&mut self, conductor_path: Option<PathBuf>) {
        self.conductor_path = conductor_path;
        // clear the previous one
        self.conductor = None;
        self.conductor_channel = None;
    }

    pub fn conductor_path(&self) -> Option<&PathBuf> {
        self.conductor_path.as_ref()
    }
}

impl App for Main {
    fn update(&mut self, ctx: &eframe::egui::Context, frame: &mut eframe::Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Pikmin 2 Music Reader");
            });
            ui.separator();
            // Check if there's a conductor path selected
            if let Some(conductor_path) = &self.conductor_path {
                // Show conductor path
                ui.horizontal(|ui| {
                    ui.label("File Path:");
                    ui.monospace(conductor_path.to_str().unwrap_or("Invalid path!"));
                });
                // Check if there's a Conductor struct in memory
                if let Some(conductor) = &self.conductor {
                    // Show Conductor
                    ui.label("success!");
                    ui.horizontal_top(|ui| {
                        ui.label("Bpm:");
                        ui.monospace(conductor.bpm.to_string());
                    });
                // No Conductor in memory, check if we're currently waiting on the parsing thread
                } else if let Some(reciever) = &self.conductor_channel {
                    // Check if the parsing is done
                    if let Ok(conductor) = reciever.try_recv() {
                        self.conductor = Some(conductor);
                    } else {
                        // Show loading text
                        ui.horizontal(|ui| {
                            ui.label("Parsing file...");
                            ui.spinner();
                        });
                    }
                } else {
                    // No thread active, spawn a parsing thread
                    let (sender, reciever) = channel();
                    self.conductor_channel = Some(reciever);
                    let file_path: PathBuf = self.conductor_path.clone().expect("Somehow");
                    let file_path_clone = file_path.clone();
                    let ctx = ctx.clone();

                    thread::spawn(move || -> Result<(), Box<dyn Error + Send + Sync>> {
                        let conductor = Conductor::from_file(&file_path_clone)?;
                        let send = sender.send(conductor);
                        ctx.request_repaint();
                        Ok(send?)
                    });
                }
            } else {
                // No file selected
                ui.label("Drag a .cnd file into the window to read it:");
            }
            if ui.button("Open conductor file…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Conductor files", &["cnd"])
                    .pick_file()
                {
                    self.set_conductor_path(Some(path));
                }
            }
        });
    }

    fn raw_input_hook(
        &mut self,
        _ctx: &eframe::egui::Context,
        _raw_input: &mut eframe::egui::RawInput,
    ) {
        // Check if user dropped file on screen
        if let Some(dropped_file) = _raw_input.dropped_files.first() {
            println!("${:#?}", dropped_file);
            // if it's a conductor file
            if let Some(path) = &dropped_file.path {
                if path.extension().unwrap() == "cnd" {
                    self.set_conductor_path(dropped_file.path.clone());
                }
            }
        }
    }
}

#[derive()]
pub struct Conductor {
    pub louie_swing: u8,
    pub bpm: u8,
    pub track_count: NonZeroU8,
    pub tracks: Vec<Track>,
}

impl Conductor {
    pub fn from_file(file: &PathBuf) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let mut file = File::open(file)?;
        let mut metadata_buffer: [u8; 3] = [0; 3];
        file.read_exact(&mut metadata_buffer)?;
        let [louie_swing, bpm, track_count] = metadata_buffer;
        // simulate work
        thread::sleep(Duration::from_secs(2));
        // TODO: actually parse the file
        Ok(Self {
            louie_swing,
            bpm,
            track_count: NonZeroU8::new(track_count).unwrap(),
            tracks: Vec::new(),
        })
    }
}

pub struct Track {
    pub init_delay: u8,
    pub b_offset_flag: bool,
    pub q_offset_flag: bool,
    description: [u8; 8],
    pub track_copy: u8,
    pub echo: u8,
    pub ordered: bool,
    pub bank: Bank,
    pub program: u8,
    pub gesture_set: u8,
    pub timing: u8,
    pub gesture_count: u8,
    pub silent_count: u8,
    pub transposition: i8,
    pub volume: u8,
    pub panning: i8,
}

impl Track {
    pub fn description(&self) -> &str {
        std::str::from_utf8(&self.description).unwrap_or("!!Description string is corrupted!!")
    }
}

pub enum Bank {
    Pikmin1SFX,
    WatanabeSFX,
    TotakaSFX,
    HikinoSFX,
    WakaiInstruments,
    TotakaInstruments,
}

fn main() -> Result<(), eframe::Error> {
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = native_options
        .viewport
        .with_title("Pikmin 2 Music Reader")
        .with_drag_and_drop(true);
    run_native(
        "Pikmin2MusicReader",
        native_options,
        Box::new(|cc| Box::new(Main::new(cc))),
    )
}
