use eframe::egui::{self, RichText};
use sqlx::sqlite::SqlitePool;
use tokio;
use tokio::sync::mpsc;
use std::collections::HashSet;
use std::fs::{self};
use std::thread::JoinHandle;
use serde::Deserialize;
use crate::assets::*;
use crate::processing::*;

#[derive( serde::Deserialize, serde::Serialize)]
#[serde(default)] 
pub struct Config {
    pub search: bool,
    // #[serde(skip)]
    // pub db: Database,
    // pub option: Option<String>,
    pub list: Vec<String>,
    pub selected: String,
    #[serde(skip)]
    pub status: String,
    #[serde(skip)]
    pub records: HashSet<FileRecord>,
    #[serde(skip)]
    pub working: bool,
    #[serde(skip)]
    tx: Option<mpsc::Sender<HashSet<FileRecord>>>,
    #[serde(skip)]
    rx: Option<mpsc::Receiver<HashSet<FileRecord>>>,
    #[serde(skip)]
    progress_sender: Option<mpsc::Sender<ProgressMessage>>,
    #[serde(skip)]
    progress_receiver: Option<mpsc::Receiver<ProgressMessage>>,
    #[serde(skip)]
    progress: (f32, f32),
    #[serde(skip)]
    handle: Option<tokio::task::JoinHandle<()>>,
}
impl Default for Config {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(1);
        let (progress_sender, progress_receiver) = mpsc::channel(32);
        Self {
            search: false,
            list: Vec::new(),
            selected: String::new(),
            status: String::new(),
            records: HashSet::new(),
            working: false,
            tx: Some(tx),
            rx: Some(rx),
            progress_sender: Some(progress_sender),
            progress_receiver: Some(progress_receiver),
            progress: (0.0, 0.0),
            handle: None,

        }
    }
}

impl Config {
    fn new(on: bool) -> Self {
        let (tx, rx) = mpsc::channel(1);
        let (progress_sender, progress_receiver) = mpsc::channel(32);
        // println!("Initializing new config with tx: {:?}, rx: {:?}", tx, rx);
        Self {
            search: on,
            list: Vec::new(),
            selected: String::new(),
            status: String::new(),
            records: HashSet::new(),
            working: false,
            tx: Some(tx),
            rx: Some(rx),
            progress_sender: Some(progress_sender),
            progress_receiver: Some(progress_receiver),
            progress: (0.0, 0.0),
            handle: None,

        }
    }
    fn new_option(on: bool, o: &str) -> Self {
        let (tx, rx) = mpsc::channel(1);
        let (progress_sender, progress_receiver) = mpsc::channel(32);
        // println!("Initializing new config with tx: {:?}, rx: {:?}", tx, rx);
        Self {
            search: on,
            list: Vec::new(),
            selected: o.to_string(),
            status: String::new(),
            records: HashSet::new(),
            working: false,
            tx: Some(tx),
            rx: Some(rx),
            progress_sender: Some(progress_sender),
            progress_receiver: Some(progress_receiver),
            progress: (0.0, 0.0),
            handle: None,

        }
    }
    
}

#[derive(Clone)]
pub struct Database {
    pub path: String,
    pub pool: SqlitePool,
    pub name: String,
    pub size: usize,
    pub columns: Vec<String>,
}

impl Database {
    pub async fn open(db_path: String) -> Self {
        let db_pool = SqlitePool::connect(&db_path).await.expect("Pool did not open");
        let db_size = get_db_size(&db_pool).await.expect("get db size");
        let db_columns = get_columns(&db_pool).await.expect("get columns");
        Self {
            path: db_path.clone(),
            pool: db_pool,
            name: db_path.split('/').last().expect("Name From Pathname").to_string(),
            size: db_size,
            columns: db_columns,
        }
    }
}



#[derive(Hash, Eq, PartialEq, Clone, Debug,)]
pub struct FileRecord {
    pub id: usize,
    pub filename: String,
    pub duration: String,
}

pub enum ProgressMessage {
    Update(usize, usize), // (current, total)
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // #[serde(skip)]
    // rt: tokio::runtime::Runtime,
    #[serde(skip)]
    tx: Option<mpsc::Sender<Database>>,
    #[serde(skip)]
    rx: Option<mpsc::Receiver<Database>>,
    #[serde(skip)]
    c_tx: Option<mpsc::Sender<Database>>,
    #[serde(skip)]
    c_rx: Option<mpsc::Receiver<Database>>,
    #[serde(skip)]
    find_tx: Option<mpsc::Sender<usize>>,
    #[serde(skip)]
    find_rx: Option<mpsc::Receiver<usize>>,
    #[serde(skip)]
    replace_tx: Option<mpsc::Sender<HashSet<FileRecord>>>,
    #[serde(skip)]
    replace_rx: Option<mpsc::Receiver<HashSet<FileRecord>>>,
    #[serde(skip)]
    db: Option<Database>,
    #[serde(skip)]
    c_db: Option<Database>,
    // total_records: usize,

    column: String,
    find: String,
    replace: String,
    dirty: bool,

    main: Config,
    group: Config,
    group_null: bool,
    tags: Config,
    deep: Config,
    compare: Config,

    #[serde(skip)] // This how you opt-out of serialization of a field
    safe: bool,
    #[serde(skip)] // This how you opt-out of serialization of a field
    dupes_db: bool,
    #[serde(skip)] // This how you opt-out of serialization of a field
    my_panel: Panel,
    #[serde(skip)] // This how you opt-out of serialization of a field
    new_tag: String,
    #[serde(skip)] // This how you opt-out of serialization of a field
    sel_tags: Vec<usize>,
    #[serde(skip)] // This how you opt-out of serialization of a field
    new_line: String,
    #[serde(skip)] // This how you opt-out of serialization of a field
    sel_line: Option<usize>,
    #[serde(skip)] // This how you opt-out of serialization of a field
    order_text: String,
    #[serde(skip)] // This how you opt-out of serialization of a field
    help: bool,
    #[serde(skip)] // This how you opt-out of serialization of a field
    replace_safety: bool,
    #[serde(skip)] // This how you opt-out of serialization of a field
    count: usize,
    #[serde(skip)] // This how you opt-out of serialization of a field
    gather_dupes: bool,

}    



#[derive(PartialEq, serde::Serialize, Deserialize)]
enum Panel { Duplicates, Order, OrderText, Tags, Find }


impl Default for TemplateApp {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel(1);
        let (c_tx, c_rx) = mpsc::channel(1);
        let (find_tx, find_rx) = mpsc::channel(1);
        let (replace_tx, replace_rx) = mpsc::channel(1);
        let mut app = Self {
            // rt: tokio::runtime::Runtime::new().unwrap(),
            tx: Some(tx),
            rx: Some(rx),
            c_tx: Some(c_tx),
            c_rx: Some(c_rx),
            find_tx: Some(find_tx),
            find_rx: Some(find_rx),
            replace_tx: Some(replace_tx),
            replace_rx: Some(replace_rx),
            db: None,
            c_db: None,
            // total_records: 0,
            column: "Filepath".to_owned(),
            find: String::new(),
            replace: String::new(),
            dirty: true,
            main: Config::new(true),
            group: Config::new_option(false, "Show"),
            group_null: false,
     
            tags: Config::new_option(false, "-"),
            deep: Config::new(false),
            compare: Config::new(false),

            safe: true,
            dupes_db: false,
            my_panel: Panel::Duplicates,
            new_tag: String::new(),
            sel_tags: Vec::new(),
            new_line: String::new(),
            sel_line: None,
            order_text: String::new(),
            help: false,
            replace_safety: false,
            count: 0,
            gather_dupes: false,
        };
        app.tags.list = default_tags();
        app.main.list = default_order();

        app
    }
}

impl TemplateApp {
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
    fn reset_to_defaults(&mut self, db: Option<Database>) {
        *self = Self::default();
        self.db = db;
    }
        
    fn reset_to_tjf_defaults(&mut self, db: Option<Database>) {
        *self = Self::default();
        self.db = db;
        self.main.list = tjf_order();
        self.tags.list = tjf_tags();        
    }



}


impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui
        
        
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Open Database").clicked() {
                            ui.close_menu();
                            
                            let tx = self.tx.clone().expect("tx channel exists");
                            tokio::spawn(async move {
                                let db = open_db().await.unwrap();
                                if let Err(_) = tx.send(db).await {
                                    // eprintln!("Failed to send db");
                                }
                            });
                        }
                        if ui.button("Close Database").clicked() {ui.close_menu(); self.db = None;}
                        ui.separator();
                        if ui.button("Restore Defaults").clicked() {
                            ui.close_menu(); 
                            self.reset_to_defaults(self.db.clone());
                          
                        }
                        if  ui.input(|i| i.modifiers.alt ) {
                            if ui.button("TJF Defaults").clicked() {
                                ui.close_menu();
                                self.reset_to_tjf_defaults(self.db.clone());
                            }
                        }
                        ui.separator();
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
              
                    ui.menu_button("View", |ui| {
                        if ui.button("Duplicates Search").clicked() {ui.close_menu(); self.my_panel = Panel::Duplicates}
                        if ui.button("Find & Replace").clicked() {ui.close_menu(); self.my_panel = Panel::Find}
                        ui.separator();
                        if ui.button("Duplicate Search Logic").clicked() {ui.close_menu(); self.my_panel = Panel::Order}
                        if ui.button("Tag Editor").clicked() {ui.close_menu(); self.my_panel = Panel::Tags}

                    });
               
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {

                        egui::widgets::global_dark_light_mode_buttons(ui);
                    });
                  
                    // egui::menu::bar(ui, |ui| {
                        
                    //     ui.selectable_value(&mut self.my_panel, Panel::Duplicates, RichText::new("Duplicate Filename Search"),);
                        
                    //     ui.add_space(16.0);
                    //     // ui.selectable_value(&mut self.my_panel, Panel::Order, "Adjust Search Order Config",);
                    //     // ui.add_space(16.0);
                    //     // ui.selectable_value(&mut self.my_panel, Panel::Tags, "Manage Audiosuite Tags",);
                    //     ui.selectable_value(&mut self.my_panel, Panel::Find, "Find and Replace",);
                    //     ui.add_space(16.0);
    
                    // });
                }
                
               
            });

            
            
        });
        
    // The central panel the region left after adding TopPanel's and SidePanel's
            
            egui::CentralPanel::default().show(ctx, |ui| {
                if let Some(rx) = self.rx.as_mut() {
                    if let Ok(db) = rx.try_recv() {
                        self.db = Some(db);
                    }
                }
                if self.db.is_none() {
                    ui.vertical_centered(|ui| {

                        large_button(ui, "Open Database", ||{
                            let tx = self.tx.clone().expect("tx channel exists");
                            tokio::spawn(async move {
                                let db = open_db().await.unwrap();
                                if let Err(_) = tx.send(db).await {
                                    // eprintln!("Failed to send db");
                                }
                            });

                        });


                        // if ui.add_sized([200.0, 50.0], egui::Button::new(RichText::new("Open Database").size(24.0).strong())).clicked() {
                        //     let tx = self.tx.clone().expect("tx channel exists");
                        //     tokio::spawn(async move {
                        //         let db = open_db().await.unwrap();
                        //         if let Err(_) = tx.send(db).await {
                        //             eprintln!("Failed to send db");
                        //         }
                        //     });
                        // }
                    });
                    return; // Return early if database is not loaded
                }
                if let Some(db)  = &self.db {
                    ui.horizontal(|_| {});
                
                    ui.vertical_centered(|ui| {
            
                        ui.heading(RichText::new(&db.name).size(24.0).strong().extra_letter_spacing(5.0));
                        ui.label(format!("{} records", &db.size));
                        
                    });
                    ui.horizontal(|_| {});
                    ui.separator();
                    ui.horizontal(|_| {});


                    match self.my_panel {
                        Panel::Find => {
                            ui.heading("Find and Replace");
                            ui.label("Note: Search is Case Sensitive");
                            ui.separator();
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
                                combo_box(ui, "find_column", &mut self.column, &db.columns);
                            });
                            ui.separator();
                            ui.checkbox(&mut self.dirty,"Mark Records as Dirty?");
                            ui.label("Dirty Records are audio files with metadata that is not embedded");
                            ui.separator();
                
                            if self.find.is_empty() {
                                return;
                            }
                            if ui.button("Search").clicked() {
                                self.replace_safety = true;
                             
                                let tx = self.find_tx.clone().expect("tx channel exists");
                                let pool = db.pool.clone();
                                let mut find = self.find.clone();
                                let mut column = self.column.clone();
                                tokio::spawn(async move {
                                    let count = smreplace_get(&pool, &mut find, &mut column).await.unwrap();
                                    if let Err(_) = tx.send(count).await {
                                        // eprintln!("Failed to send db");
                                    }
                                });
                                
                                
                            }
                            if let Some(rx) = self.find_rx.as_mut() {
                                if let Ok(count) = rx.try_recv() {
                                    self.count = count;
                                }
                            }
                            if self.replace_safety {
                                // if let Some(path) = &self.db {
                                //     self.count = smreplace_get(path.clone(), &mut self.find,  &mut self.column);

                                // }
                                ui.label(format!("Found {} records matching '{}' in {} of SM database: {}", self.count, self.find, self.column, db.name));
                                if self.count == 0 {return;}
                                ui.label(format!("Replace with \"{}\" ?", self.replace));
                                ui.label(format!("This is NOT undoable"));
                                ui.separator();
                                ui.horizontal(|ui| {

                                    if ui.button("Proceed").clicked() {
                                        // let tx = self.find_tx.clone().expect("tx channel exists");
                                        let pool = db.pool.clone();
                                        let mut find = self.find.clone();
                                        let mut replace = self.replace.clone();
                                        let mut column = self.column.clone();
                                        let dirty = self.dirty;
                                        tokio::spawn(async move {
                                            smreplace_process(&pool, &mut find, &mut replace, &mut column, dirty).await;
                                            // if let Err(_) = tx.send(count).await {
                                            //     eprintln!("Failed to send db");
                                            // }
                                        });
                                        self.replace_safety = false;
                                    }
                                    if ui.button("Cancel").clicked() {
                                        self.count = 0;
                                        self.replace_safety = false;
                                    }
                                });
                            }
                            else if self.count > 0 {
                                ui.label(format!("{} records replaced", self.count));
                            }
                                
                        }
                        Panel::Duplicates => {
                            ui.heading(RichText::new("Search for Duplicate Records").strong());
                
                            ui.checkbox(&mut self.main.search, "Basic Duplicate Filename Search");
                                
                                //GROUP GROUP GROUP GROUP
                                ui.horizontal(|ui| {
                                    ui.add_space(24.0);
                                    ui.checkbox(&mut self.group.search, "Group Duplicate Filename Search by: ");
                                    combo_box(ui, "group", &mut self.group.selected, &db.columns);
                                    
                                    
                                });
                                ui.horizontal(|ui| {
                                    ui.add_space(44.0);
                                    ui.label("Records without group entry: ");
                                    ui.radio_value(&mut self.group_null, false, "Skip/Ignore");
                                    ui.radio_value(&mut self.group_null, true, "Process Together");
                                    // ui.checkbox(&mut self.group_null, "Process records without defined group together, or skip?");
                                });
                                ui.horizontal( |ui| {
                                    if self.group.working { ui.spinner();}
                                    else {ui.add_space(24.0);}
                                    ui.label(RichText::new(self.group.status.clone()).strong());
                                    

                                });
                                ui.separator();
                                //DEEP DIVE DEEP DIVE DEEP DIVE
                                ui.checkbox(&mut self.deep.search, "Deep Dive Duplicates Search (Slow)");
                                ui.horizontal( |ui| {
                                    ui.add_space(24.0);
                                    ui.label("Filenames ending in .#, .#.#.#, or .M will be examined as possible duplicates");
                                });
                                ui.horizontal( |ui| {
                                    if self.deep.working {ui.spinner();}
                                    else {ui.add_space(24.0)}
                                    ui.label(RichText::new(self.deep.status.clone()).strong());

                                });

                                

                                if let Some(progress_receiver) = &mut self.deep.progress_receiver {
                                    // Update progress state based on messages
                                    while let Ok(message) = progress_receiver.try_recv() {
                                        if let ProgressMessage::Update(current, total) = message {
                                            self.deep.progress = (current as f32, total as f32);
                                        }

                                }

                                }
                                if self.deep.working{
                                    ui.add( egui::ProgressBar::new(self.deep.progress.0 / self.deep.progress.1)
                                            // .text("progress")
                                            .desired_height(4.0)
                                        );
                                    ui.label(format!("Progress: {} / {}", self.deep.progress.0, self.deep.progress.1));
                                }

                                // Use `progress` to update your UI
                                ui.separator();

                            //TAGS TAGS TAGS TAGS
                            ui.checkbox(&mut self.tags.search, "Search for Records with AudioSuite Tags");


                                ui.horizontal(|ui| {
                                    ui.add_space(24.0);
                                    ui.label("Filenames with Common Protools AudioSuite Tags will be marked for removal")
                                });
                                
                                ui.horizontal(|ui| {
                                    if self.tags.working {ui.spinner();}
                                    else {ui.add_space(24.0);}
                                    ui.label(RichText::new(self.tags.status.clone()).strong());

                                });
                                ui.separator();

                            //COMPARE COMPARE COMPARE COMPARE
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.compare.search, "Compare against database: ");
                                if let Some(cdb) = &self.c_db {
                                    ui.label(&cdb.name);
                                }

                                
                                button(ui, "Select DB", ||{
                                    let tx = self.c_tx.clone().expect("tx channel exists");
                                        tokio::spawn(async move {
                                            let db = open_db().await.unwrap();
                                            if let Err(_) = tx.send(db).await {
                                                // eprintln!("Failed to send db");
                                            }
                                        });
                                });

                                // if ui.button("Select DB").clicked() {
                                        
                                // }
                                if let Some(rx) = self.c_rx.as_mut() {
                                    if let Ok(db) = rx.try_recv() {
                                        self.c_db = Some(db);
                                    }
                                }

                            });
                        
                                ui.horizontal(|ui| {
                                    ui.add_space(24.0);
                                    ui.label("Filenames from Target Database found in Comparison Database will be Marked for Removal");
                                });
                                ui.horizontal(|ui| {
                                    if self.compare.working {ui.spinner();}
                                    else {ui.add_space(24.0)}
                                    ui.label(RichText::new(self.compare.status.clone()).strong());

                                });
                                ui.separator();

                            ui.horizontal(|_| {});
                            ui.checkbox(&mut self.safe, "Create Safety Database of Thinned Records");
                            ui.checkbox(&mut self.dupes_db, "Create Database of Duplicate Records");
                            ui.separator();

                            ui.horizontal( |_ui| {});
                            
                           
                            ui.horizontal(|ui| {
                                if  ui.input(|i| i.modifiers.alt ) {
                                    if ui.button("Search and Remove Duplicates").clicked() {}
                                } else {
                                    if ui.button("Search for Duplicates").clicked() {
                                        // self.gather_dupes = true;
                                        gather_duplicates(self);
                                    }

                                }

                                if self.main.records.len() > 0 && !handles_active(self) {
                                    

                                    // button(ui, "Remove Duplicates", remove_duplicates);

                                    if ui.button("Remove Duplicates").clicked() {
                                       remove_duplicates(self);
                                    }
                                }
                                if handles_active(self) {
                                    button(ui, "Cancel", ||abort_all(self));

                                }

                            });
                         
                            ui.horizontal( |ui| {
                                if self.main.working {ui.spinner();}
                                ui.label(RichText::new(self.main.status.clone()).strong());

                            });

                            if let Some(progress_receiver) = &mut self.main.progress_receiver {
                                // Update progress state based on messages
                                while let Ok(message) = progress_receiver.try_recv() {
                                    if let ProgressMessage::Update(current, total) = message {
                                        self.main.progress = (current as f32, total as f32);
                                        println!("received progress {} / {}", current, total);
                                    }
                                }
                                
                            }
                            
                            
                            if self.main.working{
                                ui.add( egui::ProgressBar::new(self.main.progress.0 / self.main.progress.1)
                                // .text("progress")
                                .desired_height(4.0)
                            );
                            if self.main.progress.1 > 0.0 {
                                println!("main progress {} / {}", self.main.progress.0, self.main.progress.1);
                                if self.main.progress.0 == self.main.progress.1 {self.main.working = false;}
                            }
                        }
                            receive_duplicates(self);
                            
                        }
                        Panel::Order => {
                            if self.help {order_help(ui)}
                            
                            for (index, line) in self.main.list.iter_mut().enumerate() {
                                let checked = self.sel_line == Some(index);
                                if ui.selectable_label(checked, line.clone()).clicked {
                                    self.sel_line = if checked { None } else { Some(index) };
                                }
                            }
                            ui.separator();

                            order_toolbar(ui,self);

                            ui.separator();
                            if ui.button("Text Editor").clicked() {
                                self.order_text = self.main.list.join("\n");
                                self.my_panel = Panel::OrderText;
                            }
                            
                        }

                        Panel:: OrderText => {
                            if self.help {order_help(ui)}

                            ui.columns(1, |columns| {
                                // columns[0].heading("Duplicate Filename Keeper Priority Order:");
                                columns[0].text_edit_multiline(&mut self.order_text);
                            });
                            ui.separator();
                            if ui.button("Save").clicked() {
                                self.main.list = self.order_text.lines().map(|s| s.to_string()).collect();
                                self.my_panel = Panel::Order;
                            }
                        }

                        Panel::Tags => {
                            ui.heading("Tag Editor");
                            ui.label("Protools Audiosuite Tags use the following format:  -example_");
                            ui.label("You can enter any string of text and if it is a match, the file will be marked for removal");
                            
                            ui.separator();
                            let num_columns = 6;
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                egui::Grid::new("Tags Grid")
                                .num_columns(num_columns)
                                .spacing([20.0, 8.0])
                                .striped(true)
                                .show(ui, |ui| {
                                    for (index, tag) in self.tags.list.iter_mut().enumerate() {
                                        // Check if current index is in `sel_tags`
                                        let is_selected = self.sel_tags.contains(&index);
                                        
                                        if ui.selectable_label(is_selected, tag.clone()).clicked() {
                                            if is_selected {
                                                // Deselect
                                                self.sel_tags.retain(|&i| i != index);
                                            } else {
                                                // Select
                                                self.sel_tags.push(index);
                                            }
                                        }
                                        
                                        if (index + 1) % num_columns == 0 {
                                            ui.end_row(); // Move to the next row after 4 columns
                                        }
                                    }
                                    
                                    // End the last row if not fully filled
                                    if self.tags.list.len() % 4 != 0 {
                                        ui.end_row();
                                    }
                                });
                            });
                            ui.separator();
                            ui.horizontal(|ui| {
                                if ui.button("Add Tag:").clicked() && !self.new_tag.is_empty() {

                                    self.tags.list.push(self.new_tag.clone());
                                    self.new_tag.clear(); // Clears the string      
                                    self.tags.list.sort_by_key(|s| s.to_lowercase());
                                }
                                ui.text_edit_singleline(&mut self.new_tag);
                                
                                
                            });
                            if ui.button("Remove Selected Tags").clicked() {
                                // Sort and remove elements based on `sel_tags`
                                let mut sorted_indices: Vec<usize> = self.sel_tags.clone();
                                sorted_indices.sort_by(|a, b| b.cmp(a)); // Sort in reverse order
                        
                                for index in sorted_indices {
                                    if index < self.tags.list.len() {
                                        self.tags.list.remove(index);
                                    }
                                }
                        
                                // Clear the selection list after removal
                                self.sel_tags.clear();
                            }
                        
                        }



                    }
                }

        });
    }
    
}

pub fn order_toolbar(ui: &mut egui::Ui, app: &mut TemplateApp) {
    ui.horizontal(|ui| {
        if ui.button("Up").clicked() {
            if let Some(index) = app.sel_line {
                if index > 0 {
                    app.sel_line = Some(index-1);
                    app.main.list.swap(index, index-1);
                }
            }
        }
        if ui.button("Down").clicked() {
            if let Some(index) = app.sel_line {
                if index < app.main.list.len() - 1 {
                    app.sel_line = Some(index+1);
                    app.main.list.swap(index, index+1);
                }
            }
        }
        if ui.button("Remove").clicked() {
            if let Some(index) = app.sel_line {
                app.main.list.remove(index);
                app.sel_line = None;
            }
        }            
        if ui.button("Add Line:").clicked {
            
            if app.new_line.len() > 0 {
                app.main.list.insert(0, app.new_line.clone());
                app.new_line.clear();
        }}
        ui.text_edit_singleline(&mut app.new_line);    
        if ui.button("Help").clicked {app.help = !app.help}
    });
}


pub fn gather_duplicates(app: &mut TemplateApp) {
    app.main.records.clear();
    if let Some(db) = app.db.clone() {
    
    let pool = db.pool;

    //  app.main.status = format!("Opening {}", db.name);
    // let mut conn = Connection::open(&source_db_path).unwrap(); 

    if app.main.search {
        app.main.status = format!("Searching for Dupicates");
        app.group.working = true;
        app.group.status = format!("Searching for duplicate filenames");
        if let Some(tx) = app.group.tx.clone() {
            let order = app.main.list.clone();
            let mut group_sort = None;
            if app.group.search {group_sort = Some(app.group.selected.clone())}
            let group_null = app.group_null;
            let p = pool.clone();
            let handle = tokio::spawn(async move {
                println!("tokio spawn main");
                let results  = gather_duplicate_filenames_in_database(&p, order, &group_sort, group_null, true).await;
                if let Err(_) = tx.send(results.expect("error on gather tags")).await {
                    eprintln!("Failed to send db");
                }
            });
            app.group.handle = Some(handle);
        }
  
    }

    if app.deep.search {
        app.deep.working = true;
        app.deep.status = format!("Performing Deep dive");
        if app.deep.tx.is_none() {println!("deep is none");}
            if let Some(tx) = app.deep.tx.clone() {
                if let Some(sender) = app.deep.progress_sender.clone() {

                    println!("if let some");
                    let p = pool.clone();
                    let handle = tokio::spawn(async move {
                        println!("tokio spawn tags");
                        let results  = gather_deep_dive_records(&p, sender).await;
                        if let Err(_) = tx.send(results.expect("error on gather tags")).await {
                            eprintln!("Failed to send db");
                        }
                    });
                    app.deep.handle = Some(handle);
                };

            }
    
    }

    if app.tags.search {
        app.main.status = format!("Searching for tags");
        app.tags.working = true;
        app.tags.status = format!{"Searching records with matching tags"};
        if app.tags.tx.is_none() {println!("is none");}
            if let Some(tx) = app.tags.tx.clone() {
                println!("if let some");
                let tags = app.tags.list.clone();
                let p = pool.clone();
                let handle = tokio::spawn(async move {
                    println!("tokio spawn tags");
                    let results  = gather_filenames_with_tags(&p, &tags).await;
                    if let Err(_) = tx.send(results.expect("error on gather tags")).await {
                        eprintln!("Failed to send db");
                    }
                });
                app.tags.handle = Some(handle);

            }

                            
                            

    }
    
    if app.compare.search && app.c_db.is_some() {
        if let Some(cdb) = &app.c_db {
            app.compare.working = true;
            app.compare.status = format!("Comparing against {}", cdb.name);
            if app.compare.tx.is_none() {println!("compare tx is none");}
            if let Some(tx) = app.compare.tx.clone() {
                println!("if let some");
                let p = pool.clone();
                let c_pool = cdb.pool.clone();
                let handle = tokio::spawn(async move {
                    println!("tokio spawn compare");
                    let results  = gather_compare_database_overlaps(&p, &c_pool).await;
                    if let Err(_) = tx.send(results.expect("error on compare db")).await {
                        eprintln!("Failed to send db");
                    }
                });
                app.compare.handle = Some(handle);

            }

        }

    }
    
}
}

fn receive_duplicates(app: &mut TemplateApp) {
    if let Some(rx) = app.group.rx.as_mut() {
        if let Ok(records) = rx.try_recv() {
            app.group.records = records;
            app.group.handle = None;
            app.group.working = false;
            app.group.status = format!{"Found {} duplicate filenames", app.group.records.len()};
            app.main.records.extend(app.group.records.clone());
        }
    }

    if let Some(rx) = app.deep.rx.as_mut() {
        
        if let Ok(records) = rx.try_recv() {
            app.deep.records = records;
            app.deep.handle = None;
            app.deep.working = false;
            app.deep.status = format!{"Found {} records with similar filenames", app.deep.records.len()};
            app.main.records.extend(app.deep.records.clone());
        }
    }
    

    if let Some(rx) = app.tags.rx.as_mut() {
        if let Ok(records) = rx.try_recv() {
            app.tags.records = records;
            app.tags.handle = None;
            app.tags.working = false;
            app.tags.status = format!{"Found {} records with matching tags", app.tags.records.len()};
            app.main.records.extend(app.tags.records.clone());
        }
    }
    
    if let Some(rx) = app.compare.rx.as_mut() {
        if let Ok(records) = rx.try_recv() {
            app.compare.records = records;
            app.compare.handle = None;
            app.compare.working = false;
            app.compare.status = format!{"Found {} overlapping records in {}", app.compare.records.len(), app.c_db.clone().unwrap().name};
            app.main.records.extend(app.compare.records.clone());
        }
    }
    
        if app.main.records.is_empty() {
            app.main.status = format!("No records marked for removal.");
           
        } else {
            app.main.status = format!("Marked {} total records for removal.", app.main.records.len());
    
    }
}

fn remove_duplicates(app: &mut TemplateApp) {

    if let Some(mut db) = app.db.clone() {
        app.main.working = true;
        app.main.status = "Removing Records".to_string();
        let records = app.main.records.clone();
        let mut p = db.pool.clone();
        let safe = app.safe;
        let dupes = app.dupes_db;
        if app.main.progress_sender.is_none() {println!("main progress sender is none")}
        if let Some(sender) = app.main.progress_sender.clone() {
            let handle = tokio::spawn(async move {
                println!("Spawn deletion process");
                if safe {
                    println!("Creating Thinned Database");
                    let work_db_path = format!("{}_thinned.sqlite", &db.path.trim_end_matches(".sqlite"));
                    fs::copy(&db.path, &work_db_path);
                    p = SqlitePool::connect(&work_db_path).await.expect("Pool did not open");
                }
                println!("deleting file records");
                let results = delete_file_records(&p, &records, sender.clone()).await;
                match results {
                    Ok(_) => println!("Deletion completed successfully"),
                    Err(e) => eprintln!("Error during deletion: {}", e),
                }
                if dupes {
                    print!("creating dupes database");
                    let results = create_duplicates_db(&db.path, &records, sender.clone()).await;
                    match results {
                        Ok(_) => println!("Deletion completed successfully"),
                        Err(e) => eprintln!("Error during deletion: {}", e),
                    }

                }



                println!("Exiting tokio spawn");
               
                db.size = get_db_size(&p).await.expect("get db size");
            });
            app.main.handle = Some(handle);
            
        }
        
    }
        

}



fn abort_all(app: &mut TemplateApp) {

    if let Some(handle) = &app.main.handle {
        handle.abort();
    }
    app.main.handle = None;
    app.main.working = false;

    if let Some(handle) = &app.group.handle {
        handle.abort();
    }
    app.group.handle = None;
    app.group.working = false;

    if let Some(handle) = &app.deep.handle {
        handle.abort();
    }
    app.deep.handle = None;
    app.deep.working = false;

    if let Some(handle) = &app.tags.handle {
        handle.abort();
    }
    app.tags.handle = None;
    app.tags.working = false;

    if let Some(handle) = &app.compare.handle {
        handle.abort();
    }
    app.compare.handle = None;
    app.compare.working = false;
}

fn handles_active(app: &TemplateApp) -> bool {
    app.main.handle.is_some() || 
    app.group.handle.is_some() || 
    app.deep.handle.is_some() || 
    app.tags.handle.is_some() || 
    app.compare.handle.is_some()
}