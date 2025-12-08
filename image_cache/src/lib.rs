mod cache;
pub(crate) mod memory_cache;
pub(crate) mod traits;

pub use cache::ImageCache;

// TODO: revisit if we need a file cache in addtion to memory
// pub(crate) mod file_cache;
// let mut buffer: Vec<u8> = Vec::new();
// let mut file = File::open("favicon-96x96.png").await.unwrap();
// let _ = file.read_to_end(&mut buffer).await.unwrap();
