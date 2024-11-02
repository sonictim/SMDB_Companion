use crate::prelude::*;

pub mod basic;
pub mod compare;
pub mod deep;
pub mod nodes;
pub mod remove;
pub mod tags;

use crate::config::*;
use basic::Basic;
use compare::Compare;
use deep::Deep;
use remove::Remove;
use tags::Tags;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Duplicates {
    // main: NodeConfig,
    pub basic: Basic,
    deep: Deep,
    pub tags: Tags,
    compare: Compare,
    remove: Remove,

    // nodes: Vec<Node>,
    #[serde(skip)]
    gather_dupes: bool,
    #[serde(skip)]
    go_search: bool,
    #[serde(skip)]
    go_remove: bool,
    #[serde(skip)]
    pub records_window: RecordsWindow,
}

impl Duplicates {
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
            self.basic.render(&mut column[0], db);
            self.remove.render_options(&mut column[1]);
        });
        self.basic.render_progress_bar(ui);

        self.deep.render(ui, db);
        self.deep.config.render(ui);

        self.tags.render(ui);
        self.tags.render_progress_bar(ui);

        self.compare.render(ui);

        self.compare.render_progress_bar(ui);

        ui.separator();
        empty_line(ui);

        ui.horizontal(|ui| {
            if self.handles_active() {
                self.go_remove = false;
                button(ui, "Cancel", || self.abort_all());
            } else {
                self.go_remove = true;
                if self.search_eligible() {
                    if ui.input(|i| i.modifiers.alt) {
                        rt_button(
                            ui,
                            light_red_text("Search and Remove Duplicates").size(20.0),
                            || {
                                self.go_search = true;
                                self.go_remove = false;
                                self.gather_duplicates(db);
                            },
                        );
                    } else {
                        ui.columns(2, |column| {
                            column[0].horizontal(|ui| {
                                rt_button(
                                    ui,
                                    RichText::new("Search for Duplicates").size(20.0).strong(),
                                    || self.gather_duplicates(db),
                                );
                            });
                            if !self.handles_active() && !self.remove.config.records.is_empty() {
                                column[1].horizontal(|ui| {
                                    rt_button(
                                        ui,
                                        light_red_text("Remove Duplicates").size(20.0).strong(),
                                        || self.remove.remove_duplicates(db),
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

            if self.go_remove && self.go_search {
                self.go_remove = false;
                self.go_search = false;
                self.remove.remove_duplicates(db);
            }
        });
        empty_line(ui);

        ui.horizontal(|ui| {
            if self.remove.config.working {
                ui.spinner();
            }
            ui.label(RichText::new(&*self.remove.config.status).strong());
        });

        if registration == Some(true)
            && !self.handles_active()
            && !self.remove.config.records.is_empty()
            && ui.button("Show Records").clicked()
        {
            self.records_window.open(&self.remove.config.records);
        }

        if self.remove.config.working {
            ui.add(
                egui::ProgressBar::new(
                    self.remove.config.progress.0 / self.remove.config.progress.1,
                )
                .desired_height(4.0),
            );
        }
    }

    pub fn gather_duplicates(&mut self, db: &Database) {
        self.abort_all();
        self.remove.config.records.clear();
        self.remove.config.status = "Searching for Duplicates".into();

        self.basic.process(db);
        self.deep.process(db);
        self.tags.gather(db);
        self.compare.gather(db);
    }

    pub fn reset_to_tjf_defaults(&mut self) {
        *self = Self::default();
        self.basic.tjf_default();
        self.deep.enabled = true;
        self.deep.ignore_extension = true;
        self.tags.config.enabled = true;
        self.tags.list.set(tjf_tags());
        self.remove.dupes_db = false;
    }

    pub fn clear_status(&mut self) {
        self.basic.config.clear();
        self.deep.config.clear();
        self.tags.config.clear();
        self.compare.config.clear();
        self.remove.config.clear();
    }

    pub fn abort_all(&mut self) {
        self.remove.config.abort();
        self.basic.config.abort();
        self.deep.config.abort();
        self.tags.config.abort();
        self.compare.config.abort();
    }

    fn handles_active(&self) -> bool {
        self.remove.config.handle.is_some()
            || self.basic.config.handle.is_some()
            || self.deep.config.handle.is_some()
            || self.tags.config.handle.is_some()
            || self.compare.config.handle.is_some()
    }

    fn search_eligible(&self) -> bool {
        self.remove.config.enabled
            || self.basic.enabled
            || self.deep.enabled
            || self.tags.enabled()
            || self.compare.enabled()
    }

    fn receive_async_data(&mut self) {
        if let Some(records) = self.remove.config.receive() {
            // self.clear_status();
            self.remove.config.status = format! {"Removed {} duplicates", records.len()}.into();
        }

        if let Some(records) = self.basic.config.receive() {
            self.update_main_status(records);
        }
        if let Some(records) = self.deep.config.receive() {
            self.update_main_status(records);
        }
        if let Some(records) = self.tags.config.receive() {
            self.update_main_status(records);
        }
        if let Some(records) = self.compare.config.receive() {
            self.update_main_status(records);
        }
    }

    fn update_main_status(&mut self, records: HashSet<FileRecord>) {
        // if self.handles_active() { return }
        self.remove.config.records.get().extend(records);
        if self.remove.config.records.get().is_empty() {
            self.remove.config.status = "No Records Marked for Removal".into()
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
