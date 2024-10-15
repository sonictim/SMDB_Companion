use crate::assets::*;
use crate::config::*;
use crate::app::*;
use crate::processing::*;
use eframe::egui::{self, RichText};
// use egui::accesskit::Node;
// use egui::Order;
use rayon::prelude::*;
// use sqlx::Pool;
// use sqlx::SqlitePool;
use std::collections::HashSet;
use std::fs::{self};
// use tokio::sync::mpsc;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct DupePanel {
    pub main: NodeConfig,
    pub basic: NodeConfig,
    pub match_criteria: Vec<String>,
    pub match_null: bool,
    pub new_criteria: String,
    pub sel_criteria: Vec<usize>,
    pub deep: NodeConfig,
    pub ignore_filetypes: bool,
    pub gathering_extensions: bool,
    pub tags: NodeConfig,
    pub compare: NodeConfig,


    #[serde(skip)]
    pub compare_db: Option<Database>,
    #[serde(skip)]
    cdb_io: AsyncTunnel<Database>,
    
    pub remove: RemoveConfig,

    pub go_search: bool,
    pub go_remove: bool,

    pub marked_records: String,
    pub scroll_to_top: bool,
    pub records_window: bool,
}


impl DupePanel {
    pub fn render(&mut self, ui: &mut egui::Ui, db: Option<&Database>, registered: Option<bool>, order: &OrderPanel, tags: &Vec<String>) {
        if let Some(db) = db {
            if db.size == 0 {
                ui.heading("No Records in Database");
                return;
            }
            ui.columns(2, |column| {
                column[0].heading(RichText::new("Search for Duplicate Records").strong());

                //BASIC BASIC BASIC
                column[0].checkbox(&mut self.basic.enabled, "Basic Duplicate Search");
                column[0].horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label("Duplicate Match Criteria: ");
                });
                self.render_match_criteria(&mut column[0], db);

                if column[0].input(|i| i.modifiers.alt) {
                    column[0].horizontal(|ui| {
                        ui.add_space(24.0);
                        ui.label("Unmatched Records: ");
                        ui.radio_value(&mut self.match_null, false, "Ignore");
                        ui.radio_value(&mut self.match_null, true, "Process Together");
                    });
                }
                self.remove.render(&mut column[1]);
            });

            self.basic.progress_bar(ui);

            //DEEP DIVE DEEP DIVE DEEP DIVE
            ui.checkbox(&mut self.deep.enabled, "Similar Filename Duplicates Search")
                .on_hover_text_at_pointer(
                    "Filenames ending in .#, .#.#.#, or .M will be examined as possible duplicates",
                );

            if db.file_extensions.is_empty() {
                // db.get_extensions(tx);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("Gathering Filetypes from DB");
                    self.clear_status();
                });
            } else {
                ui.horizontal(|ui| {
                    ui.add_space(24.0);

                    if db.file_extensions.len() > 1 {
                        let text = if self.ignore_filetypes {"Checked: 'example.wav' and 'example.flac' will be considered duplicate filenames"}
                        else {"Unchecked: 'example.wav' and 'example.flac' will be considered unique filenames"};
                        ui.checkbox(&mut self.ignore_filetypes, "Ignore Filetypes").on_hover_text_at_pointer(text);

                    } else {
                        ui.label("All Records are of Filetype:");
                        ui.label(&db.file_extensions[0]);
                    }
                });
            }

            self.deep.progress_bar(ui);

            //TAGS TAGS TAGS TAGS
            self.tags.render(
                ui,
                "Search for Records with AudioSuite Tags in Filename",
                "Filenames with Common Protools AudioSuite Tags will be marked for removal",
                Some(|| {}),
            );

            //COMPARE COMPARE COMPARE COMPARE

            ui.horizontal(|ui| {
                ui.checkbox(&mut self.compare.enabled, "Compare against database: ")
                    .on_hover_text_at_pointer
                        ("Filenames from Target Database found in Comparison Database will be Marked for Removal");
                
                if let Some(cdb) = &self.compare_db {
                    if ui.selectable_label(false, &cdb.name).clicked() {
                   
                        match self.cdb_io.tx.clone() {
                            Some(tx) => {
                                tokio::spawn(async move {
                                    let db = open_db().await.unwrap();
                                    let _ = tx.send(db).await;
                                });

                            },
                            None => println!("TX is NONE"),
                        }
                        

                    }
                    
                }

                else {
                    self.compare.enabled = false;
                    if ui.button("Select DB").clicked()  {
                        self.compare.enabled = false;
                        match self.cdb_io.tx.clone() {
                            Some(tx) => {
                                tokio::spawn(async move {
                                    let db = open_db().await.unwrap();
                                    let _ = tx.send(db).await;
                                });

                            },
                            None => println!("TX is NONE"),
                        }
                        
                    }


                }
                
                // if let Some(rx) = self.cdb_io.rx.as_mut() {
                //     if let Ok(db) = rx.try_recv() {
                //         self.c_db = Some(db);
                //         self.compare.enabled = true;
                //     }
                // }
            });
            if !self.compare.enabled {
                ui.label(RichText::new("Please select DB to enable").weak());
                ui.horizontal(|_ui| {});
            } else {
                node_progress_bar(ui, &self.compare);
            };

            ui.separator();

            ui.horizontal(|_ui| {});

            ui.horizontal(|ui| {
                
                if self.handles_active() {
                    self.go_remove = false;
                    button(ui, "Cancel", || self.abort_all());
                } else {
                    self.go_remove = true;
                    if self.search_eligible() {

                        
                        if ui.input(|i| i.modifiers.alt) {
                           
                            rt_button(ui, light_red_text("Search and Remove Duplicates").size(20.0), || {
                                self.go_search = true;
                                self.go_remove = false;
                                self.gather_duplicates(db, &order.extract_sql(), tags);
                            });
                        } else {
                            ui.columns(2, |column|{
                                column[0].horizontal(|ui| {

                                    rt_button(ui, RichText::new("Search for Duplicates").size(20.0).strong(), || self.gather_duplicates(db, &order.extract_sql(), tags));
                                });
    
                                if !self.main.records.is_empty() && !self.handles_active() {
                                    self.main.status = format!(
                                        "{} total records marked for removal",
                                        self.main.records.len()
                                    );
                                    column[1].horizontal(|ui|{
                                        rt_button(ui, RichText::new("Remove Duplicates").size(20.0).strong(), || self.remove_duplicates(db, registered));
                                    

                                    });
                                }
    
                            });
                        }
                    }
                    else {
                        ui.label(RichText::new("No Search Methods are enabled").strong().size(20.0));
                    }

                }

                if self.go_remove && self.go_search {
                    self.go_remove = false;
                    self.go_search = false;
                    self.remove_duplicates(db, registered);
                }
            });
            empty_line(ui);

            ui.horizontal(|ui| {
                if self.main.working {
                    ui.spinner();
                }
                ui.label(RichText::new(self.main.status.clone()).strong());
            });

            if registered == Some(true)
                && !self.handles_active()
                && !self.main.records.is_empty()
                && ui.button("Show Records").clicked()
            {
                let mut marked_records: Vec<&str> = self
                    .main
                    .records
                    .par_iter() // Use parallel iterator
                    .map(|s| s.path.as_str()) // Convert &String to &str
                    .collect();

                // Sort in parallel
                marked_records.par_sort(); // Rayon provides parallel sorting

                // Join the sorted records with newline characters
                self.marked_records = marked_records.join("\n");

                self.scroll_to_top = true;
                self.records_window = true;
            }

            if self.main.working {
                ui.add(
                    egui::ProgressBar::new(self.main.progress.0 / self.main.progress.1)
                        // .text("progress")
                        .desired_height(4.0),
                );
            }
            self.receive_async_data();
        } else {
            ui.heading(RichText::new("No Open Database").weak());
        }
    }

    pub fn receive_async_data(&mut self) {
        if let Some(records) = self.main.receive_hashset() {
            self.clear_status();
            self.main.status = format! {"Removed {} duplicates", records.len()};
            // self.main.records.clear();
        }

        if let Some(records) = self.basic.receive_hashset() {
            self.main.records.extend(records);
        }

        if let Some(records) = self.deep.receive_hashset() {
            self.main.records.extend(records);
        }

        if let Some(records) = self.tags.receive_hashset() {
            self.main.records.extend(records);
        }

        if let Some(records) = self.compare.receive_hashset() {
            self.main.records.extend(records);
        }

        self.main.receive_progress();
        self.main.receive_status();
        self.basic.receive_progress();
        self.basic.receive_status();
        self.deep.receive_progress();
        self.deep.receive_status();
        self.tags.receive_progress();
        self.tags.receive_status();
        self.compare.receive_progress();
        self.compare.receive_status();

        if let Some(rx) = self.cdb_io.rx.as_mut() {
            if let Ok(db) = rx.try_recv() {
                self.compare_db = Some(db);
                self.compare.enabled = true;
            }
        }
    }

   


    pub fn render_match_criteria(&mut self, ui: &mut egui::Ui, db: &Database) {
        if self.basic.list.is_empty() {
            self.basic.enabled = false;

            ui.horizontal(|ui| {
                ui.add_space(24.0);

                ui.label(light_red_text("Add Match Criteria to Enable Search").size(14.0));
            });
            ui.horizontal(|ui| {
                ui.add_space(24.0);

                button(ui, "Restore Defaults", || {
                    self.basic.list = vec![
                        "Filename".to_owned(),
                        "Duration".to_owned(),
                        "Channels".to_owned(),
                    ]
                });
            });
            empty_line(ui);
        } else {
            ui.horizontal(|ui| {
                ui.add_space(24.0);

                egui::Frame::none() // Use Frame to create a custom bordered area
                    .inner_margin(egui::vec2(8.0, 8.0)) // Inner margin for padding
                    .show(ui, |ui| {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                // Drawing a border manually
                                ui.add_space(2.0);
                                selectable_grid(
                                    ui,
                                    "Match Grid",
                                    4,
                                    &mut self.sel_criteria,
                                    &mut self.basic.list,
                                );

                                ui.add_space(2.0);
                            });
                        });
                    });
            });
        }
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.label(RichText::new("Add:"));

            let mut filtered_list = db.columns.clone();
            filtered_list.retain(|item| !&self.basic.list.contains(item));

            combo_box(ui, "group", &mut self.basic.selected, &filtered_list);

            if !self.basic.selected.is_empty() {
                let item = self.basic.selected.clone();
                self.basic.selected.clear();
                if !self.basic.list.contains(&item) {
                    self.basic.list.push(item);
                }
            }

            button(ui, "Remove Selected", || {
                let mut sorted_indices: Vec<usize> = self.sel_criteria.clone();
                sorted_indices.sort_by(|a, b| b.cmp(a)); // Sort in reverse order

                for index in sorted_indices {
                    if index < self.basic.list.len() {
                        self.basic.list.remove(index);
                    }
                }
                self.sel_criteria.clear();
                self.basic.selected.clear();
            });
        });
    }

    pub fn gather_duplicates(&mut self, db: &Database, order: &Vec<String>, tags: &Vec<String>,) {
        self.abort_all();
        self.main.records.clear();
   
        let Some(pool) = db.pool.clone() else {return};

      
        self.main.status = "Searching for Duplicates".to_string();
       

        if self.basic.enabled {
            if let Some(progress_sender) = self.basic.progress_io.tx.clone() {

                let pool = pool.clone();
                let order = order.clone();
                let groups = self.basic.list.clone();
                let group_null = self.match_null;
      
                wrap_async(
                    &mut self.basic,
                    "Searching For Duplicate Records",
                    move || gather_duplicate_filenames_in_database(pool, order , groups, group_null, progress_sender),
                )
            }
        }

        if self.deep.enabled {
            if let Some(progress_sender) = self.deep.progress_io.tx.clone() {
                if let Some(status_sender) = self.deep.status_io.tx.clone() {
                    let pool = pool.clone();
                    let ignore = self.ignore_filetypes;
                    wrap_async(
                        &mut self.deep,
                        "Searching for Duplicates with similar Filenames",
                        move || gather_deep_dive_records(pool, progress_sender, status_sender, ignore),
                    )
                }
            }
        }

        if self.tags.enabled {
            if let Some(sender) = self.tags.progress_io.tx.clone() {

                let pool = pool.clone();
                let tags = tags.clone();
                wrap_async(
                    &mut self.tags,
                    "Searching for Filenames with Specified Tags",
                    move || gather_filenames_with_tags(pool, tags, sender),
                );
            }
        }

        if self.compare.enabled && self.compare_db.is_some() {
            if let Some(cdb) = &self.compare_db {
                self.compare.working = true;
                self.compare.status = format!("Comparing against {}", cdb.name);
                if self.compare.records_io.tx.is_none() {
                    println!("compare tx is none");
                }
                if let Some(tx) = self.compare.records_io.tx.clone() {
                    println!("if let some");
                    let p = pool.clone();
                    let Some(c_pool) = cdb.pool.clone() else {return;};
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



    pub fn remove_duplicates(&mut self, db: &Database, registered: Option<bool>) {
        if registered == Some(false) {
            self.main.records.clear();
            self.main.status = "Unregistered!\nPlease Register to Remove Duplicates".to_string();
            return;
        }
      
            let mut work_db_path: Option<String> = Some(db.path.clone());
            let mut duplicate_db_path: Option<String> = None;
            let records = self.main.records.clone();
    
            self.main.working = true;
            if self.remove.safe {
                self.main.status = "Creating Safety Database".to_string();
                let path = format!("{}_thinned.sqlite", &db.path.trim_end_matches(".sqlite"));
                let _result = fs::copy(&db.path, &path);
                work_db_path = Some(path);
            }
            if self.remove.create_dupe_db {
                self.main.status = "Creating Database of Duplicates".to_string();
                let path = format!("{}_dupes.sqlite", &db.path.trim_end_matches(".sqlite"));
                let _result = fs::copy(&db.path, &path);
                duplicate_db_path = Some(path);
            }
    
            if let Some(sender) = self.main.progress_io.tx.clone() {
                if let Some(sender2) = self.main.status_io.tx.clone() {
                    wrap_async(&mut self.main, "Performing Record Removal", move || {
                        remove_duplicates_go(records, work_db_path, duplicate_db_path, sender, sender2)
                    })
                }
            }
            if self.remove.remove_files {
                println!("Removing Files");
                let files: HashSet<&str> = self
                    .main
                    .records
                    .par_iter()
                    .map(|record| record.path.as_str())
                    .collect();
    
                let _ = self.remove.delete.delete_files(files);
          
            
        }
    }



    fn clear_status(&mut self) {
        self.main.status.clear();
        self.main.records.clear();
        self.basic.status.clear();
        self.basic.records.clear();
        self.tags.status.clear();
        self.tags.records.clear();
        self.deep.status.clear();
        self.deep.records.clear();
        self.compare.status.clear();
        self.compare.records.clear();
        self.gathering_extensions = false;
    }

    fn handles_active(&mut self) -> bool {
        self.main.handle.is_some()
            || self.basic.handle.is_some()
            || self.deep.handle.is_some()
            || self.tags.handle.is_some()
            || self.compare.handle.is_some()
    }
    fn abort_all(&mut self) {
        self.main.abort();
        self.basic.abort();
        self.deep.abort();
        self.tags.abort();
        self.compare.abort();
    }

    fn search_eligible(&mut self) -> bool {
        self.main.enabled
            || self.basic.enabled
            || self.deep.enabled
            || self.tags.enabled
            || self.compare.enabled
    }
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct RemoveConfig {
    pub safe: bool,
    pub create_dupe_db: bool,
    pub remove_files: bool,
    pub delete: Delete,
}

impl RemoveConfig {
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Remove Options").strong());
        let mut text = RichText::new("Create New Safety Database of Thinned Records");
        if !self.safe {
            text = text.strong().color(egui::Color32::from_rgb(255, 100, 100))
        }
        ui.checkbox(&mut self.safe, text);
        if !&self.safe {
            ui.horizontal(|ui| {
                ui.label(red_text("UNSAFE!"));
                ui.label(RichText::new("Will remove records from current database").strong());
            });
        }
        ui.checkbox(
            &mut self.create_dupe_db,
            "Create New Database of Duplicate Records",
        );
        ui.horizontal_wrapped(|ui| {
            let mut text = RichText::new("Remove Duplicate Files From Disk ");
            if self.remove_files {
                text = text
                    .strong()
                    .size(14.0)
                    .color(egui::Color32::from_rgb(255, 100, 100))
            }
            ui.checkbox(&mut self.remove_files, text);

            if self.remove_files {
                enum_combo_box(ui, &mut self.delete);
                if self.remove_files && self.delete == Delete::Permanent {
                    ui.label(
                        RichText::new("UNSAFE!")
                            .color(egui::Color32::from_rgb(255, 0, 0))
                            .strong(),
                    );
                    ui.label(RichText::new("This is NOT undoable").strong());
                }
            }
        });
    }
}
