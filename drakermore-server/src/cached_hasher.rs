use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::{Arc, LazyLock, RwLock},
	time::SystemTime,
};

use sha2::{Digest, Sha512};
use tokio::fs;

static CACHED_HASHES: LazyLock<RwLock<HashMap<PathBuf, (SystemTime, Arc<str>)>>> =
	LazyLock::new(|| RwLock::new(HashMap::new()));

/// Returns a hex-encoded sha512 hash from the given file, and caches the hash
pub async fn get_hash_from_file(file_path: &Path) -> Result<Arc<str>, anyhow::Error> {
	let file_mtime = fs::metadata(file_path).await?.modified()?;
	if let Some(cached_hash) = CACHED_HASHES
		.read()
		.unwrap()
		.get(file_path)
		.and_then(|(cached_mtime, cached_hash)| {
			if file_mtime > *cached_mtime {
				None
			} else {
				Some(cached_hash)
			}
		}) {
		return Ok(cached_hash.clone());
	}
	let hash: Arc<str> = hex::encode(Sha512::digest(fs::read(file_path).await?)).into();
	// Yes, we currently don't watch for files being deleted which could cause memory leaks, too bad!
	CACHED_HASHES
		.write()
		.unwrap()
		.insert(file_path.into(), (file_mtime, hash.clone()));

	Ok(hash)
}
