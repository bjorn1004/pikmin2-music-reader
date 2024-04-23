#![windows_subsystem = "windows"]
mod conductor;

use std::{
    error::Error,
    path::PathBuf,
    sync::mpsc::{channel, Receiver},
    thread,
};

use conductor::Bank;
use conductor::Conductor;
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
                            let timing_hint = match conductor.louie_swing {
                                30 => "1/16th note",
                                60 => "1/8th note",
                                120 => "1/4th note",
                                _ => "Custom",
                            };
                            ui.monospace(format!("{} ({})", conductor.louie_swing, timing_hint));
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
                } else if let Some(receiver) = &self.conductor_channel {
                    // Check if the parsing is done
                    if let Ok(conductor) = receiver.try_recv() {
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
                    let (sender, receiver) = channel();
                    self.conductor_channel = Some(receiver);
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
