use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex};

#[derive(Clone, Debug)]
pub enum ChunkMsg {
    Data(Bytes),
    Done,
    Abort,
}

#[derive(Clone)]
pub struct ChunkedObject {
    pub chunks: Vec<Bytes>,
    pub is_complete: bool,
    pub notifier: broadcast::Sender<ChunkMsg>,
}

impl Default for ChunkedObject {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkedObject {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            chunks: Vec::new(),
            is_complete: false,
            notifier: tx,
        }
    }

    pub fn add_chunk(&mut self, chunk: Bytes) {
        self.chunks.push(chunk.clone());
        let _ = self.notifier.send(ChunkMsg::Data(chunk));
    }

    pub fn complete(&mut self) {
        self.is_complete = true;
        let _ = self.notifier.send(ChunkMsg::Done);
    }

    pub fn abort(&mut self) {
        let _ = self.notifier.send(ChunkMsg::Abort);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ChunkMsg> {
        self.notifier.subscribe()
    }
}

#[derive(Clone, Default)]
pub struct AppState {
    pub store: Arc<Mutex<HashMap<String, ChunkedObject>>>,
}

pub type SharedState = Arc<AppState>;

pub fn content_type_for(path: &str) -> &'static str {
    if path.ends_with(".mpd") {
        "application/dash+xml"
    } else if path.ends_with(".m4s") || path.ends_with(".mp4") {
        "video/mp4"
    } else {
        "application/octet-stream"
    }
}
