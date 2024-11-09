use crate::prelude::*;

pub mod basic;
pub mod compare;
pub mod deep;
pub mod remove;
pub mod tags;
pub mod waveform;

use basic::Basic;
use compare::Compare;
use deep::Deep;
use remove::Remove;
use tags::Tags;
use waveform::Waveforms;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Duplicates {
    basic: Basic,
    deep: Deep,
    tags: Tags,
    waves: Waveforms,
    compare: Compare,
    remove: Remove,

    #[serde(skip)]
    pub records_window: RecordsWindow,
}

impl Duplicates {
    pub fn tags_panel(&mut self) -> bool {
        if self.tags.open_panel {
            self.tags.open_panel = false;
            return true;
        }
        false
    }
    pub fn render_order_panel(&mut self, ui: &mut egui::Ui, db: Option<&Database>) {
        self.basic.preservation_order.render(ui, db);
    }
    pub fn render_tags_panel(&mut self, ui: &mut egui::Ui) {
        self.tags.render_panel(ui);
    }

    fn nodes(&mut self) -> [&mut dyn NodeCommon; 5] {
        [
            &mut self.basic as &mut dyn NodeCommon,
            &mut self.deep as &mut dyn NodeCommon,
            &mut self.tags as &mut dyn NodeCommon,
            &mut self.waves as &mut dyn NodeCommon,
            &mut self.compare as &mut dyn NodeCommon,
        ]
    }

    pub fn render(&mut self, ui: &mut egui::Ui, db: Option<&Database>, registration: Option<bool>) {
        let Some(db) = db else {
            ui.heading(RichText::new("No Open Database").weak());
            return;
        };
        if db.size == 0 {
            ui.heading("No Records in Database");
            return;
        }

        self.receive_async_data();
        ui.columns(2, |column| {
            column[0].heading(RichText::new("Search for Duplicate Records").strong());
            // for node in self.nodes() {
            //     node.render(&mut column[0], db);
            //     node.render_progress_bar(&mut column[0]);
            // }
            self.basic.render(&mut column[0], db);
            self.remove.render_options(&mut column[1]);
        });
        self.basic.render_progress_bar(ui);

        self.deep.render(ui, db);
        self.deep.render_progress_bar(ui);

        self.tags.render(ui, db);
        self.tags.render_progress_bar(ui);

        self.waves.render(ui, db);
        self.waves.render_progress_bar(ui);

        self.compare.render(ui, db);
        self.compare.render_progress_bar(ui);

        ui.separator();
        empty_line(ui);

        self.render_action_buttons(ui, db, registration);

        empty_line(ui);

        ui.horizontal(|ui| {
            if self.remove.config.working {
                ui.spinner();
            }
            ui.label(RichText::new(&**self.remove.config.status.get()).strong());
        });

        self.render_records_button(ui, registration);
    }
    fn render_action_buttons(
        &mut self,
        ui: &mut egui::Ui,
        db: &Database,
        registration: Option<bool>,
    ) {
        ui.horizontal(|ui| {
            if self.handles_active() {
                self.remove.run = false;
                button(ui, "Cancel", || self.abort_all());
            } else {
                self.remove.run = true;
                if self.search_eligible() {
                    if ui.input(|i| i.modifiers.alt) {
                        rt_button(
                            ui,
                            light_red_text("Search and Remove Duplicates").size(20.0),
                            || {
                                self.remove.enabled = true;
                                self.remove.run = false;
                                self.gather(db);
                            },
                        );
                    } else {
                        ui.columns(2, |column| {
                            column[0].horizontal(|ui| {
                                rt_button(
                                    ui,
                                    RichText::new("Search for Duplicates").size(20.0).strong(),
                                    || self.gather(db),
                                );
                            });
                            if !self.handles_active()
                                && !self.remove.config.records.get().is_empty()
                            {
                                column[1].horizontal(|ui| {
                                    rt_button(
                                        ui,
                                        light_red_text("Remove Duplicates").size(20.0).strong(),
                                        || self.remove.process(db, registration),
                                    );
                                });
                            }
                        });
                    }
                } else {
                    ui.label(
                        RichText::new("No Search Methods are enabled")
                            .strong()
                            .size(20.0),
                    );
                }
            }

            if self.remove.enabled && self.remove.run {
                self.remove.enabled = false;
                self.remove.run = false;
                self.remove.process(db, registration);
            }
        });
    }

    fn render_records_button(&mut self, ui: &mut egui::Ui, registration: Option<bool>) {
        if registration == Some(true)
            && !self.handles_active()
            && !self.remove.config.records.get().is_empty()
            && ui.button("Show Records").clicked()
        {
            self.records_window.open(self.remove.config.records.get());
        }

        if self.remove.config.working {
            ui.add(
                egui::ProgressBar::new(
                    self.remove.config.progress.get().counter as f32
                        / self.remove.config.progress.get().total as f32,
                )
                .desired_height(4.0),
            );
        }
    }

    pub fn gather(&mut self, db: &Database) {
        self.abort_all();
        self.remove.config.records.clear();
        self.remove
            .config
            .status
            .set("Searching for Duplicates".into());

        for node in self.nodes() {
            if node.enabled() {
                node.process(db);
            }
        }
    }

    pub fn reset_to_tjf_defaults(&mut self) {
        *self = Self::default();
        self.basic.tjf_default();
        self.deep.enabled = true;
        self.deep.ignore_extension = true;
        self.tags.enabled = true;
        self.tags.set_tjf();
        self.remove.dupes_db = false;
    }

    pub fn clear_status(&mut self) {
        for node in self.nodes() {
            node.clear();
        }
        self.remove.config.clear();
    }

    pub fn abort_all(&mut self) {
        for node in self.nodes() {
            node.abort();
        }
        self.remove.config.abort();
    }

    fn handles_active(&self) -> bool {
        self.remove.config.handle.is_some()
            || self.basic.config.handle.is_some()
            || self.deep.config.handle.is_some()
            || self.tags.config.handle.is_some()
            || self.compare.config.handle.is_some()
            || self.waves.config.handle.is_some()
    }

    fn search_eligible(&self) -> bool {
        self.remove.enabled
            || self.basic.enabled
            || self.deep.enabled
            || self.tags.enabled
            || self.compare.enabled
            || self.waves.enabled
    }

    fn receive_async_data(&mut self) {
        if let Some(records) = self.remove.config.receive() {
            self.clear_status();
            self.remove
                .config
                .status
                .set(format! {"Removed {} duplicates", records.len()}.into());
        }

        let mut updates = Vec::new();

        for node in &mut self.nodes() {
            if let Some(records) = node.receive() {
                updates.push(records);
            }
        }

        for records in updates {
            self.update_main_status(records);
        }
    }

    fn update_main_status(&mut self, records: HashSet<FileRecord>) {
        self.remove.config.records.get_mut().extend(records);
        if self.remove.config.records.get().is_empty() {
            self.remove
                .config
                .status
                .set("No Records Marked for Removal".into())
        } else {
            self.remove.config.status.set(
                format!(
                    "{} total records marked for removal",
                    self.remove.config.records.get().len()
                )
                .into(),
            );
        }
    }
}

pub trait NodeCommon {
    fn config(&mut self) -> &mut Node;
    fn enabled(&self) -> bool;

    fn receive(&mut self) -> Option<HashSet<FileRecord>> {
        self.config().receive()
    }
    fn abort(&mut self) {
        self.config().abort();
    }
    fn clear(&mut self) {
        self.config().clear()
    }
    fn render_progress_bar(&mut self, ui: &mut egui::Ui) {
        self.config().render(ui);
    }
    fn render(&mut self, ui: &mut egui::Ui, db: &Database);
    fn process(&mut self, db: &Database);
}

pub struct Node {
    pub working: bool,
    pub records: AsyncTunnel<HashSet<FileRecord>>,
    pub status: AsyncTunnel<Arc<str>>,
    pub progress: AsyncTunnel<Progress>,
    pub handle: Option<tokio::task::JoinHandle<()>>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            records: AsyncTunnel::new(1),
            working: false,
            status: AsyncTunnel::new(1),
            progress: AsyncTunnel::new(32),
            handle: None,
        }
    }
}

impl Node {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn abort(&mut self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
        self.clear();
    }

    pub fn receive_hashset(&mut self) -> Option<HashSet<FileRecord>> {
        if let Some(records) = self.records.recv() {
            self.records.set(records.clone());
            self.handle = None;
            self.working = false;
            self.progress.set(Progress::default());
            self.status
                .set(format! {"Found {} duplicate records", records.len()}.into());
            return Some(records);
        }
        None
    }

    pub fn receive_progress(&mut self) {
        while let Some(progress) = self.progress.recv() {
            self.progress.set(progress);
        }
    }

    pub fn receive_status(&mut self) {
        while let Some(message) = self.status.recv() {
            self.status.set(message);
        }
    }
    pub fn receive(&mut self) -> Option<HashSet<FileRecord>> {
        self.receive_progress();
        self.receive_status();
        self.receive_hashset()
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.working {
                ui.spinner();
            } else {
                ui.add_space(24.0)
            }
            if self.working {
                ui.label(format!(
                    "{} / {}",
                    self.progress.get().counter,
                    self.progress.get().total
                ));
            }
            ui.label(RichText::new(&**self.status.get()).strong());
        });

        if self.working {
            ui.add(
                egui::ProgressBar::new(
                    self.progress.get().counter as f32 / self.progress.get().total as f32,
                )
                .desired_height(4.0),
            );
        }
        empty_line(ui);
    }
    pub fn wrap_async<F, T>(&mut self, action: F)
    where
        F: FnOnce() -> T + Send + 'static,
        T: std::future::Future<Output = Result<HashSet<FileRecord>, sqlx::Error>> + Send + 'static,
    {
        self.working = true;
        let tx = self.records.tx.clone();

        let handle = tokio::spawn(async move {
            let results = action().await;
            if (tx.send(results.expect("Tokio Results Error HashSet")).await).is_err() {
                eprintln!("Failed to send db");
            }
        });
        self.handle = Some(handle);
    }
}

#[derive(Default)]
pub struct Progress {
    pub counter: usize,
    pub total: usize,
}

impl Progress {
    pub fn set(&mut self, counter: usize, total: usize) {
        self.counter = counter;
        self.total = total;
    }
}

#[derive(Default)]
pub struct RecordsWindow {
    open: bool,
    scroll_to_top: bool,
    Display_Data: String,
}

impl RecordsWindow {
    pub fn render(&mut self, ctx: &egui::Context) {
        let available_size = ctx.available_rect(); // Get the full available width and height
        let width = available_size.width() - 20.0;
        let height = available_size.height();
        egui::Window::new("Records Marked as Duplicates")
            .open(&mut self.open) // Control whether the window is open
            .resizable(false) // Make window non-resizable if you want it fixed
            .min_width(width)
            .min_height(height)
            .max_width(width)
            .max_height(height)
            .show(ctx, |ui| {
                // ui.label("To Be Implemented\n Testing line break");

                if self.scroll_to_top {
                    egui::ScrollArea::vertical()
                        .max_height(height)
                        .max_width(width)
                        .scroll_offset(egui::vec2(0.0, 0.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new(&self.Display_Data).size(14.0));
                        });
                    self.scroll_to_top = false;
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(height)
                        .max_width(width)
                        .show(ui, |ui| {
                            ui.label(RichText::new(&self.Display_Data).size(14.0));
                        });
                }
            });
    }
    fn open(&mut self, records: &HashSet<FileRecord>) {
        let mut marked_records: Vec<&str> = records
            .par_iter() // Use parallel iterator
            .map(|s| &*s.path) // Convert &String to &str
            .collect();

        // Sort in parallel
        marked_records.par_sort();
        self.Display_Data = marked_records.join("\n");
        self.scroll_to_top = true;
        self.open = true;
    }
}

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy, Default)]
pub enum Delete {
    #[default]
    Trash,
    Permanent,
}
impl EnumComboBox for Delete {
    fn as_str(&self) -> &'static str {
        match self {
            Delete::Trash => "Move to Trash",
            Delete::Permanent => "Permanently Delete",
        }
    }
    fn variants() -> &'static [Delete] {
        &[Delete::Trash, Delete::Permanent]
    }
}
impl Delete {
    pub fn delete_files(&self, files: HashSet<&str>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Removing Files");

        // Filter valid files directly and collect them into a Vec
        let valid_files: Vec<&str> = files
            .par_iter()
            .filter(|&&file| Path::new(file).exists())
            .cloned() // Convert &str to str for collection
            .collect();

        match self {
            Delete::Trash => {
                if !valid_files.is_empty() {
                    trash::delete_all(&valid_files).map_err(|e| {
                        eprintln!("Move to Trash Failed: {}", e);
                        e
                    })?;
                }
            }
            Delete::Permanent => {
                for file in valid_files {
                    fs::remove_file(file).map_err(|e| {
                        eprintln!("Failed to remove file {}: {}", file, e);
                        e
                    })?;
                }
            }
        }

        Ok(())
    }
}
