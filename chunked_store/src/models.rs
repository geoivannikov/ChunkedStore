use bytes::Bytes;
use tokio::sync::{broadcast, Mutex};
use std::collections::HashMap;
use std::sync::Arc;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::broadcast::error::TryRecvError;

    #[tokio::test]
    async fn content_type_detection() {
        assert_eq!(content_type_for("/a/b/manifest.mpd"), "application/dash+xml");
        assert_eq!(content_type_for("/x/y/chunk.m4s"), "video/mp4");
        assert_eq!(content_type_for("/x/y/video.mp4"), "video/mp4");
        assert_eq!(content_type_for("/x/y/file.bin"), "application/octet-stream");
    }

    #[tokio::test]
    async fn chunked_object_pubsub_flow() {
        let mut obj = ChunkedObject::new();
        let mut rx = obj.subscribe();

        let part1 = Bytes::from_static(b"hello ");
        let part2 = Bytes::from_static(b"world");

        obj.add_chunk(part1.clone());
        obj.add_chunk(part2.clone());

        let m1 = rx.recv().await.unwrap();
        match m1 { ChunkMsg::Data(b) => assert_eq!(b, part1), _ => panic!("unexpected msg") }

        let m2 = rx.recv().await.unwrap();
        match m2 { ChunkMsg::Data(b) => assert_eq!(b, part2), _ => panic!("unexpected msg") }

        obj.complete();
        let done = rx.recv().await.unwrap();
        match done { ChunkMsg::Done => {}, _ => panic!("expected done") }

        match rx.try_recv() {
            Err(TryRecvError::Empty) | Err(TryRecvError::Closed) | Err(TryRecvError::Lagged(_)) => {}
            Ok(_) => panic!("unexpected extra message after Done"),
        }
    }
}
