use crate::prelude::*;

pub struct NodeC {
    pub working: bool,
    pub records: AsyncTunnel<HashSet<FileRecord>>,
    pub status: AsyncTunnel<Arc<str>>,
    pub progress: AsyncTunnel<Progress>,
    pub handle: Option<tokio::task::JoinHandle<()>>,
}

impl Default for NodeC {
    fn default() -> Self {
        Self {
            records: AsyncTunnel::new(1),
            working: false,
            status: AsyncTunnel::new(1),
            progress: AsyncTunnel::new(32),
            handle: None,
        }
    }
}

impl NodeC {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn abort(&mut self) {
        if let Some(handle) = &self.handle {
            handle.abort();
        }
        self.clear();
    }

    pub fn receive_hashset(&mut self) -> Option<HashSet<FileRecord>> {
        if let Some(records) = self.records.recv() {
            self.records.set(records.clone());
            self.handle = None;
            self.working = false;
            self.progress.set(Progress::default());
            self.status
                .set(format! {"Found {} duplicate records", records.len()}.into());
            return Some(records);
        }
        None
    }

    pub fn receive_progress(&mut self) {
        while let Some(progress) = self.progress.recv() {
            self.progress.set(progress);
        }
    }

    pub fn receive_status(&mut self) {
        while let Some(message) = self.status.recv() {
            self.status.set(message);
        }
    }
    pub fn receive(&mut self) -> Option<HashSet<FileRecord>> {
        self.receive_progress();
        self.receive_status();
        self.receive_hashset()
    }

    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.working {
                ui.spinner();
            } else {
                ui.add_space(24.0)
            }
            ui.label(RichText::new(&**self.status.get()).strong());
            if self.working {
                ui.label(format!(
                    "Progress: {} / {}",
                    self.progress.get().count,
                    self.progress.get().total
                ));
            }
        });

        if self.working {
            ui.add(
                egui::ProgressBar::new(
                    self.progress.get().count as f32 / self.progress.get().total as f32,
                )
                .desired_height(4.0),
            );
        }
        empty_line(ui);
    }
    pub fn wrap_async<F, T>(&mut self, action: F)
    where
        F: FnOnce() -> T + Send + 'static,
        T: std::future::Future<Output = Result<HashSet<FileRecord>, sqlx::Error>> + Send + 'static,
    {
        self.working = true;
        let tx = self.records.tx.clone();

        let handle = tokio::spawn(async move {
            let results = action().await;
            if (tx.send(results.expect("Tokio Results Error HashSet")).await).is_err() {
                eprintln!("Failed to send db");
            }
        });
        self.handle = Some(handle);
    }
}

#[derive(Default)]
pub struct Progress {
    pub count: usize,
    pub total: usize,
}

impl Progress {
    pub fn set(&mut self, count: usize, total: usize) {
        self.count = count;
        self.total = total;
    }
}

trait Renderable {
    fn render(&self);
}

struct StructA;
struct StructB;
// Define more structures as needed...

impl Renderable for StructA {
    fn render(&self) {
        println!("Rendering StructA");
    }
}

impl Renderable for StructB {
    fn render(&self) {
        println!("Rendering StructB");
    }
}

// Add implementations for additional structures...

fn main() {
    // Create a vector of trait objects
    let renderables: Vec<Box<dyn Renderable>> = vec![
        Box::new(StructA),
        Box::new(StructB),
        // Add more structures as needed...
    ];

    // Call render on each structure
    for renderable in renderables {
        renderable.render();
    }
}
