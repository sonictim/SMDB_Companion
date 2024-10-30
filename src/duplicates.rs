use crate::prelude::*;

pub mod basic;
pub mod compare;
pub mod deep;
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
    main: NodeConfig,
    basic: Basic,
    deep: Deep,
    tags: Tags,
    compare: Compare,

    remove: Remove,

    #[serde(skip)]
    gather_dupes: bool,
    #[serde(skip)]
    go_search: bool,
    #[serde(skip)]
    go_remove: bool,
}

// impl Default for Duplicates {
//     fn default() -> Self {
//         Self {}
//     }
// }

impl Duplicates {
    pub fn render(&mut self, ui: &mut egui::Ui, db: Option<&Database>) {
        let Some(db) = db else {
            ui.heading(RichText::new("No Open Database").weak());
            return;
        };
        if db.size == 0 {
            ui.heading("No Records in Database");
            return;
        }

        ui.columns(2, |column| {
            column[0].heading(RichText::new("Search for Duplicate Records").strong());
            //BASIC BASIC BASIC

            self.basic.render(&mut column[0], db);
            self.remove.render(&mut column[1]);
        });
        self.basic.render_progress_bar(ui);

        self.deep.render(ui, db);
        self.deep.render_progress_bar(ui);

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
                                self.gather_duplicates();
                            },
                        );
                    } else {
                        ui.columns(2, |column| {
                            column[0].horizontal(|ui| {
                                rt_button(
                                    ui,
                                    RichText::new("Search for Duplicates").size(20.0).strong(),
                                    || self.gather_duplicates(),
                                );
                            });
                            if !self.handles_active() && !self.main.records.is_empty() {
                                column[1].horizontal(|ui| {
                                    rt_button(
                                        ui,
                                        light_red_text("Remove Duplicates").size(20.0).strong(),
                                        || self.remove_duplicates(),
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
                self.remove_duplicates();
            }
        });
        empty_line(ui);

        ui.horizontal(|ui| {
            if self.main.working {
                ui.spinner();
            }
            ui.label(RichText::new(&*self.main.status).strong());
        });

        if self.registration.valid == Some(true)
            && !self.handles_active()
            && !self.main.records.is_empty()
            && ui.button("Show Records").clicked()
        {
            let mut marked_records: Vec<&str> = self
                .main
                .records
                .par_iter() // Use parallel iterator
                .map(|s| &*s.path) // Convert &String to &str
                .collect();

            // Sort in parallel
            marked_records.par_sort();
            self.marked_records = marked_records.join("\n");
            self.scroll_to_top = true;
            self.records_window = true;
        }

        if self.main.working {
            ui.add(
                egui::ProgressBar::new(self.main.progress.0 / self.main.progress.1)
                    .desired_height(4.0),
            );
        }
    }

    pub fn gather_duplicates(&mut self) {
        self.abort_all();
        self.main.records.clear();
        if let Some(db) = self.db.as_ref() {
            let Some(pool) = db.pool() else { return };

            self.main.status = "Searching for Duplicates".into();

            if self.basic.enabled() {
                let progress_sender = self.basic.progress_io.tx.clone();
                let status_sender = self.basic.status_io.tx.clone();
                let pool = pool.clone();
                let order = self.order_panel.extract_sql().clone();
                let match_groups = self.match_criteria.get().to_vec();
                let match_null = self.match_null;
                self.basic.wrap_async(move || {
                    self.basic.gather(
                        pool,
                        progress_sender,
                        status_sender,
                        order,
                        match_groups,
                        match_null,
                    )
                })
            }

            if self.deep.enabled() {
                let progress_sender = self.deep.progress_io.tx.clone();
                let status_sender = self.deep.status_io.tx.clone();
                let pool = pool.clone();
                let ignore = self.ignore_extension;
                self.deep.wrap_async(move || Deep::gather(self, db))
            }

            if self.tags.enabled {
                let progress_sender = self.tags.progress_io.tx.clone();
                let status_sender = self.tags.status_io.tx.clone();
                let pool = pool.clone();
                let tags = self.tags_panel.list().to_vec();
                self.tags.wrap_async(move || {
                    gather_filenames_with_tags(pool, progress_sender, status_sender, tags)
                });
            }

            if self.compare.enabled && self.compare_db.is_some() {
                if let Some(cdb) = &self.compare_db {
                    self.compare.working = true;
                    self.compare.status = format!("Comparing against {}", cdb.name).into();

                    let tx = self.compare.records_io.tx.clone();
                    let p = pool.clone();
                    let Some(c_pool) = cdb.pool() else {
                        return;
                    };
                    let handle = tokio::spawn(async move {
                        println!("tokio spawn compare");
                        let results = gather_compare_database_overlaps(&p, &c_pool).await;
                        if (tx.send(results.expect("error on compare db")).await).is_err() {
                            eprintln!("Failed to send db");
                        }
                    });
                    self.compare.handle = Some(handle);
                }
            }
        }
    }

    // fn reset_to_defaults(&mut self) {
    //     let db = self.db.take();
    //     let panel = self.my_panel;
    //     let registration = self.registration.clone();
    //     *self = Self::default();
    //     self.db = db;
    //     self.my_panel = panel;
    //     self.registration = registration;
    //     self.check_for_updates();
    // }

    fn reset_to_tjf_defaults(&mut self) {
        self.default();
        self.order_panel.list = tjf_order();
        self.tags_panel.list.set(tjf_tags());
        self.deep.enabled = true;
        self.tags.enabled = true;
        self.dupes_db = false;
        self.ignore_extension = true;
        self.match_criteria.set(vec!["Filename".to_owned()]);
    }

    fn clear_status(&mut self) {
        self.main.default();
        self.main.status = "".into();
        self.main.records.clear();
        self.basic.status = "".into();
        self.basic.records.clear();
        self.tags.status = "".into();
        self.tags.records.clear();
        self.deep.status = "".into();
        self.deep.records.clear();
        self.compare.status = "".into();
        self.compare.records.clear();
        self.extensions_io.waiting = false;
    }

    fn abort_all(&mut self) {
        self.main.abort();
        self.basic.abort();
        self.deep.abort();
        self.tags.abort();
        self.compare.abort();
    }

    fn handles_active(&self) -> bool {
        self.main.handle.is_some()
            || self.basic.handle.is_some()
            || self.deep.handle.is_some()
            || self.tags.handle.is_some()
            || self.compare.handle.is_some()
    }

    fn search_eligible(&self) -> bool {
        self.main.enabled
            || self.basic.enabled
            || self.deep.enabled
            || self.tags.enabled
            || self.compare.enabled
    }

    fn receive_async_data(&mut self) {
        if let Some(records) = self.main.receive() {
            self.clear_status();
            self.main.status = format! {"Removed {} duplicates", records.len()}.into();
        }
        if let Some(records) = self.basic.receive() {
            self.main.records.extend(records);
            self.update_main_status();
        }
        if let Some(records) = self.deep.receive() {
            self.main.records.extend(records);
            self.update_main_status();
        }
        if let Some(records) = self.tags.receive() {
            self.main.records.extend(records);
            self.update_main_status();
        }
        if let Some(records) = self.compare.config.receive() {
            self.main.records.extend(records);
            self.update_main_status();
        }
    }

    fn update_main_status(&mut self) {
        // if self.handles_active() { return }

        if self.main.records.is_empty() {
            self.main.status = "No Records Marked for Removal".into()
        } else {
            self.main.status = format!(
                "{} total records marked for removal",
                self.main.records.len()
            )
            .into();
        }
    }
}
