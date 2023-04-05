use std::{
    fs::File,
    io::{self, Write},
    path::PathBuf,
    time::Instant,
};

use crc32fast::Hasher;
use flume::{Receiver, Sender};
use modify_crc32::{calculate_crc32, calculate_new_bytes};

pub struct Worker {
    tx: Sender<WorkerOutput>,
    rx: Receiver<WorkerTask>,
}

impl Worker {
    pub fn spawn(thread_name: String) -> (Sender<WorkerTask>, Receiver<WorkerOutput>) {
        let (tx, worker_rx) = flume::bounded::<WorkerTask>(1);
        let (worker_tx, rx) = flume::unbounded::<WorkerOutput>();

        let worker = Worker {
            tx: worker_tx,
            rx: worker_rx,
        };

        let _thread = std::thread::Builder::new()
            .name(thread_name)
            .spawn(move || worker.handle())
            .unwrap();

        (tx, rx)
    }

    fn handle(self) -> Result<(), io::Error> {
        while let Ok(task) = self.rx.recv() {
            match task {
                WorkerTask::CalculateCrc32 { path } => {
                    self.handle_calculate_crc32(path)?;
                }
                WorkerTask::PatchFile {
                    path,
                    old_crc32,
                    new_crc32,
                } => {
                    self.handle_patch_file(path, old_crc32, new_crc32)?;
                }
            }
        }

        Ok(())
    }

    fn handle_calculate_crc32(&self, path: PathBuf) -> Result<(), io::Error> {
        let mut timer = Instant::now();
        let value = calculate_crc32(path, |progress| {
            if timer.elapsed().as_millis() > 200 {
                timer = Instant::now();
                self.tx
                    .send(WorkerOutput::CalculateCrc32Progress(progress))
                    .unwrap();
            }
        })?;

        self.tx.send(WorkerOutput::Crc32(value)).unwrap();

        Ok(())
    }

    fn handle_patch_file(
        &self,
        path: PathBuf,
        old_crc32: u32,
        new_crc32: u32,
    ) -> Result<(), io::Error> {
        let bytes = calculate_new_bytes(old_crc32, new_crc32);

        let mut hasher = Hasher::new_with_initial(old_crc32);
        hasher.update(&bytes);

        let success = hasher.finalize() == new_crc32;
        if success {
            let mut file = File::options().append(true).open(path)?;
            file.write_all(&bytes)?;
            file.flush()?;
        }

        self.tx.send(WorkerOutput::PatchedFile { success }).unwrap();

        Ok(())
    }
}

#[derive(Debug)]
pub enum WorkerTask {
    CalculateCrc32 {
        path: PathBuf,
    },
    PatchFile {
        path: PathBuf,
        old_crc32: u32,
        new_crc32: u32,
    },
}

#[derive(Debug)]
pub enum WorkerOutput {
    Crc32(u32),
    CalculateCrc32Progress(f32),
    PatchedFile { success: bool },
}

#[derive(Clone, Copy, PartialEq)]
pub enum WorkerState {
    Idle,
    CalculatingCrc32 { progress: f32 },
    PatchingFile,
}
