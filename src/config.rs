use crate::prelude::*;

use crate::assets::*;
use crate::processing::*;

use dirs::home_dir;
use rfd::FileDialog;
use sqlx::sqlite::SqliteRow;

use std::fs::{self};
use std::hash::Hash;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct AsyncTunnel<T> {
    #[serde(skip)]
    pub tx: mpsc::Sender<T>,
    #[serde(skip)]
    pub rx: mpsc::Receiver<T>,
    #[serde(skip)]
    pub waiting: bool,
}

impl<T> Default for AsyncTunnel<T> {
    fn default() -> Self {
        AsyncTunnel::new(1)
    }
}

impl<T> AsyncTunnel<T> {
    pub fn new(channels: usize) -> AsyncTunnel<T> {
        let (tx, rx) = mpsc::channel(channels);
        AsyncTunnel {
            tx,
            rx,
            waiting: false,
        }
    }

    pub async fn send(&mut self, item: T) -> Result<(), mpsc::error::SendError<T>> {
        self.waiting = true;
        self.tx.send(item).await
    }

    pub fn recv(&mut self) -> Option<T> {
        self.rx.try_recv().ok()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]

pub struct NodeConfig {
    pub enabled: bool,
    #[serde(skip)]
    pub working: bool,

    #[serde(skip)]
    pub status: Arc<str>,
    #[serde(skip)]
    pub status_io: AsyncTunnel<Arc<str>>,
    #[serde(skip)]
    pub records: HashSet<FileRecord>,
    #[serde(skip)]
    pub records_io: AsyncTunnel<HashSet<FileRecord>>,
    #[serde(skip)]
    pub progress: (f32, f32),
    #[serde(skip)]
    pub progress_io: AsyncTunnel<ProgressMessage>,
    #[serde(skip)]
    pub handle: Option<tokio::task::JoinHandle<()>>,
}

impl Clone for NodeConfig {
    fn clone(&self) -> Self {
        NodeConfig {
            records: self.records.clone(),
            status: self.status.clone(),
            enabled: self.enabled,
            working: self.working,
            progress: self.progress,
            ..Default::default()
        }
    }
}
impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            status: Arc::from(""),
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

impl NodeConfig {
    pub fn new(on: bool) -> Self {
        Self {
            enabled: on,
            ..Default::default()
        }
    }
    pub fn abort(&mut self) -> Self {
        let enabled = self.enabled;
        if let Some(handle) = &self.handle {
            handle.abort();
        }
        Self {
            enabled,
            ..Default::default()
        }
    }

    pub fn receive_hashset(&mut self) -> Option<HashSet<FileRecord>> {
        if let Some(records) = self.records_io.recv() {
            self.records = records.clone();
            self.handle = None;
            self.working = false;
            self.progress = (0.0, 0.0);
            self.status = format! {"Found {} duplicate records", self.records.len()}.into();
            return Some(records);
        }
        None
    }

    pub fn receive_progress(&mut self) {
        while let Some(ProgressMessage::Update(current, total)) = self.progress_io.recv() {
            self.progress = (current as f32, total as f32);
        }
    }

    pub fn receive_status(&mut self) {
        while let Some(message) = self.status_io.recv() {
            self.status = message;
        }
    }
    pub fn receive(&mut self) -> Option<HashSet<FileRecord>> {
        self.receive_progress();
        self.receive_status();
        self.receive_hashset()
    }

    pub fn render_progress_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.working {
                ui.spinner();
            } else {
                ui.add_space(24.0)
            }
            ui.label(RichText::new(&*self.status).strong());
            if self.working {
                ui.label(format!(
                    "Progress: {} / {}",
                    self.progress.0, self.progress.1
                ));
            }
        });

        if self.working {
            ui.add(
                egui::ProgressBar::new(self.progress.0 / self.progress.1)
                    // .text("progress")
                    .desired_height(4.0),
            );
        } else {
            // ui.separator();
        }
        empty_line(ui);
    }

    pub fn wrap_async<F, T>(&mut self, action: F)
    where
        F: FnOnce() -> T + Send + 'static,
        T: std::future::Future<Output = Result<HashSet<FileRecord>, sqlx::Error>> + Send + 'static,
    {
        self.working = true;
        let tx = self.records_io.tx.clone();

        let handle = tokio::spawn(async move {
            let results = action().await;
            if (tx.send(results.expect("Tokio Results Error HashSet")).await).is_err() {
                eprintln!("Failed to send db");
            }
        });
        self.handle = Some(handle);
    }
}

// #[derive(Default)]
pub struct Database {
    pub path: String,
    pool: Option<SqlitePool>,
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

        Self {
            path: db_path.to_string(),
            pool: Some(db_pool),
            name: db_path
                .split('/')
                .last()
                .expect("Name From Pathname")
                .to_string(),
            size: db_size,
            columns: db_columns,
            file_extensions: Vec::new(),
            // io: AsyncTunnel::new(1),
        }
    }
    pub async fn get_size(&self) -> Result<usize, sqlx::Error> {
        let pool = self.pool.as_ref().unwrap();
        let count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", TABLE))
            .fetch_one(pool)
            .await?;

        Ok(count.0 as usize)
    }

    pub fn get_extensions(&mut self, tx: mpsc::Sender<Vec<String>>) {
        let Some(pool) = self.pool() else {
            return;
        };
        let tx = tx.clone();
        let _handle = tokio::spawn(async move {
            let results = get_audio_file_types(&pool).await;

            if (tx.send(results.expect("Tokio Results Error HashSet")).await).is_err() {
                eprintln!("Failed to send db while gathering extensions");
            }
        });
    }

    pub fn pool(&self) -> Option<SqlitePool> {
        self.pool.clone()
    }

    pub async fn fetch_filerecords(&self, query: &str) -> Result<HashSet<FileRecord>, sqlx::Error> {
        let mut file_records = HashSet::new();
        if let Some(pool) = self.pool.as_ref() {
            let rows = sqlx::query(query).fetch_all(pool).await?;

            for row in rows {
                file_records.insert(FileRecord::new(&row));
            }
        }
        Ok(file_records)
    }
    pub async fn fetch_all_filerecords(&self) -> Result<HashSet<FileRecord>, sqlx::Error> {
        println!("Gathering all records from database");
        self.fetch_filerecords(&format!(
            "SELECT rowid, filename, duration, filepath FROM {}",
            TABLE
        ))
        .await
    }
}

pub async fn open_db() -> Option<Database> {
    let home_dir = home_dir();
    match home_dir {
        Some(home_dir) => {
            println!("Found SMDB dir");
            let db_dir = home_dir.join("Library/Application Support/SoundminerV6/Databases");
            if let Some(path) = FileDialog::new()
                .add_filter("SQLite Database", &["sqlite"])
                .set_directory(db_dir)
                .pick_file()
            {
                let db_path = path.display().to_string();
                if db_path.ends_with(".sqlite") {
                    println!("Opening Database {}", db_path);
                    let db = Database::open(&db_path).await;
                    return Some(db);
                }
            }
        }
        None => {
            println!("did not find SMDB dir");
            if let Some(path) = FileDialog::new()
                .add_filter("SQLite Database", &["sqlite"])
                .pick_file()
            {
                let db_path = path.display().to_string();
                if db_path.ends_with(".sqlite") {
                    println!("Opening Database {}", db_path);
                    let db = Database::open(&db_path).await;
                    return Some(db);
                }
            }
        }
    }
    None
}

pub async fn get_db_size(pool: &SqlitePool) -> Result<usize, sqlx::Error> {
    let count: (i64,) = sqlx::query_as(&format!("SELECT COUNT(*) FROM {}", TABLE))
        .fetch_one(pool)
        .await?;

    Ok(count.0 as usize)
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct FileRecord {
    pub id: usize,
    pub filename: Arc<str>,
    pub duration: Arc<str>,
    pub path: Arc<str>,
}

impl FileRecord {
    pub fn new(row: &SqliteRow) -> Self {
        let id: u32 = row.get(0);
        let filename: &str = row.get(1);
        let duration = row.try_get(2).unwrap_or("");
        let path: &str = row.get(3);
        Self {
            id: id as usize,
            filename: filename.into(),
            duration: duration.into(),
            path: path.into(),
        }
    }
    pub async fn fetch_filerecords_from_database(
        pool: &SqlitePool,
        query: &str,
    ) -> Result<HashSet<FileRecord>, sqlx::Error> {
        let mut file_records = HashSet::new();

        let rows = sqlx::query(query).fetch_all(pool).await?;

        for row in rows {
            file_records.insert(FileRecord::new(&row));
        }
        Ok(file_records)
    }
    pub async fn fetch_all_filerecords_from_database(
        pool: &SqlitePool,
    ) -> Result<HashSet<FileRecord>, sqlx::Error> {
        println!("Gathering all records from database");
        fetch_filerecords_from_database(
            pool,
            &format!("SELECT rowid, filename, duration, filepath FROM {}", TABLE),
        )
        .await
    }
}

pub enum ProgressMessage {
    Update(usize, usize), // (current, total)
}

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy)]
pub enum Panel {
    Duplicates,
    NewDuplicates,
    Order,
    Tags,
    Find,
    KeyGen,
}

#[derive(PartialEq, serde::Serialize, Deserialize, Clone, Copy, Default)]
pub enum OrderOperator {
    Largest,
    Smallest,
    #[default]
    Contains,
    DoesNotContain,
    Is,
    IsNot,
    IsEmpty,
    IsNotEmpty,
}

impl EnumComboBox for OrderOperator {
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

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct PreservationLogic {
    pub column: String,
    pub operator: OrderOperator,
    pub variable: String,
    // pub friendly: String,
    // pub sql: String,
}

impl PreservationLogic {
    pub fn get_sql(&self) -> String {
        match self.operator {
            OrderOperator::Largest => format! {"{} DESC", self.column.to_lowercase()},
            OrderOperator::Smallest => format!("{} ASC", self.column.to_lowercase()),
            OrderOperator::Is => format!(
                "CASE WHEN {} IS '%{}%' THEN 0 ELSE 1 END ASC",
                self.column, self.variable,
            ),
            OrderOperator::IsNot => format!(
                "CASE WHEN {} IS '%{}%' THEN 1 ELSE 0 END ASC",
                self.column, self.variable
            ),
            OrderOperator::Contains => format!(
                "CASE WHEN {} LIKE '%{}%' THEN 0 ELSE 1 END ASC",
                self.column, self.variable
            ),
            OrderOperator::DoesNotContain => format!(
                "CASE WHEN {} LIKE '%{}%' THEN 1 ELSE 0 END ASC",
                self.column, self.variable
            ),
            OrderOperator::IsEmpty => format!(
                "CASE WHEN {} IS NOT NULL AND {} != '' THEN 1 ELSE 0 END ASC",
                self.column, self.column
            ),
            OrderOperator::IsNotEmpty => format!(
                "CASE WHEN {} IS NOT NULL AND {} != '' THEN 0 ELSE 1 END ASC",
                self.column, self.column
            ),
        }
    }
    pub fn get_friendly(&self) -> String {
        match self.operator {
            OrderOperator::Largest => format! {"Largest {}", self.column},
            OrderOperator::Smallest => format!("Smallest {} ", self.column),
            OrderOperator::Is => format!("{} is '{}'", self.column, self.variable),
            OrderOperator::IsNot => format!("{} is NOT '{}'", self.column, self.variable),
            OrderOperator::Contains => format!("{} contains '{}'", self.column, self.variable),
            OrderOperator::DoesNotContain => {
                format!("{} does NOT contain '{}'", self.column, self.variable)
            }
            OrderOperator::IsEmpty => format!("{} is empty", self.column,),
            OrderOperator::IsNotEmpty => format!("{} is NOT empty", self.column,),
        }
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

pub fn default_tags() -> Vec<String> {
    const DEFAULT_TAGS_VEC: [&str; 43] = [
        "-1eqa_",
        "-6030_",
        "-7eqa_",
        "-A2sA_",
        "-A44m_",
        "-A44s_",
        "-Alt7S_",
        "-ASMA_",
        "-AVrP_",
        "-AVrT_",
        "-AVSt_",
        "-DEC4_",
        "-Delays_",
        "-Dn_",
        "-DUPL_",
        "-DVerb_",
        "-GAIN_",
        "-M2DN_",
        "-NORM_",
        "-NYCT_",
        "-PiSh_",
        "-PnT2_",
        "-PnTPro_",
        "-ProQ2_",
        "-PSh_",
        "-RVRS_",
        "-RX7Cnct_",
        "-spce_",
        "-TCEX_",
        "-TiSh_",
        "-TmShft_",
        "-VariFi_",
        "-VlhllVV_",
        "-VSPD_",
        "-VitmnMn_",
        "-VtmnStr_",
        "-X2mA_",
        "-X2sA_",
        "-XForm_",
        "-Z2N5_",
        "-Z2S5_",
        "-Z4n2_",
        "-ZXN5_",
    ];

    DEFAULT_TAGS_VEC.map(|s| s.to_string()).to_vec()
}

pub fn tjf_tags() -> Vec<String> {
    const TJF_TAGS_VEC: [&str; 49] = [
        "-1eqa_",
        "-6030_",
        "-7eqa_",
        "-A2sA_",
        "-A44m_",
        "-A44s_",
        "-Alt7S_",
        "-ASMA_",
        "-AVrP_",
        "-AVrT_",
        "-AVSt_",
        "-DEC4_",
        "-Delays_",
        "-Dn_",
        "-DUPL_",
        "-DVerb_",
        "-GAIN_",
        "-M2DN_",
        "-NORM_",
        "-NYCT_",
        "-PiSh_",
        "-PnT2_",
        "-PnTPro_",
        "-ProQ2_",
        "-PSh_",
        "-Reverse_",
        "-RVRS_",
        "-RING_",
        "-RX7Cnct_",
        "-spce_",
        "-TCEX_",
        "-TiSh_",
        "-TmShft_",
        "-VariFi_",
        "-VlhllVV_",
        "-VSPD_",
        "-VitmnMn_",
        "-VtmnStr_",
        "-X2mA_",
        "-X2sA_",
        "-XForm_",
        "-Z2N5_",
        "-Z2S5_",
        "-Z4n2_",
        "-ZXN5_",
        ".new.",
        ".aif.",
        ".mp3.",
        ".wav.",
    ];
    TJF_TAGS_VEC.map(|s| s.to_string()).to_vec()
}

pub fn default_order() -> Vec<PreservationLogic> {
    vec![
        PreservationLogic {
            column: String::from("Description"),
            operator: OrderOperator::IsNotEmpty,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("Audio Files"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARIES"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("/LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARY/"),
        },
        PreservationLogic {
            column: String::from("Duration"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Channels"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("SampleRate"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BitDepth"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BWDate"),
            operator: OrderOperator::Smallest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("ScannedDate"),
            operator: OrderOperator::Smallest,
            variable: String::new(),
        },
    ]
}

pub fn tjf_order() -> Vec<PreservationLogic> {
    vec![
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("TJF RECORDINGS"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARIES"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("SHOWS/Tim Farrell"),
        },
        PreservationLogic {
            column: String::from("Description"),
            operator: OrderOperator::IsNotEmpty,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("Audio Files"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("RECORD"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("CREATED SFX"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("CREATED FX"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("/LIBRARY"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("LIBRARY/"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("SIGNATURE"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::Contains,
            variable: String::from("PULLS"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("EDIT"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("MIX"),
        },
        PreservationLogic {
            column: String::from("Pathname"),
            operator: OrderOperator::DoesNotContain,
            variable: String::from("SESSION"),
        },
        PreservationLogic {
            column: String::from("Duration"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("Channels"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("SampleRate"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BitDepth"),
            operator: OrderOperator::Largest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("BWDate"),
            operator: OrderOperator::Smallest,
            variable: String::new(),
        },
        PreservationLogic {
            column: String::from("ScannedDate"),
            operator: OrderOperator::Smallest,
            variable: String::new(),
        },
    ]
}

// pub fn default_order() -> Vec<String> {
//     const DEFAULT_ORDER_VEC: [&str; 12] = [
//         "CASE WHEN Description IS NOT NULL AND Description != '' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%Audio Files%' THEN 1 ELSE 0 END ASC",
//         "CASE WHEN pathname LIKE '%LIBRARIES%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%LIBRARY%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%/LIBRARY%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%LIBRARY/%' THEN 0 ELSE 1 END ASC",
//         "duration DESC",
//         "channels DESC",
//         "sampleRate DESC",
//         "bitDepth DESC",
//         "BWDate ASC",
//         "scannedDate ASC",
//     ];
//     DEFAULT_ORDER_VEC.map(|s| s.to_string()).to_vec()
// }
// pub fn default_order_friendly() -> Vec<String> {
//     const DEFAULT_ORDER_FRIENDLY: [&str; 12] = [
//         "Description is NOT Empty",
//         "Pathname does NOT contain 'Audio Files'",
//         "Pathname contains 'LIBRARIES'",
//         "Pathname contains 'LIBRARY'",
//         "Pathname contains '/LIBRARY'",
//         "Pathname contains 'LIBRARY/'",
//         "Largest Duration",
//         "Largest Channel Count",
//         "Largest Sample Rate",
//         "Largest Bit Depth",
//         "Smallest BWDate",
//         "Smallest Scanned Date",
//     ];
//     DEFAULT_ORDER_FRIENDLY.map(|s| s.to_string()).to_vec()
// }

// pub fn tjf_order() -> Vec<String> {
//     const TJF_ORDER_VEC: [&str; 22] = [
//         "CASE WHEN pathname LIKE '%TJF RECORDINGS%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%LIBRARIES%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%SHOWS/Tim Farrell%' THEN 1 ELSE 0 END ASC",
//         "CASE WHEN Description IS NOT NULL AND Description != '' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%Audio Files%' THEN 1 ELSE 0 END ASC",
//         "CASE WHEN pathname LIKE '%RECORD%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%CREATED SFX%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%CREATED FX%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%LIBRARY%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%/LIBRARY%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%LIBRARY/%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%SIGNATURE%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%PULLS%' THEN 0 ELSE 1 END ASC",
//         "CASE WHEN pathname LIKE '%EDIT%' THEN 1 ELSE 0 END ASC",
//         "CASE WHEN pathname LIKE '%MIX%' THEN 1 ELSE 0 END ASC",
//         "CASE WHEN pathname LIKE '%SESSION%' THEN 1 ELSE 0 END ASC",
//         "duration DESC",
//         "channels DESC",
//         "sampleRate DESC",
//         "bitDepth DESC",
//         "BWDate ASC",
//         "scannedDate ASC",
//     ];
//     TJF_ORDER_VEC.map(|s| s.to_string()).to_vec()
// }

// pub fn tjf_order_friendly() -> Vec<String> {
//     const TJF_ORDER_FRIENDLY: [&str; 22] = [
//         "Pathname contains 'TJF RECORDINGS'",
//         "Pathname contains 'LIBRARIES'",
//         "Pathname does NOT contain 'SHOWS/Tim Farrell'",
//         "Description is NOT Empty",
//         "Pathname does NOT contain 'Audio Files'",
//         "Pathname contains 'RECORD'",
//         "Pathname contains 'CREATED SFX'",
//         "Pathname contains 'CREATED FX'",
//         "Pathname contains 'LIBRARY'",
//         "Pathname contains '/LIBRARY'",
//         "Pathname contains 'LIBRARY/'",
//         "Pathname contains 'SIGNATURE'",
//         "Pathname contains 'PULLS'",
//         "Pathname does NOT contain 'EDIT'",
//         "Pathname does NOT contain 'MIX'",
//         "Pathname does NOT contain 'SESSION'",
//         "Largest Duration",
//         "Largest Channel Count",
//         "Largest Sample Rate",
//         "Largest Bit Depth",
//         "Smallest BWDate",
//         "Smallest Scanned Date",
//     ];
//     TJF_ORDER_FRIENDLY.map(|s| s.to_string()).to_vec()
// }
