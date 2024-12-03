use std::path::PathBuf;


use futures::{stream::BoxStream, Stream, StreamExt, TryStreamExt};
use tokio_stream::wrappers::ReadDirStream;

pub fn subfiles_in_folder(folder: PathBuf, go_deep: bool) -> BoxStream<'static, Result<PathBuf, std::io::Error>> {
	files_in_folders(futures::stream::once(futures::future::ready(Ok(folder.clone()))).chain(subfolders_in_folder(folder, go_deep)))
}

pub fn subfolders_in_folder(path: PathBuf, go_deep: bool) -> BoxStream<'static, Result<PathBuf, std::io::Error>> {
	let mut stream = futures::stream::once(tokio::fs::read_dir(path.clone())).map(|read_dir_result| {
		read_dir_result.map(ReadDirStream::new)
	}).map_ok(|read_dir_stream| {
		read_dir_stream.filter_map(|f_result| async {
			if let Ok(f) = &f_result {
				let metadata_result = f.metadata().await;
				let Ok(metadata) = metadata_result else {
					return Some(Err(metadata_result.unwrap_err()));
				};
				if !metadata.is_dir() {
					return None;
				}
			}
			Some(f_result)
		}).map(|f_result| {
			f_result.map(|f| {f.path()})
		})
	}).try_flatten().boxed();
	if go_deep {
		stream = stream.map_ok(|sub_folder| {
			futures::stream::once(futures::future::ready(Ok(sub_folder.clone()))).chain(subfolders_in_folder(sub_folder, true))
		}).try_flatten().boxed();
	}
	stream
}

pub fn files_in_folder(path: PathBuf) -> BoxStream<'static, Result<PathBuf, std::io::Error>> {
	futures::stream::once(tokio::fs::read_dir(path.clone())).map(|read_dir_result| {
		read_dir_result.map(ReadDirStream::new)
	}).map_ok(|read_dir_stream| {
		read_dir_stream.filter_map(|f_result| async {
			if let Ok(f) = &f_result {
				let metadata_result = f.metadata().await;
				let Ok(metadata) = metadata_result else {
					return Some(Err(metadata_result.unwrap_err()));
				};
				if !metadata.is_file() {
					return None;
				}
			}
			Some(f_result)
		}).map(|f_result| {
			f_result.map(|f| {f.path()})
		})
	}).try_flatten().boxed()
}

pub fn files_in_folders(files: impl Stream<Item = Result<PathBuf, std::io::Error>> + Send + 'static) -> BoxStream<'static, Result<PathBuf, std::io::Error>> {
	files.map_ok(|folder| {
		files_in_folder(folder)
	}).try_flatten().boxed()
}
