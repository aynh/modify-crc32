use std::path::PathBuf;

use eframe::egui;
use flume::{Receiver, Sender};

use crate::worker::{WorkerOutput, WorkerState, WorkerTask};

pub struct App {
    file_dialog: Option<egui_file::FileDialog>,
    file_path: PathBuf,
    file_crc32: u32,
    new_crc32_string: String,
    tx: Sender<WorkerTask>,
    rx: Receiver<WorkerOutput>,
    worker_state: WorkerState,
}

impl App {
    pub fn new(tx: Sender<WorkerTask>, rx: Receiver<WorkerOutput>) -> App {
        App {
            tx,
            rx,
            worker_state: WorkerState::Idle,
            file_crc32: 0,
            new_crc32_string: String::new(),
            file_path: PathBuf::default(),
            file_dialog: None,
        }
    }

    fn reset(&mut self) {
        self.worker_state = WorkerState::Idle;
        self.file_crc32 = 0;
        self.new_crc32_string = String::new();
        self.file_path = PathBuf::default();
        self.file_dialog = None;
    }

    fn idle(&self) -> bool {
        self.worker_state == WorkerState::Idle
    }

    fn ready(&self) -> bool {
        self.idle() && self.file_path.is_file() && self.file_crc32 != 0
    }

    fn file_name(&self) -> String {
        self.file_path
            .file_name()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                self.file_path
                    .display()
                    .to_string()
                    .split('/')
                    .last()
                    .unwrap_or_default()
                    .to_string()
            })
    }

    fn open_file_dialog(&mut self) {
        let file_path_parent = self.file_path.parent().map(|p| p.to_path_buf());
        let mut dialog = egui_file::FileDialog::open_file(file_path_parent)
            .default_size([280.0, 175.0])
            .resizable(false)
            .show_new_folder(false)
            .show_rename(false);

        dialog.open();
        self.file_dialog = dialog.into();
    }

    fn send_worker_task(&mut self, task: WorkerTask) {
        match task {
            WorkerTask::CalculateCrc32 { path: _ } => {
                self.worker_state = WorkerState::CalculatingCrc32 { progress: 0.0 };
            }
            WorkerTask::PatchFile {
                path: _,
                old_crc32: _,
                new_crc32: _,
            } => {
                self.worker_state = WorkerState::PatchingFile;
            }
        }

        self.tx.send(task).unwrap();
    }

    fn handle_worker_output(&mut self, ctx: &eframe::egui::Context, modal: &egui_modal::Modal) {
        if let Ok(output) = self.rx.try_recv() {
            ctx.request_repaint();
            match output {
                WorkerOutput::Crc32(value) => {
                    self.worker_state = WorkerState::Idle;
                    self.file_crc32 = value;
                }
                WorkerOutput::CalculateCrc32Progress(progress) => {
                    self.worker_state = WorkerState::CalculatingCrc32 { progress };
                }
                WorkerOutput::PatchedFile { success } => {
                    self.worker_state = WorkerState::Idle;

                    let (title, body, icon) = if success {
                        (
                            "Success",
                            "successfully patched the file",
                            egui_modal::Icon::Success,
                        )
                    } else {
                        ("Error", "failed to patch the file", egui_modal::Icon::Error)
                    };

                    modal.open_dialog(title.into(), body.into(), icon.into());
                    self.reset();
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        let mut modal = egui_modal::Modal::new(ctx, "modify-crc32-modal");
        modal.show_dialog();
        self.handle_worker_output(ctx, &modal);

        if let Some(dialog) = &mut self.file_dialog {
            if dialog.show(ctx).selected() {
                if let Some(path) = dialog.path() {
                    ctx.request_repaint();
                    self.file_path = path.clone();
                    self.file_crc32 = 0;
                    self.send_worker_task(WorkerTask::CalculateCrc32 { path });
                }
            }
        }

        egui::TopBottomPanel::bottom("modify-crc32-bottom-panel").show(ctx, |ui| {
            ui.add_space(5.0);

            ui.with_layout(
                egui::Layout::top_down(eframe::emath::Align::Max),
                |ui| match self.worker_state {
                    WorkerState::CalculatingCrc32 { progress } => {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new());
                            ui.label(format!("calculacting crc32 ({:.2}%)", progress));
                        });
                    }
                    WorkerState::PatchingFile => {
                        ui.horizontal(|ui| {
                            ui.add(egui::Spinner::new());
                            ui.label("patching file");
                        });
                    }
                    WorkerState::Idle => {
                        ui.label("idle");
                    }
                },
            );
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(egui::Layout::top_down(eframe::emath::Align::Center), |ui| {
                ui.group(|ui| {
                    ui.add_enabled_ui(self.idle(), |ui| {
                        if ui.button("select file").clicked() {
                            self.open_file_dialog();
                        }

                        ui.add_space(2.0);

                        if ui
                            .add(
                                default_text_edit(&mut self.file_name())
                                    .desired_width(ui.available_width() - 100.0)
                                    .hint_text("double click to pick a file"),
                            )
                            .double_clicked()
                        {
                            self.open_file_dialog();
                        };

                        ui.add_space(5.0);

                        ui.label("file crc32");
                        ui.add_enabled_ui(false, |ui| {
                            let mut value = if self.file_crc32 == 0 {
                                "".to_string()
                            } else {
                                format!("{:08X}", self.file_crc32)
                            };

                            ui.add(default_text_edit(&mut value).hint_text("no file selected"));
                        });
                    });
                });

                ui.add_space(15.0);

                ui.group(|ui| {
                    ui.label("new crc32");
                    ui.add_enabled_ui(self.ready(), |ui| {
                        if ui
                            .add(default_text_edit(&mut self.new_crc32_string))
                            .changed()
                        {
                            self.new_crc32_string.truncate(8);
                            self.new_crc32_string.make_ascii_uppercase();
                            self.new_crc32_string
                                .retain(|ch| matches!(ch, '0'..='9' | 'A'..='F'));
                        }
                    });

                    ui.add_space(5.0);

                    ui.centered_and_justified(|ui| {
                        let new_crc32 =
                            u32::from_str_radix(&self.new_crc32_string, 16).unwrap_or(0);
                        ui.add_enabled_ui(self.ready() && self.file_crc32 != new_crc32, |ui| {
                            if ui.button("patch").clicked() {
                                self.send_worker_task(WorkerTask::PatchFile {
                                    new_crc32,
                                    old_crc32: self.file_crc32,
                                    path: self.file_path.clone(),
                                })
                            };
                        });
                    });
                });
            });
        });
    }
}

fn default_text_edit(value: &mut String) -> egui::TextEdit {
    egui::TextEdit::singleline(value)
        .desired_width(100.0)
        .horizontal_align(eframe::emath::Align::Center)
}
