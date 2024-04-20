use std::{
    error::Error,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    num::NonZeroU8,
    path::PathBuf,
    sync::mpsc::{channel, Receiver},
    thread,
};

use eframe::{
    egui::{CentralPanel, CollapsingHeader, Grid, ScrollArea},
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
            if ui.button("Open conductor file…").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Conductor files", &["cnd"])
                    .pick_file()
                {
                    self.set_conductor_path(Some(path));
                }
            }
            // Check if there's a conductor path selected
            if let Some(conductor_path) = &self.conductor_path {
                // Show conductor path
                ui.horizontal(|ui| {
                    ui.label("File Path:");
                    ui.monospace(conductor_path.to_str().unwrap_or("Invalid path!"));
                });
                // Check if there's a Conductor struct in memory
                if let Some(conductor) = &self.conductor {
                    static INVALID_FILE_ERROR: &str = "Invalid File";
                    ui.heading(
                        conductor_path
                            .file_name()
                            .expect(INVALID_FILE_ERROR)
                            .to_str()
                            .expect(INVALID_FILE_ERROR),
                    );
                    // Show Conductor
                    Grid::new("conductor_metadata")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Louie swing:");
                            ui.monospace(conductor.louie_swing.to_string());
                            ui.end_row();

                            ui.label("Bpm:");
                            ui.monospace(conductor.bpm.to_string());
                            ui.end_row();

                            ui.label("Track count:");
                            ui.monospace(conductor.track_count.to_string());
                            ui.end_row();
                        });
                    ui.separator();
                    ui.heading("Tracks");
                    ScrollArea::vertical().show(ui, |ui| {
                        for (track_id, track) in conductor.tracks.iter().enumerate() {
                            CollapsingHeader::new(track.description())
                                .id_source(format!("{}_{track_id}", track.description()))
                                .show(ui, |ui| {
                                    Grid::new(format!("{}_grid", track.description()))
                                        .num_columns(2)
                                        .show(ui, |ui| {
                                            ui.label("Volume:");
                                            ui.monospace(track.volume.to_string());
                                            ui.end_row();

                                            ui.label("Panning:");
                                            ui.monospace(track.panning.to_string());
                                            ui.end_row();

                                            ui.label("Track copy:");
                                            ui.monospace(track.track_copy.to_string());
                                            ui.end_row();

                                            ui.label("Initial Delay:");
                                            ui.monospace(track.init_delay.to_string());
                                            ui.end_row();

                                            ui.label("Echo:");
                                            ui.monospace(track.echo.to_string());
                                            ui.end_row();

                                            ui.label("Ordered:");
                                            ui.monospace(track.ordered.to_string());
                                            ui.end_row();

                                            ui.label("Bank:");
                                            ui.monospace(match track.bank {
                                                Bank::Pikmin1SFX => "Pikmin 1 SFX",
                                                Bank::WatanabeSFX => "Watanabe SFX",
                                                Bank::TotakaSFX => "Totaka SFX",
                                                Bank::HikinoSFX => "Hikino SFX",
                                                Bank::WakaiInstruments => "Wakai Instruments",
                                                Bank::TotakaInstruments => "Totaka Instruments",
                                            });
                                            ui.end_row();

                                            ui.label("Program:");
                                            ui.monospace(track.program.to_string());
                                            ui.end_row();

                                            ui.label("Gesture set:");
                                            ui.monospace(track.gesture_set.to_string());
                                            ui.end_row();

                                            ui.label("Timing ruleset:");
                                            ui.monospace(track.timing.to_string());
                                            ui.end_row();

                                            ui.label("Gesture count:");
                                            ui.monospace(track.gesture_count.to_string());
                                            ui.end_row();

                                            ui.label("Silent count:");
                                            ui.monospace(track.silent_count.to_string());
                                            ui.end_row();

                                            ui.label("Transposition:");
                                            ui.monospace(track.transposition.to_string());
                                            ui.end_row();

                                            ui.label("B offset flag:");
                                            ui.monospace(track.b_offset_flag.to_string());
                                            ui.end_row();

                                            ui.label("Q offset flag:");
                                            ui.monospace(track.q_offset_flag.to_string());
                                            ui.end_row();
                                        })
                                });
                        }
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
                        let conductor = match Conductor::from_file(&file_path_clone) {
                            Ok(conductor) => conductor,
                            Err(e) => {
                                println!("{}", e);
                                return Err(e);
                            }
                        };
                        let send = sender.send(conductor);
                        ctx.request_repaint();
                        Ok(send?)
                    });
                }
            } else {
                // No file selected
                ui.label("Drag a .cnd file into the window to read it:");
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
        let file = File::open(file)?;
        let mut reader = BufReader::new(file);
        let mut metadata_buffer: [u8; 3] = [0; 3];
        reader.read_exact(&mut metadata_buffer)?;
        let [louie_swing, bpm, track_count] = metadata_buffer;

        // seek to the first byte of the first track
        reader.seek(SeekFrom::Current(21))?;
        let mut tracks = Vec::with_capacity(track_count.into());
        let mut byte_buffer: [u8; 4 * 9] = [0; 4 * 9];
        for i in 0..track_count {
            // read data block
            reader.read_exact(&mut byte_buffer)?;
            // extract initial data
            let ([_, init_delay, b_offset_flag], remain) = byte_buffer.split_at(3) else {
                return Err("Error parsing (1)".into());
            };
            // extract description
            let (description_buffer, remain) = remain.split_at(8);
            let mut description: [u8; 8] = [0; 8];
            description.copy_from_slice(description_buffer);
            // extract track_copy and echo
            let ([track_copy, echo], remain) = remain.split_at(2) else {
                return Err("Error parsing (2)".into());
            };
            // seek 8 bytes forwards
            let (_, remain) = remain.split_at(8);
            // extract remainder
            let [ordered, bank_byte, program, _, gesture_set, _, timing, gesture_count, silent_count, _, transposition, volume, panning, q_offset_flag, _] =
                remain
            else {
                return Err("Error parsing (3)".into());
            };
            let bank = match bank_byte {
                0 => Ok(Bank::Pikmin1SFX),
                1 => Ok(Bank::WatanabeSFX),
                2 => Ok(Bank::TotakaSFX),
                3 => Ok(Bank::HikinoSFX),
                4 => Ok(Bank::WakaiInstruments),
                5 => Ok(Bank::TotakaInstruments),
                _ => Err(format!("Error parsing Bank for track {}", i + 1)),
            }?;
            tracks.push(Track {
                init_delay: *init_delay,
                b_offset_flag: (*b_offset_flag == 1),
                q_offset_flag: (*q_offset_flag == 1),
                description,
                track_copy: *track_copy,
                echo: *echo,
                ordered: (*ordered == 1),
                bank,
                program: *program,
                gesture_set: *gesture_set,
                timing: *timing,
                gesture_count: *gesture_count,
                silent_count: *silent_count,
                transposition: *transposition as i8,
                volume: *volume,
                panning: {
                    let panning_i8: i16 = 64_i16.wrapping_add((*panning).into());
                    panning_i8 as i8
                },
            });
            match reader.seek(SeekFrom::Current(6 * 4)) {
                Ok(_) => (),
                Err(_) => break,
            };
        }

        Ok(Self {
            louie_swing,
            bpm,
            track_count: NonZeroU8::new(track_count).unwrap(),
            tracks,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
        // 205 is apparently the terminating character
        let terminated_string = &self
            .description
            .split(|n| *n == 205)
            .next()
            // if this happens, it means the Description is invalid
            .unwrap_or(&[205]);
        std::str::from_utf8(terminated_string).unwrap_or("!!Description string is corrupted!!")
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
