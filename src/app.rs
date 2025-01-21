use crate::prelude::*;

use clipboard::{ClipboardContext, ClipboardProvider};
use futures::io::empty;
use reqwest::Client;
// use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
// use std::error::Error;

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy)]
pub enum Panel {
    Duplicates,
    Order,
    Tags,
    Find,
    KeyGen,
}


/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {

    #[serde(skip)]
    db: AsyncTunnel<Option<Database>>,

    #[serde(skip)]
    my_panel: Panel,
    find_replace: FindPanel,
    duplicates: Duplicates, 
    registration: Registration,
    #[serde(skip)]
    update: Update,
}

impl Default for App {
    fn default() -> Self {
        Self {
            // db: None,
            db: AsyncTunnel::new(1),
            // extensions_io: AsyncTunnel::new(1),
         
            my_panel: Panel::Duplicates,
            find_replace: FindPanel::default(),
            duplicates: Duplicates::default(),
            registration: Registration::default(),
            update: Update::default(),
        }
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
            self.update.check();
        }

        self.receive_async_data();
        if self.duplicates.tags_panel() {self.my_panel = Panel::Tags}

        // For inspiration and more examples, go to https://emilk.github.io/egui
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {

               self.file_menu(ui, ctx);
               self.pref_menu(ui);
               self.view_menu(ui);
               self.help_menu(ui);

            //    ui.label(RichText::new("|").weak().size(18.0));

                // self.panel_tab_bar(ui);

                if ui.available_width() > 20.0 {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        egui::widgets::global_theme_preference_switch(ui);
                        // ui.label(RichText::new("|").weak().size(18.0));
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
                        self.find_replace.render(ui, self.db.get().as_ref(), self.registration.valid);
                    }
                    Panel::Duplicates => {
                        self.duplicates.render(ui, self.db.get().as_ref(), self.registration.valid);
                    }
                    Panel::Order => {
                        self.duplicates.render_order_panel(ui, self.db.get().as_ref());
                    }
                    Panel::Tags => {
                        self.duplicates.render_tags_panel(ui);
                    }
                    Panel::KeyGen => {
                        self.registration.render(ui);
                    }
                }
                empty_line(ui);
            });
            self.duplicates.records_window.render(ctx);
         
            if self.update.window.is_some() {
                self.update.render(ctx);
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
            self.duplicates = Duplicates::default();
            self.find_replace = FindPanel::default();
        }
        fn reset_to_tjf_defaults(&mut self) {
            self.reset_to_defaults();
            self.duplicates.reset_to_tjf_defaults();
        }
    
        fn receive_async_data(&mut self) {

            if let Some(db) = self.db.recv() {
                self.db.set(db);
            }    
            
            if let Some(db) = self.db.get_mut() {
                db.extensions.recv2();
            }
            
            use semver::Version;

            if let Some(version) = self.update.latest_version.recv() {
                self.update.latest_version.set(version.clone());
                if self.update.window.is_some() {
                    self.update.window = Some(true);
                }

                let current_version = Version::parse(env!("CARGO_PKG_VERSION")).expect("Invalid current version");
                let latest_version = Version::parse(&version).expect("Invalid latest version");

                if latest_version <= current_version {
                    println!("No Update Needed");
                    self.update.available = false;
                } else {
                    println!("Update Recommended");
                    self.update.available = true;
                }
            }
          
    

        }
    
    
    

    fn file_menu(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.menu_button(menu_text("File"), |ui| {
            if ui.button("Open Database").clicked() {
                ui.close_menu();
                let tx = self.db.tx.clone();
                tokio::spawn(async move {
                    let db = Database::open().await;
                    let _ = tx.send(db).await;
                });

            }
            if ui.button("Close Database").clicked() {
                ui.close_menu();
                self.duplicates.clear_status();
                self.duplicates.abort_all();
                self.db.set(None);
            }

            ui.separator();
            if !self.registration.valid.expect("some") {
                empty_line(ui);
                if ui.button("Purchase License").clicked() {
                    ui.close_menu();
                    open_purchase_url();
                }
                empty_line(ui);
                ui.separator();
           
                ui.menu_button("Register", |ui| {
                    // large_button2(ui, "Purchase License", open_purchase_url);

                    // // if ui.button(RichText::new("Purchase License").strong()).clicked() {
                    // //     open_purchase_url();
                    // // }
                    // ui.label(RichText::new("Registration Info").strong());
                    empty_line(ui);
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
                ui.separator();
            }
            if ui.input(|i| i.modifiers.alt) && self.registration.valid == Some(true)  {

                if ui.button(RichText::new("Unregister")).clicked() {
                    self.registration.clear();
                    ui.close_menu();
                }
                ui.separator();
            }
            #[cfg(debug_assertions)]
            {
            
                if ui.button("KeyGen").clicked() {
                    ui.close_menu();
                    self.my_panel = Panel::KeyGen;
                }
                ui.separator();
            }
            
            if ui.button("Quit").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        
    }

    fn pref_menu(&mut self, ui: &mut egui::Ui,) {
        ui.menu_button(menu_text("Preferences"), |ui| {
            if ui.button("Filename Preservation Priority").clicked() {
                ui.close_menu();
                self.my_panel = Panel::Order;
            }
            if ui.button("Audiosuite Tags Editor").clicked() {
                ui.close_menu();
                self.my_panel = Panel::Tags;
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
            ui.separator();
            egui::widgets::global_theme_preference_buttons(ui);
            
            
        });

    }
    fn view_menu(&mut self, ui: &mut egui::Ui,) {
        ui.menu_button(menu_text("Action"), |ui| {
            if ui.button("Search for Duplicates").clicked() {
                ui.close_menu();
                self.my_panel = Panel::Duplicates;
            }
            if ui.button("Find and Replace").clicked() {
                ui.close_menu();
                self.my_panel = Panel::Find;
            }
            
            
        });

    }

    fn help_menu(&mut self, ui: &mut egui::Ui,) {
        ui.menu_button(menu_text("Help"), |ui| {
            if ui.button("Check For Update").clicked() {
                ui.close_menu();
                self.update.window = Some(false);
                self.update.check();
            }
            if ui.button("Open Website").clicked() {
                ui.close_menu();
                open_website_url();
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
        if let Some(db) = &self.db.get() {
           

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
                    
                    let tx = self.db.tx.clone();
                    tokio::spawn(async move {
                        let db = Database::open().await;
                        let _ = tx.send(db).await;
                    });
                };
                // self.duplicates.new_db();
                ui.label(format!("{} records", &db.size));
            });
        } else {
           
            ui.vertical_centered(|ui| {
                large_button(ui, "Open Database", || {
                    // self.duplicates.new_db();
                    let tx = self.db.tx.clone();
                    tokio::spawn(async move {
                        let db = Database::open().await;
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
                let mut label = red_text("*****UNREGISTERED*****");
                if let Some(valid) = self.registration.valid {
                    if valid {
                        let text = format!("Registered to: {}", &self.registration.name);
                        label = RichText::new(text).weak();
                    }
                }
                ui.horizontal(|ui| {

                    
                    if ui.label(label).clicked() && ui.input(|i| i.key_down(egui::Key::Tab))
                    && ui.input(|i| i.key_down(egui::Key::R)) && ui.input(|i| i.key_down(egui::Key::Space)) {
                            #[cfg(debug_assertions)] {
                                
                                self.my_panel = Panel::KeyGen;
                            }
                    };
                    if let Some(valid) = self.registration.valid {
                        if !valid && ui.selectable_label(false, "Purchase License").clicked() {
                            open_purchase_url();
                        }
                    }
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
                    if self.update.available {
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
pub struct Registration {
    pub name: String,
    pub email: String,
    pub key: String,
    #[serde(skip)]
    pub valid: Option<bool>,
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
        self.key = self.generate_license_key(&self.name, &self.email);
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
        if self.generate_license_key(&self.name, &self.email) == self.key {
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
    fn generate_license_key(&self, username: &str, email: &str) -> String {
        let salt = "Valhalla Delay";
        let mut hasher = Sha256::new();
        hasher.update(format!("{}{}{}", username, email, salt).as_bytes());
        let hash = hasher.finalize();
        hex::encode_upper(hash)
    }
}


pub fn copy_to_clipboard(text: String) {
    let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();

    ctx.set_contents(text).unwrap();
}


pub fn open_download_url() {
    let url = r#"https://smdbc.com/download.php?token=please-can-i-have-it"#;
    let _ = webbrowser::open(url).is_ok();
}
pub fn open_website_url() {
    let url = r#"https://smdbc.com/"#;
    let _ = webbrowser::open(url).is_ok();
}
pub fn open_purchase_url() {
    let url = r#"https://buy.stripe.com/9AQcPw4D0dFycSYaEE"#;
    let _ = webbrowser::open(url).is_ok();
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
struct Update {
    #[serde(skip)]
    latest_version: AsyncTunnel<String>,
    #[serde(skip)]
    available: bool,
    #[serde(skip)]
    window: Option<bool>,

}

impl Update {
    pub fn render(&mut self, ctx: &egui::Context) {
        
        let width = 200.0;
        let height = 100.0;
        let mut close_window = false;

        if let Some(open) = &mut self.window {
            if self.available {
                egui::Window::new(RichText::new("Update Available").strong())
                    .open(open) // Control whether the window is open
                    .resizable(false) // Make window non-resizable if you want it fixed
                    .min_width(width)
                    .min_height(height)
                    .max_width(width)
                    .max_height(height)
                    .show(ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(format!("Latest Version is {}", self.latest_version.get()));
                            large_button(ui, "Download", || {
                                open_download_url();
                                close_window = true; // Set the flag to close the window
                            });
                        });
                    });
            } else {
                egui::Window::new(RichText::new("No Update Available").strong())
                    .open(open) // Control whether the window is open
                    .resizable(false) // Make window non-resizable if you want it fixed
                    .min_width(width)
                    .min_height(height)
                    .max_width(width)
                    .max_height(height)
                    .show(ctx, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(format!("Version {} is the Current Version", self.latest_version.get()));
                            // large_button(ui, "Download", || {
                            //     open_download_url();
                            //     close_window = true; // Set the flag to close the window
                            // });
                        });
                    });
            }
        }

        if close_window {
            self.window = None; // Dereference to set the value outside of the closure
        }
    }

    pub fn check(&mut self) {
        let tx = self.latest_version.tx.clone();
        tokio::spawn(async move {
            println!("Inside Async Task - checking version");
    
            let results = fetch_latest_version().await;
            if (tx.send(results.expect("Tokio Results Error HashSet")).await).is_err() {
                eprintln!("Failed to send db");
            }
        });
    }

}

pub async fn fetch_latest_version() -> Result<String> {
    let url = "https://smdbc.com/latest.php";
    let token = "how-cool-am-i";

    // Send GET request with token
    let client = Client::new();
    let response = client
        .get(url)
        .query(&[("token", token)]) // Add token as query parameter
        .send().await?;


    Ok(response.text().await?.trim().to_string())

    // if response.status().is_success() {
    //     let latest_version = response.text().await?.trim().to_string();
    //     return Ok(latest_version);
    // } else {
    //     eprintln!(
    //         "Failed to fetch the latest version. Status: {}",
    //         response.status()
    //     );
    // }

}
// pub async fn fetch_latest_version() -> Result<String, Box<dyn Error>> {
//     let file_id = "1C8jyVjkMgeglYK-FnmTuoRqwf5Nd6PGG";
//     let download_url = format!("https://drive.google.com/uc?export=download&id={}", file_id);
//     let client = Client::new();

//     let response = client.get(&download_url).send().await?;

//     if response.status().is_success() {
//         let content = response.text().await?;
//         Ok(content.trim().to_string())
//     } else {
//         Err(format!("Failed to retrieve the file: {}", response.status()).into())
//     }
// }
