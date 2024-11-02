use crate::prelude::*;



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
    duplicates_panel: Duplicates, 
    registration: RegistrationPanel,
}

impl Default for App {
    fn default() -> Self {
        Self {
            db: None,
            db_io: AsyncTunnel::new(1),
            extensions_io: AsyncTunnel::new(1),
            latest_version_io: AsyncTunnel::new(1),
            latest_version: String::new(),
            update_available: false,
            update_window: None,
            
            my_panel: Panel::Duplicates,
            find_panel: FindPanel::default(),
            duplicates_panel: Duplicates::default(),
            registration: RegistrationPanel::default(),
        }
    }
}


impl App {

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
                        self.duplicates_panel.render(ui, self.db.as_ref(), self.registration.valid);
                    }
                    Panel::Order => {
                        self.duplicates_panel.basic.preservation_order.render(ui, self.db.as_ref());
                    }
                    Panel::Tags => {
                        self.duplicates_panel.tags.render_panel(ui);
                    }
                    Panel::KeyGen => {
                        self.registration.render(ui);
                    }
                }
                empty_line(ui);
            });
            self.duplicates_panel.records_window.render(ctx);
         
            if self.update_window.is_some() {
                update_window(ctx, &mut self.update_window, &self.latest_version, self.update_available);
            }  
        });

        self.registration_bar(ctx);
        self.version_bar(ctx);

    }
    
}

impl App {
        /// Called once before the first frame.
        pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
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
            Default::default()
        }
    
        fn reset_to_defaults(&mut self)  {
            self.duplicates_panel = Duplicates::default();
            self.find_panel = FindPanel::default();
        }
        fn reset_to_tjf_defaults(&mut self) {
            self.reset_to_defaults();
            self.duplicates_panel.reset_to_tjf_defaults();
        }
    
        fn receive_async_data(&mut self) {
            if let Some(db) = self.db_io.recv() {
                self.db = Some(db);
            }      
    
            if let Some(db) = &mut self.db {
                if let Some(records) = self.extensions_io.recv() {
                    db.file_extensions = records;
                }
            }
            
            if let Some(version) = self.latest_version_io.recv() {
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
    fn file_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.menu_button(RichText::new("File").weak().size(18.0), |ui| {
            if ui.button("Open Database").clicked() {
                ui.close_menu();
                let tx = self.db_io.tx.clone();
                tokio::spawn(async move {
                    let db = Database::open().await.unwrap();
                    let _ = tx.send(db).await;
                });

            }
            if ui.button("Close Database").clicked() {
                ui.close_menu();
                self.duplicates_panel.clear_status();
                self.duplicates_panel.abort_all();
                self.db = None;
            }

            ui.separator();
            if ui.button("Restore Defaults").clicked() {
                ui.close_menu();
                // self.clear_status();
                self.reset_to_defaults();
            }
            if ui.input(|i| i.modifiers.alt) && ui.button("TJF Defaults").clicked() {
                ui.close_menu();
                // self.clear_status();
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
            add_tab_button(ui, &mut self.my_panel, Panel::Duplicates, "Search for Duplicates", size_big, size_small, column_width);
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
                        let db = Database::open().await.unwrap();
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
                        let db = Database::open().await.unwrap();
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


