use crate::assets::*;
use crate::processing::*;
use eframe::egui::{self, RichText};
use rayon::prelude::*;
use serde::Deserialize;
use sqlx::sqlite::SqlitePool;
use std::collections::HashSet;
use std::fs::{self};
use std::hash::Hash;
use tokio::sync::mpsc;
use std::path::Path;

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
#[derive(Clone)]
struct Registration {
    name: String,
    email: String,
    key: String,
    #[serde(skip)]
    valid: Option<bool>,
}


impl Registration {
    fn validate(&mut self) {
        if generate_license_key(&self.name, &self.email) == self.key {
            self.valid = Some(true);
        } else {
            self.valid = Some(false);
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AsyncTunnel<T> {
    #[serde(skip)]
    pub tx: Option<mpsc::Sender<T>>,
    #[serde(skip)]
    pub rx: Option<mpsc::Receiver<T>>,
}

impl<T> AsyncTunnel<T> {
    // Make `new` an associated function, and use `Self` for the return type
    pub fn new(channels: usize) -> AsyncTunnel<T> {
        let (tx, rx) = mpsc::channel(channels);
        AsyncTunnel {
            tx: Some(tx),
            rx: Some(rx),
        }
    }

    // You might want to add methods to send and receive messages
    // pub async fn send(&self, item: T) -> Result<(), mpsc::error::SendError<T>> {
    //     if let Some(tx) = &self.tx {
    //         tx.send(item).await
    //     } else {
    //         Err(mpsc::error::SendError(item))
    //     }
    // }

    // pub async fn receive(&self) -> Option<T> {
    //     if let Some(rx) = &self.rx {
    //         rx.recv().await.ok()
    //     } else {
    //         None
    //     }
    // }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]

pub struct Config {
    pub search: bool,
    pub list: Vec<String>,
    pub selected: String,
    #[serde(skip)]
    pub status: String,
    #[serde(skip)]
    pub status_io: AsyncTunnel<String>,
    #[serde(skip)]
    pub records: HashSet<FileRecord>,
    #[serde(skip)]
    pub working: bool,

    #[serde(skip)]
    pub records_io: AsyncTunnel<HashSet<FileRecord>>,
    #[serde(skip)]
    pub progress_io: AsyncTunnel<ProgressMessage>,
    #[serde(skip)]
    pub progress: (f32, f32),
    #[serde(skip)]
    pub handle: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for Config {
    fn clone(&self) -> Self {
        Config {
            search: self.search,
            list: self.list.clone(),
            selected: self.selected.clone(),
            status: self.status.clone(),
            status_io: AsyncTunnel::new(1),
            records: self.records.clone(),
            working: self.working,
            records_io: AsyncTunnel::new(1),
            progress_io: AsyncTunnel::new(32),
            progress: self.progress,
            handle: None, // JoinHandle does not implement Clone
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            search: false,
            list: Vec::new(),
            selected: String::new(),
            status: String::new(),
            status_io: AsyncTunnel::new(1),
            records: HashSet::new(),
            working: false,
            records_io: AsyncTunnel::new(1),
            progress_io: AsyncTunnel::new(32),
            progress: (0.0, 0.0),
            handle: None,
        }
    }
}

impl Config {
    fn new(on: bool) -> Self {
        Self {
            search: on,
            list: Vec::new(),
            selected: String::new(),
            status: String::new(),
            status_io: AsyncTunnel::new(1),
            records: HashSet::new(),
            working: false,
            records_io: AsyncTunnel::new(1),
            progress_io: AsyncTunnel::new(32),
            progress: (0.0, 0.0),
            handle: None,
        }
    }
    fn new_option(on: bool, o: &str) -> Self {

        Self {
            search: on,
            list: Vec::new(),
            selected: o.to_string(),
            status: String::new(),
            status_io: AsyncTunnel::new(1),
            records: HashSet::new(),
            working: false,
            records_io: AsyncTunnel::new(1),
            progress_io: AsyncTunnel::new(32),
            progress: (0.0, 0.0),
            handle: None,
        }
    }
    fn abort(&mut self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
        self.handle = None;
        self.working = false;
        self.records.clear();
        self.status.clear();
        self.progress = (0.0, 0.0);
    }

    fn receive_hashset(&mut self) -> Option<HashSet<FileRecord>> {
        if let Some(rx) = self.records_io.rx.as_mut() {
            if let Ok(records) = rx.try_recv() {
                self.records = records.clone();
                self.handle = None;
                self.working = false;
                self.progress = (0.0, 0.0);
                self.status = format! {"Found {} duplicate records", self.records.len()};
                return Some(records);
            }
        }
        None
    }
    fn receive_progress(&mut self) {
        if let Some(progress_receiver) = &mut self.progress_io.rx {
            while let Ok(message) = progress_receiver.try_recv() {
                let ProgressMessage::Update(current, total) = message;
                self.progress = (current as f32, total as f32);
            }
        }
    }
    fn receive_status(&mut self) {
        if let Some(status_receiver) = &mut self.status_io.rx {
            while let Ok(message) = status_receiver.try_recv() {
                self.status = message;
            }
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
    pub file_extensions: Vec<String>,
}

impl Database {
    pub async fn open(db_path: &str) -> Self {
        let db_pool = SqlitePool::connect(db_path)
            .await
            .expect("Pool did not open");
        let db_size = get_db_size(&db_pool).await.expect("get db size");
        let db_columns = get_columns(&db_pool).await.expect("get columns");
        // let db_extensions = get_audio_file_types(&db_pool)
        //     .await
        //     .expect("get extensions");
        Self {
            path: db_path.to_string(),
            pool: db_pool,
            name: db_path
                .split('/')
                .last()
                .expect("Name From Pathname")
                .to_string(),
            size: db_size,
            columns: db_columns,
            file_extensions: Vec::new(),
        }
    }
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct FileRecord {
    pub id: usize,
    pub filename: String,
    pub duration: String,
    pub path: String,
}

pub enum ProgressMessage {
    Update(usize, usize), // (current, total)
}

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy)]
pub enum Panel {
    Duplicates,
    Order,
    OrderText,
    Tags,
    Find,
    KeyGen,
}

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy)]
pub enum OrderOperator {
    Largest,
    Smallest,
    Contains,
    DoesNotContain,
    Is,
    IsNot,
    IsEmpty,
    IsNotEmpty,
}

impl OrderOperator {
    fn as_str(&self) -> &'static str {
        match self {
            OrderOperator::Largest => "Largest",
            OrderOperator::Smallest => "Smallest",
            OrderOperator::Is => "is",
            OrderOperator::IsNot => "is NOT",
            OrderOperator::Contains => "Contains",
            OrderOperator::DoesNotContain => "Does NOT Contain",
            OrderOperator::IsEmpty => "Is Empty",
            OrderOperator::IsNotEmpty => "Is NOT Empty",
        }
    }

    fn variants() -> &'static [OrderOperator] {
        &[
            OrderOperator::Largest,
            OrderOperator::Smallest,
            OrderOperator::Contains,
            OrderOperator::DoesNotContain,
            OrderOperator::Is,
            OrderOperator::IsNot,
            OrderOperator::IsEmpty,
            OrderOperator::IsNotEmpty,
        ]
    }
}

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy)]
pub enum Delete {
    Trash,
    Permanent,
}

impl Delete {
    fn as_str(&self) -> &'static str {
        match self {
            Delete::Trash => "Move to Trash",
            Delete::Permanent => "Permanently Delete",
        }
    }
    fn variants() -> &'static [Delete] {
        &[Delete::Trash, Delete::Permanent]
    }
 
    fn delete_files(&self, files: HashSet<&str>) -> Result<(), Box<dyn std::error::Error>> {
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

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    #[serde(skip)]
    db_io: AsyncTunnel<Database>,
    #[serde(skip)]
    cdb_io: AsyncTunnel<Database>,
    #[serde(skip)]
    find_io: AsyncTunnel<usize>,
    #[serde(skip)]
    replace_io: AsyncTunnel<HashSet<FileRecord>>,
    #[serde(skip)]
    extensions_io: AsyncTunnel<Vec<String>>,
    #[serde(skip)]
    gathering_extensions: bool,
    #[serde(skip)]
    db: Option<Database>,
    #[serde(skip)]
    c_db: Option<Database>,

    column: String,
    find: String,
    replace: String,
    search_replace_path: bool,
    dirty: bool,
    case_sensitive: bool,
    find_buf: String,
    replace_buf: String,

    main: Config,
    group: Config,
    group_null: bool,
    tags: Config,
    deep: Config,
    ignore_extension: bool,
    sel_extension: String,
    compare: Config,
   
    safe: bool,
    dupes_db: bool,
    remove_files: bool,
    delete_action: Delete,
    #[serde(skip)]
    my_panel: Panel,
    #[serde(skip)]
    new_tag: String,
    #[serde(skip)]
    sel_tags: Vec<usize>,
    #[serde(skip)]
    new_line: String,
    #[serde(skip)]
    sel_line: Option<usize>,
    #[serde(skip)]
    order_text: String,
    #[serde(skip)]
    help: bool,
    #[serde(skip)]
    replace_safety: bool,
    #[serde(skip)]
    count: usize,
    #[serde(skip)]
    gather_dupes: bool,
    #[serde(skip)]
    go_search: bool,
    #[serde(skip)]
    go_replace: bool,

    order_friendly: Vec<String>,
    order_column: String,
    order_operator: OrderOperator,
    #[serde(skip)]
    order_input: String,
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
            replace_io: AsyncTunnel::new(1),
            extensions_io: AsyncTunnel::new(1),
            gathering_extensions: false,
            db: None,
            c_db: None,
          
            column: "Library".to_owned(),
            find: String::new(),
            replace: String::new(),
            search_replace_path: true,
            dirty: true,
            case_sensitive: true,
            find_buf: String::new(),
            replace_buf: String::new(),
           
            main: Config::new(true),
            group: Config::new_option(false, "Show"),
            group_null: false,

            tags: Config::new_option(false, "-"),
            deep: Config::new(false),
            ignore_extension: false,
            sel_extension: String::new(),
            compare: Config::new(false),
           
            safe: true,
            dupes_db: false,
            remove_files: false,
            delete_action: Delete::Trash,
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
            go_search: false,
            go_replace: false,
            order_friendly: Vec::new(),
            order_column: "Pathname".to_owned(),
            order_operator: OrderOperator::Contains,
            order_input: String::new(),
            marked_records: String::new(),
            records_window: false,
            scroll_to_top: false,

            registered: Registration::default(),
        };
        app.tags.list = default_tags();
        app.main.list = default_order();
        app.order_friendly = default_order_friendly();

        app
    }
}
impl App {
    fn clear_status(&mut self) {
        self.main.status.clear();
        self.main.records.clear();
        self.group.status.clear();
        self.group.records.clear();
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
    fn reset_to_defaults(
        &mut self,
        db: Option<Database>,
        panel: Panel,
        registration: Registration,
    ) {
        *self = Self::default();
        self.db = db;
        self.my_panel = panel;
        self.registered = registration;
    }

    fn reset_to_tjf_defaults(
        &mut self,
        db: Option<Database>,
        panel: Panel,
        registration: Registration,
    ) {
        *self = Self::default();
        self.db = db;
        self.my_panel = panel;
        self.registered = registration;
        self.main.list = tjf_order();
        self.order_friendly = tjf_order_friendly();
        self.tags.list = tjf_tags();
        self.deep.search = true;
        self.tags.search = true;
        self.dupes_db = true;
        self.ignore_extension = true;
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

        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!

                ui.menu_button(RichText::new("File").weak(), |ui| {
                    if ui.button("Open Database").clicked() {
                        ui.close_menu();
                        // self.clear_status();
                        let tx = self.db_io.tx.clone().expect("tx channel exists");
                        tokio::spawn(async move {
                            let db = open_db().await.unwrap();
                            let _ = tx.send(db).await;
                        });

                    }
                    if ui.button("Close Database").clicked() {
                        ui.close_menu();
                        self.clear_status();
                        abort_all(self);
                        self.db = None;
                    }

                    ui.separator();
                    if ui.button("Restore Defaults").clicked() {
                        ui.close_menu();
                        self.clear_status();
                        self.reset_to_defaults(
                            self.db.clone(),
                            self.my_panel,
                            self.registered.clone(),
                        );
                    }
                    if ui.input(|i| i.modifiers.alt) && ui.button("TJF Defaults").clicked() {
                        ui.close_menu();
                        self.clear_status();
                        self.reset_to_tjf_defaults(
                            self.db.clone(),
                            self.my_panel,
                            self.registered.clone(),
                        );
                    }
                    egui::widgets::global_dark_light_mode_buttons(ui);
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

                            if ui
                                .add_sized(
                                    [200.0, 50.0],
                                    egui::Button::new(
                                        RichText::new("Register").size(24.0).strong(),
                                    ),
                                )
                                .clicked()
                            {
                                #[cfg(debug_assertions)]
                                {
                                    if ui.input(|i| i.modifiers.alt)
                                    // && ui.input(|i| i.modifiers.command)
                                    // && ui.input(|i| i.modifiers.shift)
                                    // && ui.input(|i| i.modifiers.ctrl)
                                    {
                                        self.registered.key = generate_license_key(
                                            &self.registered.name,
                                            &self.registered.email,
                                        );
                                    }
                                }
                                self.registered.validate();
                            }
                        });
                    }
                    #[cfg(debug_assertions)]
                    {
                        ui.separator();
                        if ui.button(RichText::new("Unregister")).clicked() {
                            self.registered.name.clear();
                            self.registered.email.clear();
                            self.registered.key.clear();
                            self.registered.valid = Some(false);
                            ui.close_menu();
                        }
                    
                        if ui.button("KeyGen").clicked() {
                            ui.close_menu();
                            self.my_panel = Panel::KeyGen;
                        }
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.label(RichText::new("|").weak());

                self.panel_tab_bar(ui);

                if ui.available_width() > 20.0 {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                        egui::widgets::global_dark_light_mode_switch(ui);
                        ui.label(RichText::new("|").weak());
                    });
                }

                // // let mut show_help_window = self.help;
                // if self.help {
                //     egui::Window::new("Records Marked for Duplication")
                //         .open(&mut self.help) // Control whether the window is open
                //         .show(ctx, |ui| {
                //             ui.label("This is a dialog!");
                //             // if ui.button("Close").clicked() {
                //             //     self.help = false; // Close the window when clicked
                //             // }
                //         });
                // }
                // }
            });
        });

        // The central panel the region left after adding TopPanel's and SidePanel's

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(rx) = self.db_io.rx.as_mut() {
                if let Ok(db) = rx.try_recv() {
                    self.db = Some(db);
                }
            }
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
                        
                        let tx = self.db_io.tx.clone().expect("tx channel exists");
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
                        let tx = self.db_io.tx.clone().expect("tx channel exists");
                        tokio::spawn(async move {
                            let db = open_db().await.unwrap();
                            let _ = tx.send(db).await;
                        });
                    });
                });
            }

            //empty_line(ui);
            // ui.separator();
            //empty_line(ui);

            // self.panel_tab_bar(ui);

            empty_line(ui);
            ui.separator();
            empty_line(ui);

            egui::ScrollArea::vertical().show(ui, |ui| {
                match self.my_panel {
                    Panel::Find => {
                        self.find_panel(ui);
                    }

                    Panel::Duplicates => {
                        self.duplictes_panel(ui);
                    }

                    Panel::Order => {
                        self.order_panel(ui);
                    }

                    Panel::OrderText => {
                        self.order_text_panel(ui);
                    }

                    Panel::Tags => {
                        self.tags_panel(ui);
                    }

                    Panel::KeyGen => {
                        self.keygen_panel(ui);
                    }
                }
                empty_line(ui);
            });

            if self.records_window {
                let available_size = ctx.available_rect(); // Get the full available width and height
                let width = available_size.width() - 20.0;
                let height = available_size.height();
                egui::Window::new("Records Marked for Duplication")
                    .open(&mut self.records_window) // Control whether the window is open
                    .resizable(false) // Make window non-resizable if you want it fixed
                    .min_width(width)
                    .min_height(height)
                    .max_width(width)
                    .max_height(height)
                    .show(ctx, |ui| {
                        // ui.label("To Be Implemented\n Testing line break");

                        if self.scroll_to_top {
                            egui::ScrollArea::vertical()
                                .max_height(height) // Set the maximum height of the scroll area
                                .max_width(width)
                                .scroll_offset(egui::vec2(0.0, 0.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new(&self.marked_records).size(14.0));
                                });
                            self.scroll_to_top = false;
                        } else {
                            egui::ScrollArea::vertical()
                                .max_height(height) // Set the maximum height of the scroll area
                                .max_width(width)
                                // .scroll_offset(egui::vec2(0.0, 0.0))
                                .show(ui, |ui| {
                                    // for filename in &self.marked_records {
                                    ui.label(RichText::new(&self.marked_records).size(14.0));
                                    // }
                                    // ui.set_min_width(width - 20.0);
                                    // egui::Grid::new("Dupe Records")
                                    //     .spacing([20.0, 8.0])
                                    //     .striped(true)
                                    //     .show(ui, |ui| {
                                    //         for filename in &self.marked_records {
                                    //             ui.label(filename);
                                    //             ui.end_row();
                                    //         }
                                    //     });
                                    // ui.horizontal(|ui| {

                                    // });
                                });
                        }
                    });
            }

            // self.show_version_in_bottom_right(ctx);
        });

        let id2 = egui::Id::new("bottom panel registration");
        egui::Area::new(id2)
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(0.0, 0.0)) // Pin to bottom
            // .default_width(500.0)
            .show(ctx, |ui| {
                let mut label = RichText::new("*****UNREGISTERED")
                    .color(egui::Color32::from_rgb(255, 0, 0))
                    .strong();
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
            let mut space = ui.available_width() / 2.0 - 300.0;
            if space < 5.0 {
                space = 0.0
            };
            ui.add_space(space);
            // self.panel_tab_bar(ui);
            let size_big = 16.0;
            let size_small = 16.0;
            let column_width = 1.0; // Set the fixed width for each column
            {
                let checked = self.my_panel == Panel::Find;
                let label = "Find & Replace";
                let text = if checked {
                    RichText::new(label).size(size_big).strong()
                } else {
                    RichText::new(label).size(size_small).weak()
                };
                ui.allocate_exact_size(egui::vec2(column_width, 20.0), egui::Sense::click());

                if ui.selectable_label(checked, text).clicked() {
                    self.my_panel = Panel::Find;
                }
            }
            {
                let checked = self.my_panel == Panel::Duplicates;
                let label = "Search for Duplicates";
                let text = if checked {
                    RichText::new(label).size(size_big).strong()
                } else {
                    RichText::new(label).size(size_small).weak()
                };
                ui.allocate_exact_size(egui::vec2(column_width, 20.0), egui::Sense::click());
                if ui.selectable_label(checked, text).clicked() {
                    self.my_panel = Panel::Duplicates;
                }
            }
            {
                let checked = self.my_panel == Panel::Order;
                let label = "Preservation Priority";
                // let label = "Modify Duplicate Search Order";
                let text = if checked {
                    RichText::new(label).size(size_big).strong()
                } else {
                    RichText::new(label).size(size_small).weak()
                };
                ui.allocate_exact_size(egui::vec2(column_width, 20.0), egui::Sense::click());
                if ui.selectable_label(checked, text).clicked() {
                    self.my_panel = Panel::Order;
                }
            }
            {
                let checked = self.my_panel == Panel::Tags;
                let label = "Tag Editor";
                let text = if checked {
                    RichText::new(label).size(size_big).strong()
                } else {
                    RichText::new(label).size(size_small).weak()
                };
                ui.allocate_exact_size(egui::vec2(column_width, 20.0), egui::Sense::click());
                if ui.selectable_label(checked, text).clicked() {
                    self.my_panel = Panel::Tags;
                }
            }
            // egui::widgets::global_dark_light_mode_buttons(ui);
        });
    }

    fn find_panel(&mut self, ui: &mut egui::Ui) {
        if let Some(db) = &self.db {
            if db.size == 0 {
                ui.heading("No Records in Database");
                return;
            }
            ui.heading(RichText::new("Find and Replace").strong());
            // ui.label("Note: Search is Case Sensitive");
            empty_line(ui);
            ui.horizontal(|ui| {
                // ui.add_space(68.0);
                ui.checkbox(&mut self.case_sensitive, "Case Sensitive");
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
                egui::ComboBox::from_id_source("find_column")
                    .selected_text(&self.column)
                    .show_ui(ui, |ui| {
                        for item in filtered_columns {
                            ui.selectable_value(&mut self.column, item.clone(), item);
                        }
                    });
                // combo_box(ui, "find_column", &mut self.column, &filtered_columns);
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
                // self.searched = true;
                self.replace_safety = true;
                if self.search_replace_path {
                    self.column = "FilePath".to_string()
                }
                let tx = self.find_io.tx.clone().expect("tx channel exists");
                let pool = db.pool.clone();
                let mut find = self.find.clone();
                let mut column = self.column.clone();
                let case_sensitive = self.case_sensitive;
                tokio::spawn(async move {
                    let count = smreplace_get(&pool, &mut find, &mut column, case_sensitive)
                        .await
                        .unwrap();
                    let _ = tx.send(count).await;
                });
            }
            empty_line(ui);
            if let Some(rx) = self.find_io.rx.as_mut() {
                if let Ok(count) = rx.try_recv() {
                    self.count = count;
                }
            }
            if self.find != self.find_buf || self.replace != self.replace_buf {
                self.replace_safety = false;
                self.find_buf = self.find.clone();
                self.replace_buf = self.replace.clone();
            }
            if self.replace_safety {
                ui.label(
                    RichText::new(format!(
                        "Found {} records matching '{}' in {} of SM database: {}",
                        self.count, self.find, self.column, db.name
                    ))
                    .strong(),
                );
                if self.count == 0 {
                    return;
                }

                if self.registered.valid == Some(false) {
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
                        let pool = db.pool.clone();
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
            } else if self.count > 0 && self.registered.valid == Some(true) {
                ui.label(format!("{} records replaced", self.count));
            }
        } else {
            ui.heading(RichText::new("No Open Database").weak());
        }
    }

    fn duplictes_panel(&mut self, ui: &mut egui::Ui) {
        if let Some(db) = &mut self.db {
            if db.size == 0 {
                ui.heading("No Records in Database");
                return;
            }
            ui.heading(RichText::new("Search for Duplicate Records").strong());

            //GROUP GROUP GROUP GROUP
            ui.checkbox(&mut self.main.search, "Basic Duplicate Filename Search");

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.checkbox(
                    &mut self.group.search,
                    "Group Duplicate Filename Search by: ",
                );
                combo_box(ui, "group", &mut self.group.selected, &db.columns);
            });

            ui.horizontal(|ui| {
                ui.add_space(44.0);
                ui.label("Records without group entry: ");
                ui.radio_value(&mut self.group_null, false, "Skip/Ignore");
                ui.radio_value(&mut self.group_null, true, "Process Together");
            });

            ui.horizontal(|ui| {
                if self.group.working {
                    ui.spinner();
                } else {
                    ui.add_space(24.0);
                }
                ui.label(RichText::new(self.group.status.clone()).strong());
            });

            ui.separator();

            //DEEP DIVE DEEP DIVE DEEP DIVE
            ui.checkbox(&mut self.deep.search, "Deep Dive Duplicates Search");

            if let Some(rx) = self.extensions_io.rx.as_mut() {
                if let Ok(records) = rx.try_recv() {
                    db.file_extensions = records;
                }
            }
            if db.file_extensions.is_empty() && !self.gathering_extensions {
                self.gathering_extensions = true;
                let pool = db.pool.clone();
                if let Some(tx) = self.extensions_io.tx.clone() {
                    let _handle = tokio::spawn(async move {
                        let results = get_audio_file_types(&pool).await;

                        if (tx.send(results.expect("Tokio Results Error HashSet")).await).is_err() {
                            eprintln!("Failed to send db");
                        }
                    });
                }
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
                        ui.checkbox(&mut self.ignore_extension, "Ignore Filetypes");
    
                        if self.ignore_extension {
                            ui.label(
                                RichText::new(
                                    "('example.wav' and 'example.flac' will be considered duplicates)",
                                ), // .weak(),
                            );
                            // ui.label("Prefer:");
                            // combo_box(
                            //     ui,
                            //     "Extensions",
                            //     &mut self.sel_extension,
                            //     &db.file_extensions,
                            // );
                        } else {
                            ui.label(
                                RichText::new(
                                    "('example.wav' and 'example.flac' will be considered unique)",
                                ), // .weak(),
                            );
                        }
                    } else {
                        ui.label("All Records are of Filetype:");
                        ui.label(&self.sel_extension);
                    }
                });
            }
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    "Filenames ending in .#, .#.#.#, or .M will be examined as possible duplicates",
                );
            });

            ui.horizontal(|ui| {
                if self.deep.working {
                    ui.spinner();
                } else {
                    ui.add_space(24.0)
                }
                ui.label(RichText::new(self.deep.status.clone()).strong());
            });

            if self.deep.working {
                ui.add(
                    egui::ProgressBar::new(self.deep.progress.0 / self.deep.progress.1)
                        // .text("progress")
                        .desired_height(4.0),
                );
                ui.label(format!(
                    "Progress: {} / {}",
                    self.deep.progress.0, self.deep.progress.1
                ));
            }

            ui.separator();

            //TAGS TAGS TAGS TAGS
            ui.checkbox(
                &mut self.tags.search,
                "Search for Records with AudioSuite Tags",
            );

            ui.horizontal(|ui| {
                ui.add_space(24.0);
                ui.label(
                    "Filenames with Common Protools AudioSuite Tags will be marked for removal",
                )
            });

            ui.horizontal(|ui| {
                if self.tags.working {
                    ui.spinner();
                } else {
                    ui.add_space(24.0);
                }
                ui.label(RichText::new(self.tags.status.clone()).strong());
            });
            ui.separator();

            // //EXTENSIONS EXTENSIONS EXTENSIONS
            // ui.checkbox(
            //     &mut self.extensions.search,
            //     "Search for Duplicates Among Different File Types",
            // );

            // ui.horizontal(|ui| {
            //     ui.add_space(24.0);
            //     ui.label("Prefer records of filetype:");
            //     if self.extensions.selected.is_empty() {
            //         self.extensions.selected = db.file_extensions[0].clone();
            //     }
            //     // if db.size > 0 && db.file_extensions.is_empty() {
            //     //     db.file_extensions = get_extensions(&db.pool).await;
            //     // }
            //     if db.file_extensions.len() > 1 {
            //         combo_box(
            //             ui,
            //             "Extensions",
            //             &mut self.extensions.selected,
            //             &db.file_extensions,
            //         );
            //     } else {
            //         ui.label(&self.extensions.selected);
            //     }
            // });

            // ui.horizontal(|ui| {
            //     if self.extensions.working {
            //         ui.spinner();
            //     } else {
            //         ui.add_space(24.0)
            //     }
            //     ui.label(RichText::new(self.extensions.status.clone()).strong());
            // });

            // if self.extensions.working {
            //     ui.add(
            //         egui::ProgressBar::new(self.extensions.progress.0 / self.extensions.progress.1)
            //             // .text("progress")
            //             .desired_height(4.0),
            //     );
            //     ui.label(format!(
            //         "Progress: {} / {}",
            //         self.extensions.progress.0, self.extensions.progress.1
            //     ));
            // }

            // ui.separator();

            //COMPARE COMPARE COMPARE COMPARE
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.compare.search, "Compare against database: ");
                if let Some(cdb) = &self.c_db {
                    ui.label(&cdb.name);
                }

                button(ui, "Select DB", || {
                    let tx = self.cdb_io.tx.clone().expect("tx channel exists");
                    tokio::spawn(async move {
                        let db = open_db().await.unwrap();
                        let _ = tx.send(db).await;
                    });
                });
                if let Some(rx) = self.cdb_io.rx.as_mut() {
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
                if self.compare.working {
                    ui.spinner();
                } else {
                    ui.add_space(24.0)
                }
                ui.label(RichText::new(self.compare.status.clone()).strong());
            });
            ui.separator();

            // DELETION PREFERENCES
            empty_line(ui);
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.safe, "Create Safety Database of Thinned Records");
                if !&self.safe {
                    ui.label(
                        RichText::new("UNSAFE!")
                            .color(egui::Color32::from_rgb(255, 0, 0))
                            .strong(),
                    );
                    ui.label(RichText::new("Will remove records from current database").strong());
                }
            });
            ui.checkbox(&mut self.dupes_db, "Create Database of Duplicate Records");
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.remove_files, "Remove Dupicate Files?");
                enum_combo_box2(ui, &mut self.delete_action);
                if self.remove_files && self.delete_action == Delete::Permanent {
                    ui.label(
                        RichText::new("UNSAFE!")
                            .color(egui::Color32::from_rgb(255, 0, 0))
                            .strong(),
                    );
                    ui.label(RichText::new("This is NOT undoable").strong());
                }
            });

            empty_line(ui);
            ui.separator();

            ui.horizontal(|_ui| {});

            ui.horizontal(|ui| {
                if handles_active(self) {
                    self.go_replace = false;
                    button(ui, "Cancel", || abort_all(self));
                } else {
                    self.go_replace = true;

                    if ui.input(|i| i.modifiers.alt) {
                        rt_button(ui, RichText::new("Search and Remove Duplicates").size(16.0).strong(), || {
                            self.go_search = true;
                            self.go_replace = false;
                            gather_duplicates(self);
                        });
                    } else {
                        rt_button(ui, RichText::new("Search for Duplicates").size(16.0), || gather_duplicates(self));
                    }
                }
                if !self.main.records.is_empty() && !handles_active(self) {
                    self.main.status = format!(
                        "{} total records marked for removal",
                        self.main.records.len()
                    );
                    
                    if ui.button(RichText::new("Remove Duplicates").strong().size(16.0)).clicked() {
                        remove_duplicates(self);
                    }
                }

                if self.go_replace && self.go_search {
                    self.go_replace = false;
                    self.go_search = false;
                    remove_duplicates(self);
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
                && !handles_active(self)
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
            receive_async_data(self);
        } else {
            ui.heading(RichText::new("No Open Database").weak());
        }
    }
    fn order_panel(&mut self, ui: &mut egui::Ui) {
        // if self.help {
        //     order_help(ui)
        // }
        ui.heading(RichText::new("Duplicate Filename Preservation Priority").strong());
        ui.label("or... How to decide which file to keep when duplicates are found");
        ui.label("Entries at the top of list take precedence to those below");
        empty_line(ui);
        // ui.separator();
        order_toolbar2(ui, self);
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
                                    // for (index, line) in self.main.list.iter_mut().enumerate() {
                                    for (index, line) in self.order_friendly.iter_mut().enumerate()
                                    {
                                        let checked = self.sel_line == Some(index);
                                        if ui
                                            .selectable_label(
                                                checked,
                                                RichText::new(format!(
                                                    "{:02} : {}",
                                                    index + 1,
                                                    line.clone()
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

                order_toolbar(ui, self);
            },
        );

        // ui.separator();
    }

    fn order_text_panel(&mut self, ui: &mut egui::Ui) {
        // if self.help {
        //     order_help(ui)
        // }

        ui.columns(1, |columns| {
            // columns[0].heading("Duplicate Filename Keeper Priority Order:");
            columns[0].text_edit_multiline(&mut self.order_text);
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("Save").clicked() {
                self.main.list = self.order_text.lines().map(|s| s.to_string()).collect();
                self.my_panel = Panel::Order;
            }
            if ui.button("Cancel").clicked() {
                #[cfg(debug_assertions)]
                {
                    if ui.input(|i| i.modifiers.alt)
                        && ui.input(|i| i.modifiers.command)
                        && ui.input(|i| i.modifiers.shift)
                        && ui.input(|i| i.modifiers.ctrl)
                    {
                        self.my_panel = Panel::KeyGen;
                        return;
                    }
                }
                self.my_panel = Panel::Order;
            }
        });
    }
    fn tags_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading(RichText::new("Tag Editor").strong());
        ui.label("Protools Audiosuite Tags use the following format:  -example_");
        ui.label("You can enter any string of text and if it is a match, the file will be marked for removal");
        empty_line(ui);
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

                        if ui
                            .selectable_label(is_selected, RichText::new(tag.clone()).size(14.0))
                            .clicked()
                        {
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
            ui.separator();
            empty_line(ui);
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
        });
    }

    fn keygen_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Name: ");
            ui.text_edit_singleline(&mut self.registered.name);
        });
        ui.horizontal(|ui| {
            ui.label("Email: ");
            ui.text_edit_singleline(&mut self.registered.email);
        });
        self.registered.key = generate_license_key(&self.registered.name, &self.registered.email);
        ui.horizontal(|ui| {
            ui.label("License Key: ");
            ui.label(&self.registered.key);
            // ui.text_edit_singleline(&mut self.registered.key);
        });
    }

    // fn panel_chunk<F>(&mut self, ui: &mut egui::Ui, display: F, config: &Config)
    // where
    // F: FnOnce(),
    // {
    //     display();

    //     ui.horizontal( |ui| {
    //         if config.working {ui.spinner();}
    //         else {ui.add_space(24.0)}
    //         ui.label(RichText::new(config.status.clone()).strong());

    //     });

    //     if config.working{
    //         ui.add( egui::ProgressBar::new(config.progress.0 / config.progress.1)
    //                 // .text("progress")
    //                 .desired_height(4.0)
    //             );
    //         ui.label(format!("Progress: {} / {}", config.progress.0, config.progress.1));
    //     }
    //     ui.separator();
    // }
}

// pub async fn records_window_display(ctx: &egui::Context, app: &mut App) {
//     egui::Window::new("Records Marked for Duplication")
//         .open(&mut app.records_window) // Control whether the window is open
//         .show(ctx, |ui| {
//             ui.label("To Be Implemented");
//             let width = ui.available_width();
//             let height = ui.available_height();

//             egui::ScrollArea::vertical()
//                 .max_height(height) // Set the maximum height of the scroll area
//                 .show(ui, |ui| {
//                     ui.horizontal(|ui| {
//                         ui.set_min_width(width);

//                         egui::Grid::new("Dupe Records")
//                             .spacing([20.0, 8.0])
//                             .striped(true)
//                             .show(ui, |ui| {
//                                 // Use par_iter() to process in parallel
//                                 app.marked_records.par_iter().for_each(|filename| {
//                                     ui.label(filename); // Display each filename
//                                     ui.end_row(); // Move to the next row
//                                 });
//                             });
//                     });
//                 });
//         });
// }

pub fn order_toolbar(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        if ui.button("Move Up").clicked() {
            if let Some(index) = app.sel_line {
                if index > 0 {
                    app.sel_line = Some(index - 1);
                    app.main.list.swap(index, index - 1);
                    app.order_friendly.swap(index, index - 1);
                }
            }
        }
        if ui.button("Move Down").clicked() {
            if let Some(index) = app.sel_line {
                if index < app.main.list.len() - 1 {
                    app.sel_line = Some(index + 1);
                    app.main.list.swap(index, index + 1);
                    app.order_friendly.swap(index, index + 1);
                }
            }
        }
        if ui.button("Remove").clicked() {
            if let Some(index) = app.sel_line {
                app.main.list.remove(index);
                app.order_friendly.remove(index);
                app.sel_line = None;
            }
        }

        if ui.input(|i| i.modifiers.alt) && ui.button("Text Editor").clicked() {
            app.order_text = app.main.list.join("\n");
            app.my_panel = Panel::OrderText;
        }
        // if ui.button("Add Line:").clicked && !app.new_line.is_empty() {
        //     app.main.list.insert(0, app.new_line.clone());
        //     app.new_line.clear();
        // }

        // ui.text_edit_singleline(&mut app.new_line);
        // if ui.button("Help").clicked {
        //     app.help = !app.help
        // }
    });
}
pub fn order_toolbar2(ui: &mut egui::Ui, app: &mut App) {
    ui.horizontal(|ui| {
        // ui.label("Add Line: ");
        if let Some(db) = &app.db.clone() {
            combo_box(ui, "order_column", &mut app.order_column, &db.columns);
        } else {
            combo_box(
                ui,
                "order_column",
                &mut "no database".to_string(),
                &app.group.list,
            );
        }
        enum_combo_box(ui, &mut app.order_operator);
        match app.order_operator {
            OrderOperator::Largest
            | OrderOperator::Smallest
            | OrderOperator::IsEmpty
            | OrderOperator::IsNotEmpty => {}
            _ => {
                ui.text_edit_singleline(&mut app.order_input);
            }
        }

        if ui.button("Add Line").clicked {
            match app.order_operator {
                OrderOperator::Largest
                | OrderOperator::Smallest
                | OrderOperator::IsEmpty
                | OrderOperator::IsNotEmpty => {
                    app.main.list.insert(
                        0,
                        parse_to_sql(
                            app.order_column.clone(),
                            app.order_operator,
                            app.order_input.clone(),
                        ),
                    );
                    app.order_friendly.insert(
                        0,
                        parse_to_user_friendly(
                            app.order_column.clone(),
                            app.order_operator,
                            app.order_input.clone(),
                        ),
                    );
                    app.order_input.clear();
                }
                _ => {
                    if !app.order_input.is_empty() {
                        app.main.list.insert(
                            0,
                            parse_to_sql(
                                app.order_column.clone(),
                                app.order_operator,
                                app.order_input.clone(),
                            ),
                        );
                        app.order_friendly.insert(
                            0,
                            parse_to_user_friendly(
                                app.order_column.clone(),
                                app.order_operator,
                                app.order_input.clone(),
                            ),
                        );
                        app.order_input.clear();
                    }
                }
            }
        }
        // if ui.button("Help").clicked {
        //     app.help = !app.help
        // }
    });
  
}

pub fn gather_duplicates(app: &mut App) {
    abort_all(app);
    app.main.records.clear();
    if let Some(db) = app.db.clone() {
        let pool = db.pool.clone();

      
        app.main.status = "Searching for Duplicates".to_string();
       

        if app.main.search {
            let pool = pool.clone();
            let order = app.main.list.clone();
            let mut group_sort = None;
            if app.group.search {
                group_sort = Some(app.group.selected.clone())
            }
           
            let group_null = app.group_null;
            wrap_async(
                &mut app.group,
                "Searching For Duplicate Filenames",
                move || gather_duplicate_filenames_in_database(pool, order, group_sort, group_null),
            )
        }

        if app.deep.search {
            if let Some(sender) = app.deep.progress_io.tx.clone() {
                if let Some(sender2) = app.deep.status_io.tx.clone() {
                    let pool = pool.clone();
                    let ignore = app.ignore_extension;
                    wrap_async(
                        &mut app.deep,
                        "Searching for Duplicates with similar Filenames",
                        move || gather_deep_dive_records(pool, sender, sender2, ignore),
                    )
                }
            }
        }

        if app.tags.search {
            let pool = pool.clone();
            let tags = app.tags.list.clone();
            wrap_async(
                &mut app.tags,
                "Searching for Filenames with Specified Tags",
                move || gather_filenames_with_tags(pool, tags),
            );
        }

        if app.compare.search && app.c_db.is_some() {
            if let Some(cdb) = &app.c_db {
                app.compare.working = true;
                app.compare.status = format!("Comparing against {}", cdb.name);
                if app.compare.records_io.tx.is_none() {
                    println!("compare tx is none");
                }
                if let Some(tx) = app.compare.records_io.tx.clone() {
                    println!("if let some");
                    let p = pool.clone();
                    let c_pool = cdb.pool.clone();
                    let handle = tokio::spawn(async move {
                        println!("tokio spawn compare");
                        let results = gather_compare_database_overlaps(&p, &c_pool).await;
                        if (tx.send(results.expect("error on compare db")).await).is_err() {
                            eprintln!("Failed to send db");
                        }
                    });
                    app.compare.handle = Some(handle);
                }
            }
        }
    }
}

fn receive_async_data(app: &mut App) {
    if let Some(records) = app.main.receive_hashset() {
        app.clear_status();
        app.main.status = format! {"Removed {} duplicates", records.len()};
        // app.main.records.clear();
    }

    if let Some(records) = app.group.receive_hashset() {
        app.main.records.extend(records);
    }

    if let Some(records) = app.deep.receive_hashset() {
        app.main.records.extend(records);
    }

    if let Some(records) = app.tags.receive_hashset() {
        app.main.records.extend(records);
    }

    if let Some(records) = app.compare.receive_hashset() {
        app.main.records.extend(records);
    }

    app.main.receive_progress();
    app.main.receive_status();
    app.deep.receive_progress();
    app.deep.receive_status();
}

fn remove_duplicates(app: &mut App) {
    if app.registered.valid == Some(false) {
        app.main.records.clear();
        app.main.status = "Unregistered!\nPlease Register to Remove Duplicates".to_string();
        return;
    }
    if let Some(db) = app.db.clone() {
        let mut work_db_path: Option<String> = Some(db.path.clone());
        let mut duplicate_db_path: Option<String> = None;
        let records = app.main.records.clone();

        app.main.working = true;
        if app.safe {
            app.main.status = "Creating Safety Database".to_string();
            let path = format!("{}_thinned.sqlite", &db.path.trim_end_matches(".sqlite"));
            let _result = fs::copy(&db.path, &path);
            work_db_path = Some(path);
        }
        if app.dupes_db {
            app.main.status = "Creating Database of Duplicates".to_string();
            let path = format!("{}_dupes.sqlite", &db.path.trim_end_matches(".sqlite"));
            let _result = fs::copy(&db.path, &path);
            duplicate_db_path = Some(path);
        }

        if let Some(sender) = app.main.progress_io.tx.clone() {
            if let Some(sender2) = app.main.status_io.tx.clone() {
                wrap_async(&mut app.main, "Performing Record Removal", move || {
                    remove_duplicates_go(records, work_db_path, duplicate_db_path, sender, sender2)
                })
            }
        }
        if app.remove_files {
            println!("Removing Files");
            let files: HashSet<&str> = app
                .main
                .records
                .par_iter()
                .map(|record| record.path.as_str())
                .collect();

            let _ = app.delete_action.delete_files(files);
            // for path in files {
            //     app.delete_action
            //         .delete_file(path)
            //         .expect("all files should be valid")
            // }
        }
    }
}

pub async fn remove_duplicates_go(
    records: HashSet<FileRecord>,
    main_db_path: Option<String>,
    dupe_db_path: Option<String>,
    sender: mpsc::Sender<ProgressMessage>,
    sender2: mpsc::Sender<String>,
) -> Result<HashSet<FileRecord>, sqlx::Error> {
    if let Some(main_path) = &main_db_path {
        let main_db = Database::open(main_path).await;
        let _result =
            delete_file_records(&main_db.pool, &records, sender.clone(), sender2.clone()).await;
        if let Some(path) = dupe_db_path {
            let dupes_db = Database::open(&path).await;
            let _result =
                create_duplicates_db(&dupes_db.pool, &records, sender.clone(), sender2.clone())
                    .await;
        }
    }
    Ok(records)
}

fn abort_all(app: &mut App) {
    app.main.abort();
    app.group.abort();
    app.deep.abort();
    app.tags.abort();
    app.compare.abort();
}

fn handles_active(app: &App) -> bool {
    app.main.handle.is_some()
        || app.group.handle.is_some()
        || app.deep.handle.is_some()
        || app.tags.handle.is_some()
        || app.compare.handle.is_some()
}

fn enum_combo_box(ui: &mut egui::Ui, selected_variant: &mut OrderOperator) {
    egui::ComboBox::from_id_source("variants")
        .selected_text(selected_variant.as_str())
        .show_ui(ui, |ui| {
            for variant in OrderOperator::variants() {
                ui.selectable_value(selected_variant, *variant, variant.as_str());
            }
        });
}
fn enum_combo_box2(ui: &mut egui::Ui, selected_variant: &mut Delete) {
    egui::ComboBox::from_id_source("variants")
        .selected_text(selected_variant.as_str())
        .show_ui(ui, |ui| {
            for variant in Delete::variants() {
                ui.selectable_value(selected_variant, *variant, variant.as_str());
            }
        });
}

pub fn parse_to_sql(column: String, operator: OrderOperator, input: String) -> String {
    match operator {
        OrderOperator::Largest => format! {"{} DESC", column.to_lowercase()},
        OrderOperator::Smallest => format!("{} ASC", column.to_lowercase()),
        OrderOperator::Is => format!(
            "CASE WHEN {} IS '%{}%' THEN 0 ELSE 1 END ASC",
            column.to_lowercase(),
            input
        ),
        OrderOperator::IsNot => format!(
            "CASE WHEN {} IS '%{}%' THEN 1 ELSE 0 END ASC",
            column.to_lowercase(),
            input
        ),
        OrderOperator::Contains => format!(
            "CASE WHEN {} LIKE '%{}%' THEN 0 ELSE 1 END ASC",
            column.to_lowercase(),
            input
        ),
        OrderOperator::DoesNotContain => format!(
            "CASE WHEN {} LIKE '%{}%' THEN 1 ELSE 0 END ASC",
            column.to_lowercase(),
            input
        ),
        OrderOperator::IsEmpty => format!(
            "CASE WHEN {} IS NOT NULL AND {} != '' THEN 1 ELSE 0 END ASC",
            column.to_lowercase(),
            column.to_lowercase()
        ),
        OrderOperator::IsNotEmpty => format!(
            "CASE WHEN {} IS NOT NULL AND {} != '' THEN 0 ELSE 1 END ASC",
            column.to_lowercase(),
            column.to_lowercase()
        ),
    }
}

pub fn parse_to_user_friendly(column: String, operator: OrderOperator, input: String) -> String {
    match operator {
        OrderOperator::Largest => format! {"Largest {}", column},
        OrderOperator::Smallest => format!("Smallest {} ", column),
        OrderOperator::Is => format!("{} is '{}'", column, input),
        OrderOperator::IsNot => format!("{} is NOT '{}'", column, input),
        OrderOperator::Contains => format!("{} contains '{}'", column, input),
        OrderOperator::DoesNotContain => format!("{} does NOT contain '{}'", column, input),
        OrderOperator::IsEmpty => format!("{} is empty", column,),
        OrderOperator::IsNotEmpty => format!("{} is NOT empty", column,),
    }
}
