use crate::assets::*;
use crate::processing::*;
use crate::config::*;
// use crate::dupe_panel::*;
use eframe::egui::{self, RichText};
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::{self};
// use std::sync::Arc;
use tokio::sync::mpsc;
use std::sync::{Arc, Mutex};
use crate::window_manager::{SharedState, create_new_window};



/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {

    #[serde(skip)]
    db: Option<Database>,
    #[serde(skip)]
    db_io: AsyncTunnel<Database>,
    #[serde(skip)]
    extensions_io: AsyncTunnel<Vec<String>>,
    #[serde(skip)]
    latest_version: String,
    #[serde(skip)]
    latest_version_io: AsyncTunnel<String>,
    #[serde(skip)]
    update_available: bool,
    #[serde(skip)]
    update_window: Option<bool>,
    
    
    #[serde(skip)]
    my_panel: Panel,
    find_panel: FindPanel,
    order_panel: OrderPanel,
    tags_panel: TagsPanel, 
    registration: RegistrationPanel,
    
    main: NodeConfig,
    basic: NodeConfig,
    match_criteria: SelectableList,
    match_null: bool,
    
    deep: NodeConfig,
    ignore_extension: bool,
    tags: NodeConfig,
    compare: NodeConfig,
    #[serde(skip)]
    compare_db: Option<Database>,
    #[serde(skip)]
    cdb_io: AsyncTunnel<Database>,

    safe: bool,
    dupes_db: bool,
    remove_files: bool,
    delete_action: Delete,

    #[serde(skip)]
    gather_dupes: bool,
    #[serde(skip)]
    go_search: bool,
    #[serde(skip)]
    go_remove: bool,

    #[serde(skip)]
    marked_records: String,
    #[serde(skip)]
    records_window: bool,
    scroll_to_top: bool,
    
    #[serde(skip)]
    shared_state: Option<Arc<Mutex<SharedState>>>, 
}

impl Default for App {
    fn default() -> Self {
        let mut app = Self {
            db: None,
            db_io: AsyncTunnel::new(1),
            extensions_io: AsyncTunnel::new(1),
            latest_version_io: AsyncTunnel::new(1),
            latest_version: String::new(),
            update_available: false,
            update_window: None,
            
            compare_db: None,
            cdb_io: AsyncTunnel::new(1),
            
            my_panel: Panel::Duplicates,
            find_panel: FindPanel::default(),
            order_panel: OrderPanel::default(),
            tags_panel: TagsPanel::default(),
            registration: RegistrationPanel::default(),
            
            main: NodeConfig::new(false),
            basic: NodeConfig::new(true),
            match_criteria: SelectableList::default(),
            match_null: false,
            
            tags: NodeConfig::new(false),
            deep: NodeConfig::new(false),
            ignore_extension: false,
            compare: NodeConfig::new(false),
           
            safe: true,
            dupes_db: true,
            remove_files: false,
            delete_action: Delete::Trash,

            gather_dupes: false,
            go_search: false,
            go_remove: false,

            marked_records: String::new(),
            records_window: false,
            scroll_to_top: false,

            shared_state: None,

           
        };
        app.match_criteria.set(vec!["Channels".to_owned(), "Duration".to_owned(), "Filename".to_owned()]);      
        app.tags_panel.list.set(default_tags());
        app.order_panel.list = default_order();

        app
    }
}


impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>, shared_state: Arc<Mutex<SharedState>>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // let stroke = egui::Stroke {
        //     width: 1.0,
        //     color: egui::Color32::LIGHT_RED
        // };

        // let w = Widgets {
        //     active: true,
            
        // }
        // let visuals = egui::Visuals {
        //     widgets.active: false,
        //     dark_mode: false,
            
        //     ..Default::default()
        // };

        // cc.egui_ctx.set_visuals(visuals);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Self {
            shared_state: Some(shared_state),
            ..Default::default()
        }
        // Default::default()
    }

    pub fn new_secondary_window(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize with minimal state needed for secondary windows
        Self {
            // Initialize only the fields needed for secondary windows...
            shared_state: None,
            ..Default::default()
        }
    }

    // Method to open a new window
    pub fn open_new_window(&self, title: &str, size: (f64, f64)) {
        if let Some(shared_state) = &self.shared_state {
            create_new_window(shared_state.clone(), title, size);
        }
    }

    fn reset_to_defaults(&mut self)  {
        let db = self.db.take();
        let panel = self.my_panel;
        let registration = self.registration.clone();
        *self = Self::default();
        self.db = db;
        self.my_panel = panel;
        self.registration = registration;
        self.check_for_updates();
    }

    fn reset_to_tjf_defaults(&mut self) {
        self.reset_to_defaults();
        self.order_panel.list = tjf_order();
        self.tags_panel.list.set(tjf_tags());
        self.deep.enabled = true;
        self.tags.enabled = true;
        self.dupes_db = false;
        self.ignore_extension = true;
        self.match_criteria.set(vec!("Filename".to_owned()));
    }

    fn clear_status(&mut self) {
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
        self.main.handle.is_some() ||
        self.basic.handle.is_some() ||
        self.deep.handle.is_some() ||
        self.tags.handle.is_some() ||
        self.compare.handle.is_some()
    }
    
    fn search_eligible(&self) -> bool {
        self.main.enabled || 
        self.basic.enabled || 
        self.deep.enabled || 
        self.tags.enabled || 
        self.compare.enabled 
    }

    fn receive_async_data(&mut self) {
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



        if let Ok(db) = self.db_io.rx.try_recv() {
            self.db = Some(db);
        }      
        if let Ok(db) = self.cdb_io.rx.try_recv() {
            self.compare_db = Some(db);
            self.compare.enabled = true;
        }    
        if let Some(db) = &mut self.db {
            if let Ok(records) = self.extensions_io.rx.try_recv() {
                db.file_extensions = records;
            }
        }
        if let Ok(count) = self.find_panel.find_io.rx.try_recv() {
            self.find_panel.count = count;
            self.find_panel.handle = None;
        }
        if let Some(records) = self.main.receive_hashset() {
            self.clear_status();
            self.main.status = format! {"Removed {} duplicates", records.len()}.into();
        }    
        if let Some(records) = self.basic.receive_hashset() {
            self.main.records.extend(records);
            self.update_main_status();
        }   
        if let Some(records) = self.deep.receive_hashset() {
            self.main.records.extend(records);
            self.update_main_status();
        }    
        if let Some(records) = self.tags.receive_hashset() {
            self.main.records.extend(records);
            self.update_main_status();
        }    
        if let Some(records) = self.compare.receive_hashset() {
            self.main.records.extend(records);
            self.update_main_status();
        }

        if let Ok(version) = self.latest_version_io.rx.try_recv() {
            self.latest_version = version;
            if self.update_window.is_some() {self.update_window = Some(true);}
            if self.latest_version == env!("CARGO_PKG_VERSION") 
            {
                println!("No Update Needed");
                self.update_available = false;
            }
            else {
                println!("Update Recommended");
                self.update_available = true;
            }
        }
    }

    fn update_main_status(&mut self) {
        // if self.handles_active() { return }

        if self.main.records.is_empty() {self.main.status = "No Records Marked for Removal".into()}
        else {
            self.main.status = format!(
                "{} total records marked for removal",
                self.main.records.len()
            ).into();

        }      
    }

    pub fn check_for_updates(&mut self) {
        let tx = self.latest_version_io.tx.clone();
        tokio::spawn(async move {
            println!("Inside Async Task - checking version");
    
            let results = fetch_latest_version().await;
            if (tx.send(results.expect("Tokio Results Error HashSet")).await).is_err() {
                eprintln!("Failed to send db");
            }
        });
    }
}



impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.registration.valid.is_none() {
            self.registration.validate();
            self.check_for_updates();
        }

        self.receive_async_data();

        // For inspiration and more examples, go to https://emilk.github.io/egui
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {

               self.file_menu(ui, ctx);

               ui.label(RichText::new("|").weak().size(18.0));

                self.panel_tab_bar(ui);

                if ui.available_width() > 20.0 {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        egui::widgets::global_theme_preference_switch(ui);
                        ui.label(RichText::new("|").weak().size(18.0));
                    });
                }

 
            });
        });
        // The central panel the region left after adding TopPanel's and SidePanel's
        egui::CentralPanel::default().show(ctx, |ui| {

            empty_line(ui);

            self.render_db(ui);

            empty_line(ui);
            ui.separator();
            empty_line(ui);

            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.my_panel {
                    Panel::Find => {
                        self.find_panel.render(ui, self.db.as_ref(), self.registration.valid);
                    }
                    Panel::Duplicates => {
                        self.duplictes_panel(ui);
                    }
                    Panel::NewDuplicates => {
                        // self.duplicates_panel.render(ui, self.db.as_ref(), self.registration.valid, &self.order_panel, &self.tags_panel.list);
                    }
                    Panel::Order => {
                        self.order_panel.render(ui, self.db.as_ref());
                    }
                    Panel::Tags => {
                        self.tags_panel.render(ui);
                    }
                    Panel::KeyGen => {
                        self.registration.render(ui);
                    }
                }
                empty_line(ui);
            });
            records_window(ctx, &self.marked_records, &mut self.records_window, &mut self.scroll_to_top);
         
            if self.update_window.is_some() {
                update_window(ctx, &mut self.update_window, &self.latest_version, self.update_available);
            }  
        });

        self.registration_bar(ctx);
        self.version_bar(ctx);

    }
    
}

impl App {
    fn file_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.menu_button(RichText::new("File").weak().size(18.0), |ui| {
            if ui.button("Open Database").clicked() {
                ui.close_menu();
                // self.clear_status();
                let tx = self.db_io.tx.clone();
                tokio::spawn(async move {
                    let db = open_db().await.unwrap();
                    let _ = tx.send(db).await;
                });

            }
            if ui.button("Close Database").clicked() {
                ui.close_menu();
                self.clear_status();
                self.abort_all();
                self.db = None;
            }

            ui.separator();
            if ui.button("Restore Defaults").clicked() {
                ui.close_menu();
                self.clear_status();
                self.reset_to_defaults();
            }
            if ui.input(|i| i.modifiers.alt) && ui.button("TJF Defaults").clicked() {
                ui.close_menu();
                self.clear_status();
                self.reset_to_tjf_defaults();
            }
            egui::widgets::global_theme_preference_buttons(ui);
            if !self.registration.valid.expect("some") {
                ui.separator();
                
           
                ui.menu_button("Register", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name: ");
                        ui.text_edit_singleline(&mut self.registration.name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Email: ");
                        ui.text_edit_singleline(&mut self.registration.email);
                    });
                    ui.horizontal(|ui| {
                        ui.label("License Key: ");
                        ui.text_edit_singleline(&mut self.registration.key);
                    });

                    large_button(ui, "Register", ||self.registration.validate());
                });
            }
            if ui.input(|i| i.modifiers.alt) && self.registration.valid == Some(true)  {

                ui.separator();
                if ui.button(RichText::new("Unregister")).clicked() {
                    self.registration.clear();
                    ui.close_menu();
                }
            }
            #[cfg(debug_assertions)]
            {
            
                if ui.button("KeyGen").clicked() {
                    ui.close_menu();
                    self.my_panel = Panel::KeyGen;
                }
            }
            ui.separator();
            if ui.button("Check For Update").clicked() {
                ui.close_menu();
                self.update_window = Some(false);
                self.check_for_updates();
            }
            if ui.button("Open Download URL").clicked() {
                ui.close_menu();
                open_download_url();
            }
            ui.separator();
            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        
    }

    fn panel_tab_bar(&mut self, ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            let mut space = ui.available_width() / 2.0 - 350.0;
            if space < 5.0 {
                space = 0.0;
            }
            ui.add_space(space);
    
            let size_big = 20.0;
            let size_small = 20.0;
            let column_width = 1.0;
    
            // Define a helper function to avoid repeating the logic for each panel
            fn add_tab_button(
                ui: &mut egui::Ui,
                current_panel: &mut Panel,
                panel: Panel,
                label: &str,
                size_big: f32,
                size_small: f32,
                column_width: f32,
            ) {
                let checked = *current_panel == panel;
                let text = if checked {
                    RichText::new(label).size(size_big).strong()
                } else {
                    RichText::new(label).size(size_small).weak()
                };
                ui.allocate_exact_size(egui::vec2(column_width, 20.0), egui::Sense::click());
                if ui.selectable_label(checked, text).clicked() {
                    *current_panel = panel;
                }
            }
    
            add_tab_button(ui, &mut self.my_panel, Panel::Find, "Find & Replace", size_big, size_small, column_width);
            // if ui.input(|i| i.modifiers.alt) {
            //     add_tab_button(ui, &mut self.my_panel, Panel::NewDuplicates, "Search for Duplicates2", size_big, size_small, column_width);
            // } else {
                add_tab_button(ui, &mut self.my_panel, Panel::Duplicates, "Search for Duplicates", size_big, size_small, column_width);
            // }
            add_tab_button(ui, &mut self.my_panel, Panel::Order, "Preservation Priority", size_big, size_small, column_width);
            add_tab_button(ui, &mut self.my_panel, Panel::Tags, "Tag Editor", size_big, size_small, column_width);
        });
    }

    fn render_db(&mut self, ui: &mut egui::Ui) {
        if let Some(db) = &self.db {
           

            ui.vertical_centered(|ui| {
                if ui
                    .selectable_label(
                        false,
                        RichText::new(&db.name)
                            .size(24.0)
                            .strong()
                            .extra_letter_spacing(5.0),
                    )
                    .clicked()
                {
                    
                    let tx = self.db_io.tx.clone();
                    tokio::spawn(async move {
                        let db = open_db().await.unwrap();
                        let _ = tx.send(db).await;
                    });
                };
                ui.label(format!("{} records", &db.size));
            });
        } else {
           
            ui.vertical_centered(|ui| {
                large_button(ui, "Open Database", || {
                    let tx = self.db_io.tx.clone();
                    tokio::spawn(async move {
                        let db = open_db().await.unwrap();
                        let _ = tx.send(db).await;
                    });
                });
            });
        }
    }

    fn registration_bar(&mut self, ctx: &egui::Context) {
        let id = egui::Id::new("bottom panel registration");
        egui::Area::new(id)
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(0.0, 0.0)) // Pin to bottom
            .show(ctx, |ui| {
                let mut label = red_text("*****UNREGISTERED");
                if let Some(valid) = self.registration.valid {
                    if valid {
                        let text = format!("Registered to: {}", &self.registration.name);
                        label = RichText::new(text).weak();
                    }
                }
                ui.horizontal(|ui| {
                    if ui.label(label).clicked()
                        // && ui.input(|i| i.modifiers.command)
                        // && ui.input(|i| i.modifiers.shift)
                        && ui.input(|i| i.key_down(egui::Key::Tab))
                        && ui.input(|i| i.key_down(egui::Key::R))
                        && ui.input(|i| i.key_down(egui::Key::Space))
                    {
                        self.my_panel = Panel::KeyGen;
                    };
                });
            });


    }

    fn version_bar(&mut self, ctx: &egui::Context) {
            
        let id = egui::Id::new("bottom panel");
        egui::Area::new(id)
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(0.0, 0.0)) // Pin to bottom-right
            .show(ctx, |ui| {
                let version_text = format!("Version: {}    ", env!("CARGO_PKG_VERSION"));
                ui.horizontal(|ui|{
                    if self.update_available {
                        ui.label(red_text("Update Available"));
                        if ui.selectable_label(false, "Download").clicked() {
                            open_download_url();
                        }
                    }
                    // ui.label("test");
                    ui.label(RichText::new(version_text).weak());

                });
            });
    }


    fn duplictes_panel(&mut self, ui: &mut egui::Ui) {

        
        // let w =    Widgets  {
        //         noninteractive: WidgetVisuals {
        //             weak_bg_fill: Color32::from_gray(27),
        //             bg_fill: Color32::from_gray(27),
        //             bg_stroke: egui::Stroke::new(1.0, Color32::from_gray(60)), // separators, indentation lines
        //             fg_stroke: egui::Stroke::new(1.0, Color32::from_gray(140)), // normal text color
        //             rounding: Rounding::same(2.0),
        //             expansion: 0.0,
        //         },
        //         inactive: WidgetVisuals {
        //             weak_bg_fill: Color32::from_gray(60), // button background
        //             bg_fill: Color32::from_gray(60),      // checkbox background
        //             bg_stroke: Default::default(),
        //             fg_stroke: egui::Stroke::new(1.0, Color32::from_gray(180)), // button text CHECK MARKS #1
        //             rounding: Rounding::same(2.0),
        //             expansion: 0.0,
        //         },
        //         hovered: WidgetVisuals {
        //             weak_bg_fill: Color32::from_gray(70),
        //             bg_fill: Color32::from_gray(70),
        //             bg_stroke: egui::Stroke::new(1.0, Color32::from_gray(150)), // e.g. hover over window edge or button
        //             fg_stroke: egui::Stroke::new(1.5, Color32::from_gray(240)), //CHECK MARKS #2
        //             rounding: Rounding::same(3.0),
        //             expansion: 1.0,
        //         },
        //         active: WidgetVisuals {
        //             weak_bg_fill: Color32::from_gray(55),
        //             bg_fill: Color32::from_gray(55),
        //             bg_stroke: egui::Stroke::new(1.0, Color32::WHITE),
        //             fg_stroke: egui::Stroke::new(2.0, Color32::WHITE),
        //             rounding: Rounding::same(2.0),
        //             expansion: 1.0,
        //         },
        //         open: WidgetVisuals {
        //             weak_bg_fill: Color32::from_gray(45),
        //             bg_fill: Color32::from_gray(27),
        //             bg_stroke: egui::Stroke::new(1.0, Color32::from_gray(60)),
        //             fg_stroke: egui::Stroke::new(1.0, Color32::from_gray(210)),
        //             rounding: Rounding::same(2.0),
        //             expansion: 0.0,
        //         },
        // };

        // ui.visuals_mut().widgets = w;
        // // ui.visuals_mut().widgets.active.bg_stroke = stroke;
        // // ui.visuals_mut().widgets.inactive = visuals;

        
        let Some(db) = &mut self.db else {
            ui.heading(RichText::new("No Open Database").weak());
            return;
        };
        if db.size == 0 {
            ui.heading("No Records in Database");
            return;
        }
        
        
        ui.columns(2, |column|{
            column[0].heading(RichText::new("Search for Duplicate Records").strong());
            //BASIC BASIC BASIC
            column[0].checkbox(&mut self.basic.enabled, "Basic Duplicate Search");
            column[0].horizontal(|ui| {
                ui.add_space(24.0);
                ui.label("Duplicate Match Criteria: ");

            });
            if self.match_criteria.get().is_empty() {
                self.basic.enabled = false;
                column[0].horizontal(|ui|{
                    ui.add_space(24.0);
                    ui.label(light_red_text("Add Match Criteria to Enable Search").size(14.0));
                });
                column[0].horizontal(|ui|{
                    ui.add_space(24.0);
                    button(ui, "Restore Defaults", ||{self.match_criteria.set(vec!{"Filename".to_owned(), "Duration".to_owned(), "Channels".to_owned()}) });
                });
                empty_line(&mut column[0]);
            } else {
                column[0].horizontal(|ui|{
                    ui.add_space(24.0);
                    self.match_criteria.render(ui, 3, "Match Criteria", true);
                });
            }
            column[0].horizontal(|ui|{
                ui.add_space(24.0);
                ui.label(RichText::new("Add:"));
                self.match_criteria.add_combo_box(ui, &db.columns);

                button(ui, "Remove Selected", ||{
                    self.match_criteria.remove_selected();

                });
            });

            if column[0].input(|i| i.modifiers.alt) {
                column[0].horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label("Unmatched Records: ");
                    ui.radio_value(&mut self.match_null, false, "Ignore");
                    ui.radio_value(&mut self.match_null, true, "Process Together");
                });
            } 
            
            column[1].heading(RichText::new("Remove Options").strong());
            let mut text = RichText::new("Create New Safety Database of Thinned Records");
            if !self.safe {text = text.strong().color(egui::Color32::from_rgb(255, 100, 100))}
            column[1].checkbox(&mut self.safe, text);
            if !&self.safe {
                column[1].horizontal(|ui| {
                        ui.label(
                            red_text("UNSAFE!")
                        );
                        ui.label(RichText::new("Will remove records from current database").strong());
                    });
                }
            column[1].checkbox(&mut self.dupes_db, "Create New Database of Duplicate Records");
            column[1].horizontal_wrapped(|ui| {
                let mut text = RichText::new("Remove Duplicate Files From Disk ");
                if self.remove_files {text = text.strong().size(14.0).color(egui::Color32::from_rgb(255, 100, 100))}
                ui.checkbox(&mut self.remove_files, text);
                
                if self.remove_files {
                    enum_combo_box(ui, &mut self.delete_action);
                    if self.remove_files && self.delete_action == Delete::Permanent {
                        ui.label(
                            RichText::new("UNSAFE!")
                            .color(egui::Color32::from_rgb(255, 0, 0))
                            .strong(),
                        );
                        ui.label(RichText::new("This is NOT undoable").strong());
                    }
                }
            });
        });
        self.basic.progress_bar(ui);


        //DEEP DIVE DEEP DIVE DEEP DIVE
        ui.checkbox(&mut self.deep.enabled, "Similar Filename Duplicates Search")
            .on_hover_text_at_pointer("Filenames ending in .#, .#.#.#, or .M will be examined as possible duplicates");

        if db.file_extensions.is_empty() && !self.extensions_io.waiting  {
            self.extensions_io.waiting  = true;
            db.get_extensions(self.extensions_io.tx.clone());
            
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Gathering Filetypes from DB");
                self.clear_status();
            });
        }
        else {
            ui.horizontal(|ui| {
                ui.add_space(24.0);

                if db.file_extensions.len() > 1 {
                    let text = if self.ignore_extension {"Checked: 'example.wav' and 'example.flac' will be considered duplicate filenames"}
                    else {"Unchecked: 'example.wav' and 'example.flac' will be considered unique filenames"};
                    ui.checkbox(&mut self.ignore_extension, "Ignore Filetypes").on_hover_text_at_pointer(text);

                } else {
                    ui.label("All Records are of Filetype:");
                    ui.label(&db.file_extensions[0]);
                }
            });
        }
        self.deep.progress_bar(ui);


        //TAGS TAGS TAGS TAGS
        let enabled = !self.tags_panel.list().is_empty();
        let text = enabled_text("Search for Records with AudioSuite Tags in Filename", &enabled);
        ui.checkbox(&mut self.tags.enabled,text,)
            .on_hover_text_at_pointer("Filenames with Common Protools AudioSuite Tags will be marked for removal");
       
        if enabled {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                if ui.button("Edit Tags List").clicked() {self.my_panel=Panel::Tags}
            });

        } else {
            self.tags.enabled = false;
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                if ui.button("Add Tags to Enable").clicked() {self.my_panel=Panel::Tags}
            });

        }
        self.tags.progress_bar(ui);
        

        //COMPARE COMPARE COMPARE COMPARE
        ui.horizontal(|ui| {
            let enabled = self.compare.enabled || self.compare_db.is_some();
            let text = enabled_text("Compare against database: ", &enabled);
            ui.checkbox(&mut self.compare.enabled, text)
                .on_hover_text_at_pointer
                    ("Filenames from Target Database found in Comparison Database will be Marked for Removal");
            
            if let Some(cdb) = &self.compare_db {
                if ui.selectable_label(false, &cdb.name).clicked() {
                    let tx = self.cdb_io.tx.clone();
                    tokio::spawn(async move {
                        let db = open_db().await.unwrap();
                        let _ = tx.send(db).await;
                    });
                }
            }
            else {
                self.compare.enabled = false;
                if ui.button("Select DB").clicked()  {
                    self.compare.enabled = false;
                    let tx = self.cdb_io.tx.clone();
                    tokio::spawn(async move {
                        let db = open_db().await.unwrap();
                        let _ = tx.send(db).await;
                    });
                }
            }
        });
        self.compare.progress_bar(ui);

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
                        rt_button(ui, light_red_text("Search and Remove Duplicates").size(20.0), || {
                            self.go_search = true;
                            self.go_remove = false;
                            self.gather_duplicates();
                        });
                    } else {
                        ui.columns(2, |column|{
                            column[0].horizontal(|ui| {
                                rt_button(ui, RichText::new("Search for Duplicates").size(20.0).strong(), || self.gather_duplicates());
                            });
                            if !self.handles_active() && !self.main.records.is_empty() {
                                column[1].horizontal(|ui|{
                                    rt_button(ui, light_red_text("Remove Duplicates").size(20.0).strong(), || self.remove_duplicates());
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
            let Some(pool) = db.pool() else {return};
    
          
            self.main.status = "Searching for Duplicates".into();
           
    
            if self.basic.enabled {
                let progress_sender = self.basic.progress_io.tx.clone(); 
                let status_sender = self.basic.status_io.tx.clone(); 
                let pool = pool.clone();
                let order = self.order_panel.extract_sql().clone();  
                let match_groups = self.match_criteria.get().to_vec();               
                let match_null = self.match_null;
                self.basic.wrap_async(
                    move || gather_duplicate_filenames_in_database(pool, progress_sender, status_sender, order, match_groups, match_null),
                )
            }
    
            if self.deep.enabled {
                let progress_sender = self.deep.progress_io.tx.clone(); 
                let status_sender = self.deep.status_io.tx.clone();
                let pool = pool.clone();
                let ignore = self.ignore_extension;
                self.deep.wrap_async(
                    move || gather_deep_dive_records(pool, progress_sender, status_sender, ignore),
                )
                    
                
            }
    
            if self.tags.enabled {
                let progress_sender = self.tags.progress_io.tx.clone();
                let status_sender = self.tags.status_io.tx.clone();
                let pool = pool.clone();
                let tags = self.tags_panel.list().to_vec();
                self.tags.wrap_async(
                    move || gather_filenames_with_tags(pool, progress_sender, status_sender, tags),
                );
            }
    
            if self.compare.enabled && self.compare_db.is_some() {
                if let Some(cdb) = &self.compare_db {
                    self.compare.working = true;
                    self.compare.status = format!("Comparing against {}", cdb.name).into();
        
                    let tx = self.compare.records_io.tx.clone();
                        let p = pool.clone();
                        let Some(c_pool) = cdb.pool() else {return;};
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

    
    
    
    fn remove_duplicates(&mut self) {
        if self.registration.valid == Some(false) {
            self.main.records.clear();
            self.main.status = "Unregistered!\nPlease Register to Remove Duplicates".into();
            return;
        }
        if let Some(db) = self.db.as_ref() {
            let mut work_db_path: Option<String> = Some(db.path.clone());
            let mut duplicate_db_path: Option<String> = None;
            let records = self.main.records.clone();
    
            self.main.working = true;
            if self.safe {
                self.main.status = "Creating Safety Database".into();
                let path = format!("{}_thinned.sqlite", &db.path.trim_end_matches(".sqlite"));
                let _result = fs::copy(&db.path, &path);
                work_db_path = Some(path);
            }
            if self.dupes_db {
                self.main.status = "Creating Database of Duplicates".into();
                let path = format!("{}_dupes.sqlite", &db.path.trim_end_matches(".sqlite"));
                let _result = fs::copy(&db.path, &path);
                duplicate_db_path = Some(path);
            }
    
            let progress_sender = self.main.progress_io.tx.clone();
            let status_sender = self.main.status_io.tx.clone(); 
            self.main.wrap_async(move || {
                remove_duplicates_go(records, work_db_path, duplicate_db_path, progress_sender, status_sender)
            });
                
            
            if self.remove_files {
                println!("Removing Files");
                let files: HashSet<&str> = self
                    .main
                    .records
                    .par_iter()
                    .map(|record| &*record.path)
                    .collect();
    
                let _ = self.delete_action.delete_files(files);
          
            }
        }
    }

}

pub async fn remove_duplicates_go(
    records: HashSet<FileRecord>,
    main_db_path: Option<String>,
    dupe_db_path: Option<String>,
    progress_sender: mpsc::Sender<ProgressMessage>,
    status_sender: mpsc::Sender<Arc<str>>,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    let _ = status_sender.send("Performing Record Removal".into()).await;
    if let Some(main_path) = &main_db_path {
        let main_db = Database::open(main_path).await;
        let Some(main_pool) = main_db.pool() else {return Err(sqlx::Error::PoolClosed);};
        let _result =
            delete_file_records(&main_pool, &records, progress_sender.clone(), status_sender.clone()).await;
        if let Some(path) = dupe_db_path {
            let dupes_db = Database::open(&path).await;
            let Some(dupes_pool) = dupes_db.pool() else {return Err(sqlx::Error::PoolClosed);};
            let _result =
                create_duplicates_db(&dupes_pool, &records, progress_sender.clone(), status_sender.clone())
                    .await;
        }
    }
    Ok(records)
}


#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct FindPanel {
    pub column: String,
    pub find: String,
    pub find_buf: String,
    pub replace: String,
    pub replace_buf: String,
    pub search_replace_path: bool,
    pub path_buf: bool,
    pub dirty: bool,
    pub case_sensitive: bool,
    #[serde(skip)]
    pub find_io: AsyncTunnel<usize>,
    #[serde(skip)]
    pub handle: Option<tokio::task::JoinHandle<()>>,

    #[serde(skip)]
    pub replace_safety: bool,
    #[serde(skip)]
    pub count: usize,
}

impl FindPanel {
    fn render(&mut self, ui: &mut egui::Ui, db: Option<&Database>, registration: Option<bool>) {
        if let Some(db) = db {
            if db.size == 0 {
                ui.heading("No Records in Database");
                return;
            }
            if self.column.is_empty() {
                self.column = String::from("Library");
                self.search_replace_path = true;
            }
            ui.heading(RichText::new("Find and Replace").strong());
        
            empty_line(ui);
            ui.horizontal(|ui| {
                let mut text = RichText::new("Case Sensitive").size(14.0);
                if self.case_sensitive {text = text.color(egui::Color32::from_rgb(255, 0, 0)).strong()}
                ui.checkbox(&mut self.case_sensitive, text);
            });
            empty_line(ui);
            ui.horizontal(|ui| {
                ui.label("Find Text: ");
                ui.text_edit_singleline(&mut self.find);
            });
            ui.horizontal(|ui| {
                ui.label("Replace: ");
                ui.add_space(8.0);
                ui.text_edit_singleline(&mut self.replace);
            });
            ui.horizontal(|ui| {
                ui.label("in Column: ");
                ui.radio_value(&mut self.search_replace_path, true, "FilePath");
                ui.radio_value(&mut self.search_replace_path, false, "Other");
                let filtered_columns: Vec<_> = db
                    .columns
                    .iter()
                    .filter(|col| {
                        col.as_str() != "FilePath"
                            && col.as_str() != "Pathname"
                            && col.as_str() != "Filename"
                    })
                    .collect();
                egui::ComboBox::from_id_salt("find_column")
                    .selected_text(&self.column)
                    .show_ui(ui, |ui| {
                        for item in filtered_columns {
                            ui.selectable_value(&mut self.column, item.clone(), item);
                        }
                    });
               
            });
            empty_line(ui);
            ui.separator();
            empty_line(ui);
            if !self.search_replace_path {
                ui.checkbox(&mut self.dirty, "Mark Records as Dirty?");
                ui.label("Dirty Records are audio files with metadata that is not embedded");
                empty_line(ui);
                ui.separator();
                empty_line(ui);
            }

            if self.find.is_empty() {
                return;
            }
            if ui
                .button(RichText::new("Find Records").size(16.0))
                .clicked()
            {
               
                self.replace_safety = true;

                let tx = self.find_io.tx.clone();
                let Some(pool) = db.pool() else {return};
                let mut find = self.find.clone();
                let mut column =  if self.search_replace_path {"FilePath".to_string()} else {self.column.clone()};
                let case_sensitive = self.case_sensitive;
                let handle = tokio::spawn(async move {
                    println!("Inside Find Async");
                    let count = smreplace_get(&pool, &mut find, &mut column, case_sensitive)
                        .await
                        .unwrap();
                    let _ = tx.send(count).await;
                });
                self.handle = Some(handle);
            }
            empty_line(ui);
           
            if self.find != self.find_buf || self.replace != self.replace_buf || self.search_replace_path != self.path_buf {
                self.replace_safety = false;
                self.find_buf = self.find.clone();
                self.replace_buf = self.replace.clone();
                self.path_buf = self.search_replace_path;
            }
            if self.replace_safety {
                if self.handle.is_some() {
                    ui.spinner();
                } else {
                    let column = if self.search_replace_path {"FilePath"} else {&self.column};
                    ui.label(
                        RichText::new(format!(
                            "Found {} records matching '{}' in {} of SM database: {}",
                            self.count, self.find, column, db.name
                        ))
                        .strong(),
                    );
                }
                if self.count == 0 {
                    return;
                }

                if registration == Some(false) {
                    ui.label(
                        RichText::new(
                            "\nUNREGISTERED!\nPlease Register to Continue with Replacement",
                        )
                        .strong(),
                    );
                    return;
                }
                ui.label(format!("Replace with \"{}\" ?", self.replace));
                ui.horizontal(|ui| {
                    ui.label("This is");
                    ui.label(RichText::new("NOT").strong());
                    ui.label("undoable");
                });
                if self.search_replace_path {
                    ui.label("This does not alter your file system.");
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new("Replace Records").size(16.0))
                        .clicked()
                    {
                        // let tx = self.find_tx.clone().expect("tx channel exists");
                        let Some(pool) = db.pool() else {return;};
                        let mut find = self.find.clone();
                        let mut replace = self.replace.clone();
                        let mut column = self.column.clone();
                        let dirty = self.dirty;
                        let filepath = self.search_replace_path;
                        let case_sensitive = self.case_sensitive;
                        tokio::spawn(async move {
                            smreplace_process(
                                &pool,
                                &mut find,
                                &mut replace,
                                &mut column,
                                dirty,
                                filepath,
                                case_sensitive,
                            )
                            .await;
                        });
                        self.replace_safety = false;
                    }
                    if ui.button(RichText::new("Cancel").size(16.0)).clicked() {
                        self.count = 0;
                        self.replace_safety = false;
                    }
                });
            } else if self.count > 0 && registration == Some(true) {
                ui.label(format!("{} records replaced", self.count));
            }
        } else {
            ui.heading(RichText::new("No Open Database").weak());
        }
    }
}


#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct OrderPanel {
    pub list: Vec<PreservationLogic>,
    #[serde(skip)]
    pub sel_line: Option<usize>,
    pub column: String,
    pub operator: OrderOperator,
    #[serde(skip)]
    pub input: String,
}


impl OrderPanel {
    pub fn render(&mut self, ui: &mut egui::Ui, db: Option<&Database>) {
        ui.heading(RichText::new("Duplicate Filename Preservation Priority").strong());
        ui.label("or... How to decide which file to keep when duplicates are found");
        ui.label("Entries at the top of list take precedence to those below");
        empty_line(ui);
        // ui.separator();
        if let Some(db) = db {
            self.top_toolbar(ui, &db.columns);
        }
        else {ui.label(light_red_text("Open DB to enable ADD NEW"));}
        empty_line(ui);
        ui.separator();

        ui.with_layout(
            egui::Layout::top_down_justified(egui::Align::Center),
            |ui| {
                // Determine the width available for the scrollable area
                let width = ui.available_width();
                let height = ui.available_height();

                // Create a vertical scrollable area with a specified width and height
                egui::ScrollArea::vertical()
                    .max_height(height - 55.0) // Set the maximum height of the scroll area
                    .show(ui, |ui| {
                        // Create a container with the desired width and height
                        ui.horizontal(|ui| {
                            ui.set_min_width(width);

                            // Create a grid layout within the scrollable area
                            egui::Grid::new("Order Grid")
                                .spacing([20.0, 8.0])
                                .striped(true)
                                .show(ui, |ui| {
                                 
                                    for (index, line) in self.list.iter_mut().enumerate()
                                    {
                                        let checked = self.sel_line == Some(index);
                                        let text: &str = if ui.input(|i| i.modifiers.alt) {&line.get_sql()} else {&line.get_friendly()};
                                        if ui
                                            .selectable_label(
                                                checked,
                                                RichText::new(format!(
                                                    "{:02} : {}",
                                                    index + 1,
                                                    text
                                                ))
                                                .size(14.0),
                                            )
                                            .clicked()
                                        {
                                            self.sel_line =
                                                if checked { None } else { Some(index) };
                                        }
                                        ui.end_row();
                                    }
                                });
                           
                            
                        });
                    });
                ui.separator();
                empty_line(ui);

                self.bottom_toolbar(ui);
            },
        );

    }

    pub fn top_toolbar(&mut self, ui: &mut egui::Ui, db_columns: &[String]) {
        ui.horizontal(|ui| {
            

            combo_box(ui, "order_column", &mut self.column, db_columns);
  
            enum_combo_box(ui, &mut self.operator);
            match self.operator {
                OrderOperator::Largest
                | OrderOperator::Smallest
                | OrderOperator::IsEmpty
                | OrderOperator::IsNotEmpty => {}
                _ => {
                    ui.text_edit_singleline(&mut self.input);
                }
            }
    
            if ui.button("Add Line").clicked {
                match self.operator {
                    OrderOperator::Largest
                    | OrderOperator::Smallest
                    | OrderOperator::IsEmpty
                    | OrderOperator::IsNotEmpty => {
        
                        self.list.insert(
                            0,
                            PreservationLogic {
                                column: self.column.clone(),
                                operator: self.operator,
                                variable: self.input.clone(),
                            },
                        );
                        self.input.clear();
                    }
                    _ => {
                        if !self.input.is_empty() {
    
                           self.list.insert(
                                0,
                                PreservationLogic {
                                    column: self.column.clone(),
                                    operator: self.operator,
                                    variable: self.input.clone(),
                                },
                            );
                            self.input.clear();
                        }
                    }
                }
            }
       
        });
    }

    pub fn bottom_toolbar(&mut self, ui: &mut egui::Ui) {
          ui.horizontal(|ui| {
        if ui.button("Move Up").clicked() {
            if let Some(index) = self.sel_line {
                if index > 0 {
                    self.sel_line = Some(index - 1);
  
                    self.list.swap(index, index - 1);
                }
            }
        }
        if ui.button("Move Down").clicked() {
            if let Some(index) = self.sel_line {
                if index < self.list.len() - 1 {
                    self.sel_line = Some(index + 1);

                    self.list.swap(index, index + 1);
                }
            }
        }
        if ui.button("Remove").clicked() {
            if let Some(index) = self.sel_line {
                self.list.remove(index);
                self.sel_line = None;
            }
        }


    });

    }

    pub fn extract_sql(&self) -> Vec<String> {
        self.list.iter().map(|logic| logic.get_sql()).collect()
    }
}



#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct TagsPanel {          // Use &str for new
    list: SelectableList,   // Use &str for the grid

}

impl TagsPanel {
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Tag Editor").strong());
        ui.label("Protools Audiosuite Tags use the following format:  -example_");
        ui.label("You can enter any string of text and if it is a match, the file will be marked for removal");
        empty_line(ui);
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            self.list.render(ui, 6, "tags editor", false);
            if !self.list().is_empty() {
                ui.separator();
            }
            empty_line(ui);
            self.list.add_text_input(ui);

            if !self.list.get().is_empty() && ui.button("Remove Selected Tags").clicked() {
                self.list.remove_selected();
            }
        });
    }
    pub fn list(&self) -> &[String] {
        self.list.get()
    }

}

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
#[serde(default)]
pub struct RegistrationPanel {
    pub name: String,
    pub email: String,
    pub key: String,
    #[serde(skip)]
    pub valid: Option<bool>,
}

impl RegistrationPanel {
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Name: ");
            ui.text_edit_singleline(&mut self.name);
        });
        ui.horizontal(|ui| {
            ui.label("Email: ");
            ui.text_edit_singleline(&mut self.email);
        });
        self.key = generate_license_key(&self.name, &self.email);
        ui.horizontal(|ui| {
            ui.label("License Key: ");
            ui.label(&self.key);
            // ui.text_edit_singleline(&mut self.registration.key);
        });
        
        ui.horizontal(|ui|{
            if ui.button("Register").clicked() {
                self.validate();
            }
            if ui.button("Copy to Clipboard").clicked() {
                copy_to_clipboard(format!("SMDB COMPANION\nDownload Link: https://drive.google.com/open?id=1qdGqoUMqq_xCrbA6IxUTYliZUmd3Tn3i&usp=drive_fs\n\nRegistration Info\nName: {}\nEmail: {}\nKey: {}\n\n", self.name, self.email, self.key));
        
            }
        });

    }
    pub fn validate(&mut self) {
        if generate_license_key(&self.name, &self.email) == self.key {
            self.valid = Some(true);
        } else {
            self.valid = Some(false);
        }
    }
    pub fn clear(&mut self) {
        self.name.clear();
        self.email.clear();
        self.key.clear();
        self.valid = Some(false);
    }
}







