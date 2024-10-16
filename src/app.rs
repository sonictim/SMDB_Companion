use crate::assets::*;
use crate::processing::*;
use crate::config::*;
// use crate::dupe_panel::*;
use eframe::egui::{self, RichText};
// use egui::accesskit::Node;
// use egui::Order;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs::{self};
// use std::thread::JoinHandle;
use tokio::sync::mpsc;






/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize,)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    #[serde(skip)]
    db_io: AsyncTunnel<Database>,
    #[serde(skip)]
    cdb_io: AsyncTunnel<Database>,
    #[serde(skip)]
    find_io: AsyncTunnel<usize>,
    // #[serde(skip)]
    // replace_io: AsyncTunnel<HashSet<FileRecord>>,
    #[serde(skip)]
    extensions_io: AsyncTunnel<Vec<String>>,
    #[serde(skip)]
    gathering_extensions: bool,
    #[serde(skip)]
    db: Option<Database>,
    #[serde(skip)]
    c_db: Option<Database>,

   

    main: NodeConfig,
    basic: NodeConfig,
    #[serde(skip)]
    sel_groups: Vec<usize>,
    group_null: bool,

    tags: NodeConfig,
    deep: NodeConfig,
    ignore_extension: bool,
    sel_extension: String,
    compare: NodeConfig,
   
    safe: bool,
    dupes_db: bool,
    remove_files: bool,
    delete_action: Delete,
    #[serde(skip)]
    my_panel: Panel,
    find_panel: FindPanel,
    // duplicates_panel: DupePanel,
    order_panel: OrderPanel,
    tags_panel: TagsPanel,
  
    #[serde(skip)]
    new_line: String,


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

    registered: Registration,
}



impl Default for App {
    fn default() -> Self {
      
        let mut app = Self {
            db_io: AsyncTunnel::new(1),
            cdb_io: AsyncTunnel::new(1),
            find_io: AsyncTunnel::new(1),
            // replace_io: AsyncTunnel::new(1),
            extensions_io: AsyncTunnel::new(1),
            gathering_extensions: false,
            db: None,
            c_db: None,
          

           
            main: NodeConfig::new(false),
            basic: NodeConfig::new(true),
            sel_groups: Vec::new(),
            group_null: false,

            tags: NodeConfig::new_option(false, "-"),
            deep: NodeConfig::new(false),
            ignore_extension: false,
            sel_extension: String::new(),
            compare: NodeConfig::new(false),
           
            safe: true,
            dupes_db: true,
            remove_files: false,
            delete_action: Delete::Trash,
            my_panel: Panel::Duplicates,
            find_panel: FindPanel::default(),
            // duplicates_panel: DupePanel::default(),
            tags_panel: TagsPanel::default(),
            order_panel: OrderPanel::default(),

            new_line: String::new(),
 

            gather_dupes: false,
            go_search: false,
            go_remove: false,

            marked_records: String::new(),
            records_window: false,
            scroll_to_top: false,

            registered: Registration::default(),
        };
        app.basic.list = vec!["Filename".to_owned(), "Duration".to_owned(), "Channels".to_owned()];
       
        app.tags_panel.list = default_tags();
 
        app.order_panel.list = get_default_struct_order();

        app
    }
}
impl App {
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
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn reset_to_defaults(&mut self)  {

        let db = self.db.take();
        let panel = self.my_panel;
        let registration = self.registered.clone();
        *self = Self::default();
        self.db = db;
        self.my_panel = panel;
        self.registered = registration;
        // self.duplicates_panel.match_criteria = vec!["Filename".to_owned(), "Duration".to_owned(), "Channels".to_owned()];
        // self.duplicates_panel.basic.enabled = true;
        // self.duplicates_panel.remove.safe = true;
        // self.duplicates_panel.remove.create_dupe_db = true;
       
    }

    fn reset_to_tjf_defaults(
        &mut self,
    ) {
    
        self.reset_to_defaults();
      
        self.order_panel.list = get_tjf_struct_order();
        self.tags_panel.list = tjf_tags();
        self.deep.enabled = true;
        self.tags.enabled = true;
        self.dupes_db = true;
        self.ignore_extension = true;
        self.basic.list = vec!("Filename".to_owned());

        
        // self.duplicates_panel.match_criteria = vec!["Filename".to_owned()];
        // self.duplicates_panel.deep.enabled = true;
        // self.duplicates_panel.tags.enabled = true;
        // self.duplicates_panel.ignore_filetypes = true;
  
    }

}



impl eframe::App for App {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.registered.valid.is_none() {
            self.registered.validate();
        }

        self.receive_async_data();

        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!

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
                    if !self.registered.valid.expect("some") {
                        ui.separator();
                        
                   
                        ui.menu_button("Register", |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Name: ");
                                ui.text_edit_singleline(&mut self.registered.name);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Email: ");
                                ui.text_edit_singleline(&mut self.registered.email);
                            });
                            ui.horizontal(|ui| {
                                ui.label("License Key: ");
                                ui.text_edit_singleline(&mut self.registered.key);
                            });

                            large_button(ui, "Register", ||self.registered.validate());
                        });
                    }
                    if ui.input(|i| i.modifiers.alt) && self.registered.valid == Some(true)  {

                        ui.separator();
                        if ui.button(RichText::new("Unregister")).clicked() {
                            self.registered.clear();
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
                    if ui.button("Open Download URL").clicked() {
                        ui.close_menu();
                        let url = r#"https://drive.google.com/open?id=1qdGqoUMqq_xCrbA6IxUTYliZUmd3Tn3i&usp=drive_fs"#;
                        let _ = webbrowser::open(url).is_ok();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
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


            
            if let Some(db) = &self.db {
                empty_line(ui);

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
                empty_line(ui);
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

   

            empty_line(ui);
            ui.separator();
            empty_line(ui);

            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.my_panel {
                    Panel::Find => {
                        // self.find_panel(ui);
                        self.find_panel.render(ui, &self.db, self.registered.valid);
                    }

                    Panel::Duplicates => {
                        self.duplictes_panel(ui);
                        
                    }

                    Panel::NewDuplicates => {
                        // self.duplicates_panel.render(ui, self.db.as_ref(), self.registered.valid, &self.order_panel, &self.tags_panel.list);
                    }

                    Panel::Order => {
                        self.order_panel.render(ui, self.db.as_ref());
                    }

                    Panel::Tags => {
                        self.tags_panel.render(ui);
                    }

                    Panel::KeyGen => {
                        self.registered.render(ui);
                    }
                }
                empty_line(ui);
            });

            if self.records_window {
                records_window(ctx, &self.marked_records, &mut self.records_window, &mut self.scroll_to_top);

            }
        });

        let id2 = egui::Id::new("bottom panel registration");
        egui::Area::new(id2)
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(0.0, 0.0)) // Pin to bottom
            // .default_width(500.0)
            .show(ctx, |ui| {
                let mut label = red_text("*****UNREGISTERED");
                if let Some(valid) = self.registered.valid {
                    if valid {
                        let text = format!("Registered to: {}", &self.registered.name);
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
                    // && ui.input(|i| i.key_pressed(egui::Key::Tab))
                    {
                        self.my_panel = Panel::KeyGen;
                    };
                });
            });
        let id = egui::Id::new("bottom panel");
        egui::Area::new(id)
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(0.0, 0.0)) // Pin to bottom-right
            .show(ctx, |ui| {
                let version_text = format!("Version: {}    ", env!("CARGO_PKG_VERSION"));
                ui.label(RichText::new(version_text).weak());
                // ui.label("This is the bottom panel.");
            });
    }
    
}

impl App {
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


    fn duplictes_panel(&mut self, ui: &mut egui::Ui) {
        if let Some(db) = &mut self.db {
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
                if self.basic.list.is_empty() {
                    self.basic.enabled = false;

                    column[0].horizontal(|ui|{
                        ui.add_space(24.0);
    
                        ui.label(light_red_text("Add Match Criteria to Enable Search").size(14.0));

                    });
                    column[0].horizontal(|ui|{
                        ui.add_space(24.0);
                        
                        
                        button(ui, "Restore Defaults", ||{self.basic.list = vec!{"Filename".to_owned(), "Duration".to_owned(), "Channels".to_owned()} });
                        
                        
                        
                    });
                    empty_line(&mut column[0]);
                
                } else {
    
                    
                    column[0].horizontal(|ui|{
                        ui.add_space(24.0);
                    
                        egui::Frame::none() // Use Frame to create a custom bordered area
                        .inner_margin(egui::vec2(8.0, 8.0)) // Inner margin for padding
                        .show(ui, |ui| {
                            ui.group(|ui| {
                             
                              
                                ui.horizontal(|ui| {
                                    // Drawing a border manually
                                    ui.add_space(2.0);
                                    selectable_grid(ui, "Match Grid", 4, &mut self.sel_groups, &mut self.basic.list);
                                   
                                    ui.add_space(2.0);
                                });
                            });
                        });
        
        
        
        
                    });
                }
                column[0].horizontal(|ui|{
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
                
                    button(ui, "Remove Selected", ||{
                        let mut sorted_indices: Vec<usize> = self.sel_groups.clone();
                        sorted_indices.sort_by(|a, b| b.cmp(a)); // Sort in reverse order
    
                        for index in sorted_indices {
                            if index < self.basic.list.len() {
                                self.basic.list.remove(index);
                            }
                        }
                        self.sel_groups.clear();
                        self.basic.selected.clear();
    
                
                    });
                });

                if column[0].input(|i| i.modifiers.alt) {
                   
                    column[0].horizontal(|ui| {
                        ui.add_space(24.0);
                        ui.label("Unmatched Records: ");
                        ui.radio_value(&mut self.group_null, false, "Ignore");
                        ui.radio_value(&mut self.group_null, true, "Process Together");
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
            ui.checkbox(&mut self.deep.enabled, "Similar Filename Duplicates Search").on_hover_text_at_pointer("Filenames ending in .#, .#.#.#, or .M will be examined as possible duplicates");

            
            if db.file_extensions.is_empty() && !self.gathering_extensions {
                self.gathering_extensions = true;
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
    
                    if self.sel_extension.is_empty() {
                        self.sel_extension = db.file_extensions[0].clone();
                    }
                    if db.file_extensions.len() > 1 {
                        let text = if self.ignore_extension {"Checked: 'example.wav' and 'example.flac' will be considered duplicate filenames"}
                        else {"Unchecked: 'example.wav' and 'example.flac' will be considered unique filenames"};
                        ui.checkbox(&mut self.ignore_extension, "Ignore Filetypes").on_hover_text_at_pointer(text);
    
                    } else {
                        ui.label("All Records are of Filetype:");
                        ui.label(&self.sel_extension);
                    }
                });
            }
            self.deep.progress_bar(ui);

          


            //TAGS TAGS TAGS TAGS
            ui.checkbox(
                &mut self.tags.enabled,
                "Search for Records with AudioSuite Tags in Filename",
            ).on_hover_text_at_pointer("Filenames with Common Protools AudioSuite Tags will be marked for removal");

            ui.horizontal(|ui| {
                ui.add_space(44.0);
                if ui.button("Edit Tags").clicked() {self.my_panel=Panel::Tags}
            });

            self.tags.progress_bar(ui);
         

 

            //COMPARE COMPARE COMPARE COMPARE
            ui.horizontal(|ui| {
                let enabled = self.compare.enabled || self.c_db.is_some();
                let text = enabled_text("Compare against database: ", &enabled);
                ui.checkbox(&mut self.compare.enabled, text)
                    .on_hover_text_at_pointer
                        ("Filenames from Target Database found in Comparison Database will be Marked for Removal");
                
                if let Some(cdb) = &self.c_db {
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
            // if !self.compare.enabled {
            //     ui.label(RichText::new("Select DB to enable").weak());
            //     ui.horizontal(|_ui| {});
            // } else {
            //     self.compare.progress_bar(ui);
            // };
            
            self.compare.progress_bar(ui);
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
                                self.gather_duplicates();
                            });
                        } else {
                            ui.columns(2, |column|{
                                column[0].horizontal(|ui| {

                                    rt_button(ui, RichText::new("Search for Duplicates").size(20.0).strong(), || self.gather_duplicates());
                                });
    
                                if !self.main.records.is_empty() && !self.handles_active() {
                                    self.main.status = format!(
                                        "{} total records marked for removal",
                                        self.main.records.len()
                                    );
                                    column[1].horizontal(|ui|{
                                        rt_button(ui, RichText::new("Remove Duplicates").size(20.0).strong(), || self.remove_duplicates());
                                    

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
                ui.label(RichText::new(self.main.status.clone()).strong());
            });

            if self.registered.valid == Some(true)
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
            
        } else {
            ui.heading(RichText::new("No Open Database").weak());
        }
    }
    pub fn gather_duplicates(&mut self) {
        self.abort_all();
        self.main.records.clear();
        if let Some(db) = self.db.as_ref() {
            let Some(pool) = db.pool.clone() else {return};
    
          
            self.main.status = "Searching for Duplicates".to_string();
           
    
            if self.basic.enabled {
                let sender = self.basic.progress_io.tx.clone(); 
                let pool = pool.clone();
                let order = self.order_panel.extract_sql().clone();  
                let groups = self.basic.list.clone();               
                let group_null = self.group_null;
    
                wrap_async(
                    &mut self.basic,
                    "Searching For Duplicate Records",
                    move || gather_duplicate_filenames_in_database(pool, order, groups, group_null, sender),
                )
                
            }
    
            if self.deep.enabled {
                let progress_sender = self.deep.progress_io.tx.clone(); 
                    let status_sender = self.deep.status_io.tx.clone();
                        let pool = pool.clone();
                        let ignore = self.ignore_extension;
                        wrap_async(
                            &mut self.deep,
                            "Searching for Duplicates with similar Filenames",
                            move || gather_deep_dive_records(pool, progress_sender, status_sender, ignore),
                        )
                    
                
            }
    
            if self.tags.enabled {
                let progress_sender = self.tags.progress_io.tx.clone();
    
                    let pool = pool.clone();
                    let tags = self.tags_panel.list.clone();
                    wrap_async(
                        &mut self.tags,
                        "Searching for Filenames with Specified Tags",
                        move || gather_filenames_with_tags(pool, tags, progress_sender),
                    );
                
            }
    
            if self.compare.enabled && self.c_db.is_some() {
                if let Some(cdb) = &self.c_db {
                    self.compare.working = true;
                    self.compare.status = format!("Comparing against {}", cdb.name);
        
                    let tx = self.compare.records_io.tx.clone();
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
    
    
    
    fn remove_duplicates(&mut self) {
        if self.registered.valid == Some(false) {
            self.main.records.clear();
            self.main.status = "Unregistered!\nPlease Register to Remove Duplicates".to_string();
            return;
        }
        if let Some(db) = self.db.as_ref() {
            let mut work_db_path: Option<String> = Some(db.path.clone());
            let mut duplicate_db_path: Option<String> = None;
            let records = self.main.records.clone();
    
            self.main.working = true;
            if self.safe {
                self.main.status = "Creating Safety Database".to_string();
                let path = format!("{}_thinned.sqlite", &db.path.trim_end_matches(".sqlite"));
                let _result = fs::copy(&db.path, &path);
                work_db_path = Some(path);
            }
            if self.dupes_db {
                self.main.status = "Creating Database of Duplicates".to_string();
                let path = format!("{}_dupes.sqlite", &db.path.trim_end_matches(".sqlite"));
                let _result = fs::copy(&db.path, &path);
                duplicate_db_path = Some(path);
            }
    
            let progress_sender = self.main.progress_io.tx.clone();
            let status_sender = self.main.status_io.tx.clone(); 
            wrap_async(&mut self.main, "Performing Record Removal", move || {
                remove_duplicates_go(records, work_db_path, duplicate_db_path, progress_sender, status_sender)
            });
                
            
            if self.remove_files {
                println!("Removing Files");
                let files: HashSet<&str> = self
                    .main
                    .records
                    .par_iter()
                    .map(|record| record.path.as_str())
                    .collect();
    
                let _ = self.delete_action.delete_files(files);
          
            }
        }
    }






    fn receive_async_data(&mut self) {

    
        if let Ok(db) = self.db_io.rx.try_recv() {
            self.db = Some(db);
        }
    
        
        if let Ok(db) = self.cdb_io.rx.try_recv() {
            self.c_db = Some(db);
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
            self.main.status = format! {"Removed {} duplicates", records.len()};
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
    

}

pub async fn remove_duplicates_go(
    records: HashSet<FileRecord>,
    main_db_path: Option<String>,
    dupe_db_path: Option<String>,
    progress_sender: mpsc::Sender<ProgressMessage>,
    status_sender: mpsc::Sender<String>,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    if let Some(main_path) = &main_db_path {
        let main_db = Database::open(main_path).await;
        let Some(main_pool) = main_db.pool.clone() else {return Err(sqlx::Error::PoolClosed);};
        let _result =
            delete_file_records(&main_pool, &records, progress_sender.clone(), status_sender.clone()).await;
        if let Some(path) = dupe_db_path {
            let dupes_db = Database::open(&path).await;
            let Some(dupes_pool) = dupes_db.pool.clone() else {return Err(sqlx::Error::PoolClosed);};
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
    fn render(&mut self, ui: &mut egui::Ui, db: &Option<Database>, registered: Option<bool>) {
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
                // ui.add_space(68.0);
                let mut text = RichText::new("Case Sensitive").size(14.0);
                if self.case_sensitive {text = text.color(egui::Color32::from_rgb(255, 0, 0)).strong()}
                ui.checkbox(&mut self.case_sensitive, text);
            });
            empty_line(ui);
            // ui.separator();
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
                let Some(pool) = db.pool.clone() else {return};
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

            // if let Ok(count) = self.find_io.rx.try_recv() {
            //     self.count = count;
            // }

           
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

                if registered == Some(false) {
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
                        let Some(pool) = db.pool.clone() else {return;};
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
            } else if self.count > 0 && registered == Some(true) {
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
    #[serde(skip)]
    pub text: String,
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
                                        let text: &str = if ui.input(|i| i.modifiers.alt) {&line.sql} else {&line.friendly};
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
                            if ui.input(|i| i.modifiers.alt) {      
                                
                            }
                            
                        });
                    });
                ui.separator();
                empty_line(ui);

                self.bottom_toolbar(ui);
            },
        );

    }

    pub fn top_toolbar(&mut self, ui: &mut egui::Ui, db_columns: &Vec<String>) {
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
                            parse_to_struct(
                                self.column.clone(),
                                self.operator,
                                self.input.clone(),
                            ),
                        );
                        self.input.clear();
                    }
                    _ => {
                        if !self.input.is_empty() {
    
                           self.list.insert(
                                0,
                                parse_to_struct(
                                    self.column.clone(),
                                    self.operator,
                                    self.input.clone(),
                                ),
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
        self.list.iter().map(|logic| logic.sql.clone()).collect()
    }
}






#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct TagsPanel {
    pub list: Vec<String>,
    new: String,
    selected: Vec<usize>,
}

impl TagsPanel {
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Tag Editor").strong());
        ui.label("Protools Audiosuite Tags use the following format:  -example_");
        ui.label("You can enter any string of text and if it is a match, the file will be marked for removal");
        empty_line(ui);
        ui.separator();
        egui::ScrollArea::vertical().show(ui, |ui| {
            selectable_grid(ui, "Tags Grid", 6, &mut self.selected, &mut self.list);
            ui.separator();
            empty_line(ui);
            ui.horizontal(|ui| {
                if ui.button("Add Tag:").clicked() {
                    self.add_tag();
                }
                ui.text_edit_singleline(&mut self.new);
            });
            if ui.button("Remove Selected Tags").clicked() {
                self.remove_selected_tags();
            }
        });
    }
    fn add_tag(&mut self) {
        if self.new.is_empty() {
            return;
        }
        self.list.push(self.new.clone());
        self.new.clear(); // Clears the string
        self.list.sort_by_key(|s| s.to_lowercase());
    }

    fn remove_selected_tags(&mut self) {
        let mut sorted_indices: Vec<usize> = self.selected.clone();
        sorted_indices.sort_by(|a, b| b.cmp(a)); // Sort in reverse order

        for index in sorted_indices {
            if index < self.list.len() {
                self.list.remove(index);
            }
        }
        self.selected.clear();
    }
}


impl Registration {

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
            // ui.text_edit_singleline(&mut self.registered.key);
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
}







