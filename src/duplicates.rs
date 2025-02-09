
use crate::prelude::*;

pub mod basic;
pub mod compare;
pub mod deep;
pub mod duration;
pub mod order;
pub mod remove;
pub mod tags;
pub mod valid_path;
pub mod waveform;

use basic::Basic;
use compare::Compare;
use deep::Deep;
use duration::Duration;
pub use order::OrderPanel;
use remove::Remove;
use tags::Tags;
use valid_path::PathValid;
use waveform::Waveforms;

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy, Default)]
pub enum Algorithm {
    #[default]
    All,
    Basic,
    Deep,
    Waveforms,
    Compare,
    Tags,
    ValidPath,
    Duration,
}

impl EnumComboBox for Algorithm {
    fn as_str(&self) -> &'static str {
        match self {
            Algorithm::Basic => "Basic Duplicate Search",
            Algorithm::Deep => "Similar Filename",
            Algorithm::Tags => "Audiosuite Tags",
            Algorithm::Duration => "Duration",
            Algorithm::ValidPath => "Invalid Filepath",
            Algorithm::Waveforms => "Audio Content Duplicates",
            Algorithm::Compare => "Compare Database",
            Algorithm::All => "All",
        }
    }
    fn variants() -> &'static [Algorithm] {
        &[
            Algorithm::All,
            Algorithm::Basic,
            Algorithm::Deep,
            Algorithm::Waveforms,
            Algorithm::Compare,
            Algorithm::Tags,
            Algorithm::ValidPath,
            Algorithm::Duration,
        ]
    }

}


#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Duplicates {
    pub basic: Basic,
    pub deep: Deep,
    pub tags: Tags,
    pub duration: Duration,
    pub valid_path: PathValid,
    pub waves: Waveforms,
    pub compare: Compare,
    pub remove: Remove,
    pub order: Arc<RwLock<OrderPanel>>,

    #[serde(skip)]
    pub records_window: RecordsWindow,
    
    #[serde(skip)]
    pub algorithm: Algorithm,
    #[serde(skip)]
    pub last: Algorithm,
    #[serde(skip)]
    cached_records: Vec<String>,
    #[serde(skip)]
    current_set: Vec<FileRecord>,
}

impl Duplicates {
    pub fn tags_panel(&mut self) -> bool {
        if self.tags.open_panel {
            self.tags.open_panel = false;
            return true;
        }
        false
    }
    pub fn find_panel(&mut self) -> bool {
        if self.valid_path.open_panel {
            self.valid_path.open_panel = false;
            return true;
        }
        false
    }
    pub fn render_order_panel(&mut self, ui: &mut egui::Ui, db: Option<&Database>) {
        let mut order = self.order.write().unwrap(); // Lock the RwLock for mutable access
        order.render(ui, db);
    }
    pub fn render_tags_panel(&mut self, ui: &mut egui::Ui) {
        self.tags.render_panel(ui);
    }

    pub fn render_remove_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, db: Option<&Database>, registration: Option<bool>) {
        if registration == Some(false) {
            ui.label(light_red_text("Registration Required to use this feature").size(18.0));
            return;
        }
    
        let Some(db) = db else {
            ui.heading(RichText::new("No Open Database").weak());
            welcome_message(ui);
            return;
        };
        
        if db.size == 0 {
            ui.heading("No Records in Database");
            welcome_message(ui);
            return;
        }
    
        self.receive_async_data();
    
        // Calculate available space once
        let available_size = ctx.available_rect();
        let width = available_size.width() - 20.0;
        let height = available_size.height();
    
        ui.heading(RichText::new("Records Marked for Removal:").strong());
        ui.label(RichText::new(
            "These records have been marked for removal based on the rules established in File Preservation Logic.\n\
            To see all the possible matching records for a filename, search for the filename in your Soundminer.\n\
            If you find you prefer a different file be selected for removal, you will need to update the File Preservation Logic accordingly."
        ).size(14.0));
        
        ui.add_space(8.0);
        
        ui.horizontal(|ui| {
            ui.label(RichText::new("Records to Display").size(16.0));
            enum_combo_box(ui, &mut self.algorithm, "algorithm");
        });
    
        ui.separator();
    
        // Update cached records only when algorithm changes
        if self.last != self.algorithm {
            self.update_cached_records();
            self.last = self.algorithm;
        }
    
        let text_height = 20.0;
        if self.cached_records.is_empty() {
            ui.label(light_red_text(&format!("{} Has No Records to Display", self.algorithm.as_str())).size(16.0));
        }
        
        egui::ScrollArea::vertical()
            .max_height(height - 150.0)
            .max_width(width)
            .auto_shrink([false; 2])
            .id_salt("records_scroll_area")
            .show_rows(
                ui,
                text_height,
                self.cached_records.len(),
                |ui, row_range| {
                    for row in row_range {
                        if let Some(record) = self.cached_records.get(row) {
                            let label = egui::Label::new(
                                RichText::new(record)
                                    .size(14.0)
                                    .family(egui::FontFamily::Monospace)
                            );
                            ui.add(label);
                        }
                    }
                },
            );
    }
    
    pub fn update_cached_records(&mut self) {
        let records = match self.algorithm {
            Algorithm::All => self.remove.config.records.get(),
            Algorithm::Basic => self.basic.config.records.get(),
            Algorithm::Deep => self.deep.config.records.get(),
            Algorithm::Waveforms => self.waves.config.records.get(),
            Algorithm::Compare => self.compare.config.records.get(),
            Algorithm::Tags => self.tags.config.records.get(),
            Algorithm::ValidPath => self.valid_path.config.records.get(),
            Algorithm::Duration => self.duration.config.records.get(), 
        };

        // Clone the HashSet to a Vec once when algorithm changes
        self.current_set = records.iter().cloned().collect();
        
        // Now work with the cloned Vec instead of the original HashSet
        let mut cached = Vec::with_capacity(self.current_set.len());
        cached.extend(self.current_set.iter().map(|record| record.path.to_string()));
        cached.sort_unstable();
        self.cached_records = cached;
    }
    // pub fn render_remove_panel(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, db: Option<&Database>, registration: Option<bool>) {
    //     if registration == Some(false) {
    //         ui.label(light_red_text("Registration Required to use this feature").size(18.0));
    //         return;
    //     }



    //     let Some(db) = db else {
    //         ui.heading(RichText::new("No Open Database").weak());
    //         welcome_message(ui);
    //         return;
    //     };
        
    //     if db.size == 0 {
    //         ui.heading("No Records in Database");
    //         welcome_message(ui);
    //         return;
    //     }

    //     self.receive_async_data();

        
    //     // REMOVE BUTTONS
    //     // ui.columns(2, |column| {
    //     //     large_button2(&mut column[0], "Remove Records", || self.remove.process(db, registration));
            
    //     //     column[0].horizontal(|ui| {
    //     //         if self.remove.config.working {
    //     //             ui.spinner();
    //     //         }
    //     //         ui.label(RichText::new(&**self.remove.config.status.get()).strong());
    //     //     });

    //     //     column[0].horizontal(|ui| {
    //     //         ui.label(RichText::new("Records to Display").size(16.0));
    //     //         egui::ComboBox::from_id_salt("marked records")
    //     //             .selected_text(&self.records_window.selected)
    //     //             .show_ui(ui, |ui| {
    //     //                 for (k, v) in &self.records_window.records {
    //     //                     if !v.is_empty() {
    //     //                         ui.selectable_value(&mut self.records_window.selected, k.clone(), RichText::new(k).size(16.0));
    //     //                     }
    //     //                 }
    //     //             });
    //     //     });
    
    //     //     self.remove.render_options(&mut column[1]);
    //     // });


    //     ui.heading(RichText::new("Records Marked for Removal:").strong());
    //     ui.label(RichText::new(
    //         "These records have been marked for removal based on the rules established in File Preservation Logic.\n\
    //         To see all the possible matching records for a filename, search for the filename in your Soundminer.\n\
    //         If you find you prefer a different file be selected for removal, you will need to update the File Preservation Logic accordingly."
    //     ).size(14.0));
        
    //     ui.add_space(8.0);
        
    //     ui.horizontal(|ui| {
    //         ui.label(RichText::new("Records to Display").size(16.0));
    //         enum_combo_box(ui, &mut self.algorithm, "algorithm");
    //     }); 


    //     ui.separator();

    //     let available_size = ctx.available_rect();
    //     let width = available_size.width() - 20.0;
    //     let height = available_size.height();

    //     if self.last != self.algorithm {
    //         self.update_cached_records();
    //         self.last = self.algorithm;
    //     }

    //     let text_height = 20.0;
    //     if self.cached_records.is_empty() {
    //         ui.label(light_red_text(&format!("{} Has No Records to Display", self.algorithm.as_str())).size(16.0));
    //     }
    //     egui::ScrollArea::vertical()
    //         .max_height(height - 150.0)
    //         .max_width(width)
    //         .auto_shrink([false; 2])
    //         .id_salt("records_scroll_area")
    //         .show_rows(
    //             ui,
    //             text_height,
    //             self.cached_records.len(),
    //             |ui, row_range| {
    //                 for row in row_range {
    //                     if let Some(record) = self.cached_records.get(row) {
    //                         let label = egui::Label::new(
    //                             RichText::new(record)
    //                                 .size(14.0)
    //                                 .family(egui::FontFamily::Monospace)
    //                         );
    //                         ui.add(label);
    //                     }
    //                 }
    //             },
    //         );

        






    // }

    // fn update_cached_records(&mut self) {

    //     let records = match self.algorithm {
    //         Algorithm::All => self.remove.config.records.get(),
    //         Algorithm::Basic => self.basic.config.records.get(),
    //         Algorithm::Deep => self.deep.config.records.get(),
    //         Algorithm::Waveforms => self.waves.config.records.get(),
    //         Algorithm::Compare => self.compare.config.records.get(),
    //         Algorithm::Tags => self.tags.config.records.get(),
    //         Algorithm::ValidPath => self.valid_path.config.records.get(),
    //         Algorithm::Duration => self.duration.config.records.get(), 
    //     };

    //     // if let Some(records) = self.algorithm.get_records() {
    //         // Pre-allocate the vector to avoid reallocations
    //         let capacity = records.len();
    //         let mut cached = Vec::with_capacity(capacity);
    //         cached.extend(
    //             records
    //                 .par_iter()
    //                 .map(|record| record.path.to_string())
    //                 .collect::<Vec<_>>()
    //         );
    //         cached.par_sort();
    //         self.cached_records = cached;
    //     // } else {
    //     //     self.cached_records.clear();
    //     // }
    // }

    fn nodes(&mut self) -> [&mut dyn NodeCommon; 7] {
        [
            &mut self.basic as &mut dyn NodeCommon,
            &mut self.deep as &mut dyn NodeCommon,
            &mut self.waves as &mut dyn NodeCommon,
            &mut self.compare as &mut dyn NodeCommon,
            &mut self.valid_path as &mut dyn NodeCommon,
            &mut self.tags as &mut dyn NodeCommon,
            &mut self.duration as &mut dyn NodeCommon,
        ]
    }

    pub fn render(&mut self, ui: &mut egui::Ui, db: Option<&Database>, registration: Option<bool>) {
        let Some(db) = db else {
            ui.heading(RichText::new("No Open Database").weak());
            welcome_message(ui);
            return;
        };
        if db.size == 0 {
            ui.heading("No Records in Database");
            welcome_message(ui);
            return;
        }

        self.receive_async_data();

        self.render_action_buttons(ui, db, registration);

        ui.horizontal(|ui| {
            if self.remove.config.working {
                ui.spinner();
            }
       
            ui.label(RichText::new(&**self.remove.config.status.get()).strong());
        //     if ui.selectable_label(false, RichText::new(&**self.remove.config.status.get()).strong()).clicked() 
        //     && registration == Some(true)
        //     && !self.handles_active()
        //     && self.has_records()
            
        // {
        //     self.records_window.open(
        //         self.remove.config.records.get(),
        //         self.basic.config.records.get(),
        //         self.deep.config.records.get(),
        //         self.waves.config.records.get(),
        //         self.compare.config.records.get(),
        //         self.tags.config.records.get(),
        //         self.duration.config.records.get(),
        //         self.valid_path.config.records.get(),
        //     );
        // }

        // if self.remove.config.working {
        //     ui.add(
        //         egui::ProgressBar::new(
        //             self.remove.config.progress.get().counter as f32
        //                 / self.remove.config.progress.get().total as f32,
        //         )
        //         .desired_height(4.0),
        //     );
        // }
        });

        self.render_records_button(ui, registration);

        empty_line(ui);


        ui.columns(2, |column| {
            // column[0].heading(RichText::new("Search for Duplicate Records").strong());
            for node in self.nodes() {
                node.render(&mut column[0], db);
                node.render_progress_bar(&mut column[0]);
            }

            if column[0].input(|i| i.modifiers.alt)
                || !self.handles_active() && !self.remove.config.records.get().is_empty()
            {
                self.remove.render_options(&mut column[1]);
            }

            // self.basic.render(&mut column[0], db);
        });
        // self.basic.render_progress_bar(ui);

        // self.deep.render(ui, db);
        // self.deep.render_progress_bar(ui);

        // self.tags.render(ui, db);
        // self.tags.render_progress_bar(ui);

        // self.compare.render(ui, db);
        // self.compare.render_progress_bar(ui);

        // ui.separator();
        // empty_line(ui);

        // self.render_action_buttons(ui, db, registration);

        // empty_line(ui);

        // ui.horizontal(|ui| {
        //     if self.remove.config.working {
        //         ui.spinner();
        //     }
        //     ui.label(RichText::new(&**self.remove.config.status.get()).strong());
        // });

        // self.render_records_button(ui, registration);


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
                large_button(ui, "Cancel", || self.abort_all());
            } else {
                self.remove.run = true;
                if self.search_eligible() {
                    if ui.input(|i| i.modifiers.alt) {
                        large_button2(
                            ui,
                            "Search and Remove Duplicates",
                            || {
                                self.remove.enabled = true;
                                self.remove.run = false;
                                self.gather(db);
                            },
                        );
                    } else {
                        ui.columns(2, |column| {
                            let text = if self.remove.config.records.get().is_empty() {
                                "Search for Duplicates"
                            } else {
                                "Clear and Search Again"
                            };
                            column[0].horizontal(|ui| {
                                large_button(ui, text, || {
                                    self.gather(db)
                                });
                            });
                            if !self.handles_active()
                                && !self.remove.config.records.get().is_empty()
                            {
                                column[1].horizontal(|ui| {
                                    large_button2(
                                        ui,
                                        "Remove Duplicates",
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
            && self.has_records()
            && ui.button("Show Marked Records").clicked()
        {
            self.records_window.clear();
            self.records_window.open(
                self.remove.config.records.get(),
                self.basic.config.records.get(),
                self.deep.config.records.get(),
                self.waves.config.records.get(),
                self.compare.config.records.get(),
                self.tags.config.records.get(),
                self.duration.config.records.get(),
                self.valid_path.config.records.get(),
            );
            self.records_window.update_cached_records();
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
        self.records_window.clear();
        self.remove
            .config
            .status
            .set("Searching for Duplicates".into());

        let columns = self.get_required_metadata_columns();
        let order = self.order.clone();
        for node in &mut self.nodes() {
            if node.enabled() {
                node.process(db, &columns, order.clone());
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
        self.order.write().unwrap().tjf_default();
    }

    pub fn clear_status(&mut self) {
        for node in self.nodes() {
            node.clear();
        }
        self.remove.config.clear();
        self.records_window.clear();
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
            || self.duration.config.handle.is_some()
            || self.valid_path.config.handle.is_some()
    }

    fn search_eligible(&self) -> bool {
        self.remove.enabled
            || self.basic.enabled
            || self.deep.enabled
            || self.tags.enabled
            || self.compare.enabled
            || self.waves.enabled
            || self.duration.enabled
            || self.valid_path.enabled
    }

    fn has_records(&self) -> bool {
        !self.basic.config.records.get().is_empty()
        || !self.deep.config.records.get().is_empty()
        || !self.tags.config.records.get().is_empty()
        || !self.compare.config.records.get().is_empty()
        || !self.waves.config.records.get().is_empty()
        || !self.duration.config.records.get().is_empty()
        || !self.remove.config.records.get().is_empty()
        || !self.valid_path.config.records.get().is_empty()
    
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
    pub fn get_required_metadata_columns(&self) -> HashSet<String> {
        let mut set = HashSet::new();
        set.insert("rowid".to_string());
        set.insert("Filename".to_string());
        set.insert("Pathname".to_string());
        set.insert("FilePath".to_string());
        for item in self.basic.match_criteria.get() {
            set.insert(item.clone());
        }
        set.extend(self.order.read().unwrap().get_columns());
        set
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
        self.config().clear();
    }
    fn render_progress_bar(&mut self, ui: &mut egui::Ui) {
        self.config().render(ui);
    }
    fn render(&mut self, ui: &mut egui::Ui, db: &Database);

    fn process(&mut self, db: &Database, columns: &HashSet<String>, order: Arc<RwLock<OrderPanel>>);
}

pub struct Node {
    pub working: bool,
    pub records: AsyncTunnel<HashSet<FileRecord>>,
    pub status: AsyncTunnel<Arc<str>>,
    // pub status2: Arc<Mutex<String>>,
    pub progress: AsyncTunnel<Progress>,
    pub handle: Option<tokio::task::JoinHandle<()>>,
}

impl Default for Node {
    fn default() -> Self {
        Self {
            records: AsyncTunnel::new(1),
            working: false,
            status: AsyncTunnel::new(1),
            // status2: Default::default(),
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
        self.working = false;
        self.handle = None;
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
        // if let Ok(status2) = self.status2.try_lock() {
        //     ui.label(format!("Status2: {}", status2));
        // }
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
    records: HashMap<String, HashSet<FileRecord>>,
    selected: String,
    last: String,
    keys: Vec<String>,
    cached_records: Vec<String>,
}

impl RecordsWindow {
    pub fn render(&mut self, ctx: &egui::Context) {
        let available_size = ctx.available_rect();
        let width = available_size.width() - 20.0;
        let height = available_size.height();

        let mut is_open = self.open;
        egui::Window::new("Records Marked as Duplicates")
            .open(&mut is_open)
            .resizable(true)
            .min_width(width)
            .min_height(height)
            .max_width(width)
            .max_height(height)
            .show(ctx, |ui| {
                ui.heading(RichText::new("Records Marked for Removal:").strong());
                ui.label(RichText::new(
                    "These records have been marked for removal based on the rules established in File Preservation Logic.\n\
                    To see all the possible matching records for a filename, search for the filename in your Soundminer.\n\
                    If you find you prefer a different file be selected for removal, you will need to update the File Preservation Logic accordingly."
                ).size(14.0));
                
                ui.add_space(8.0);
                
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Records to Display").size(16.0));
                    egui::ComboBox::from_id_salt("marked records")
                        .selected_text(&self.selected)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.selected, "All".to_owned(), RichText::new("All").size(16.0));

                            for (k, v) in &self.records {
                                if !v.is_empty() && k != "All" {
                                    ui.selectable_value(&mut self.selected, k.clone(), RichText::new(k).size(16.0));
                                }
                            }
                        });
                });

                ui.separator();

                if self.last != self.selected {
                    self.update_cached_records();
                    self.last = self.selected.clone();
                }

                let text_height = 20.0;
                if self.selected.is_empty() {self.selected = "All".to_owned()};
                if self.cached_records.is_empty() {
                    ui.label(light_red_text(&format!("{} Has No Records to Display", self.selected)).size(16.0));
                }
                egui::ScrollArea::vertical()
                    .max_height(height - 150.0)
                    .max_width(width)
                    .auto_shrink([false; 2])
                    .id_salt("records_scroll_area")
                    .show_rows(
                        ui,
                        text_height,
                        self.cached_records.len(),
                        |ui, row_range| {
                            for row in row_range {
                                if let Some(record) = self.cached_records.get(row) {
                                    let label = egui::Label::new(
                                        RichText::new(record)
                                            .size(14.0)
                                            .family(egui::FontFamily::Monospace)
                                    );
                                    ui.add(label);
                                }
                            }
                        },
                    );
            });

        self.open = is_open;
    }

    fn update_cached_records(&mut self) {
        if let Some(records) = self.records.get(&self.selected) {
            // Pre-allocate the vector to avoid reallocations
            let capacity = records.len();
            let mut cached = Vec::with_capacity(capacity);
            cached.extend(
                records
                    .par_iter()
                    .map(|record| record.path.to_string())
                    .collect::<Vec<_>>()
            );
            cached.par_sort();
            self.cached_records = cached;
        } else {
            self.cached_records.clear();
        }
    }

    fn clear(&mut self) {
        self.records.clear();
        self.keys.clear();
        // self.selected.clear();
        self.last.clear();
        self.cached_records.clear();
    }

    fn add(&mut self, name: &str, set: &HashSet<FileRecord>) {
        // if !set.is_empty() {
            self.records.insert(name.to_owned(), set.clone());
            self.keys.push(name.to_owned());
        // };
    }

    fn open(
        &mut self,
        all: &HashSet<FileRecord>,
        basic: &HashSet<FileRecord>,
        deep: &HashSet<FileRecord>,
        waveform: &HashSet<FileRecord>,
        compare: &HashSet<FileRecord>,
        tags: &HashSet<FileRecord>,
        duration: &HashSet<FileRecord>,
        valid: &HashSet<FileRecord>,
    ) {
        self.add("All", all);
        self.add("Basic Duplicate Search", basic);
        self.add("Similar Filename", deep);
        self.add("Audio Content Duplicates", waveform);
        self.add("Audiosuite Tags", tags);
        self.add("Duration", duration);
        self.add("Invalid Filepath", valid);
        self.add("Compare Database", compare);

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