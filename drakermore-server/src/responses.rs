use std::{
	io::{Cursor, Error as IoError, ErrorKind as IoErrorKind},
	ops::{Deref, DerefMut},
};

use axum::{
	http::{header, HeaderName, HeaderValue, StatusCode},
	response::{IntoResponse, Response},
};
use bytes::Bytes;
use lazy_regex::regex_replace_all;
use zip::ZipWriter;

pub fn ok_or_500_response<T: IntoResponse, E: std::error::Error>(result: Result<T, E>) -> Response {
	match result {
		Ok(response) => response.into_response(),
		Err(err) => (
			StatusCode::INTERNAL_SERVER_ERROR,
			[(
				header::CONTENT_TYPE,
				HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
			)],
			err.to_string(),
		)
			.into_response(),
	}
}
// I discovered that https://docs.rs/axum/latest/axum/response/type.Result.html exists, whoops!
pub fn ok_or_anyhow_response<T: IntoResponse>(result: Result<T, anyhow::Error>) -> Response {
	let headers = [(
		header::CONTENT_TYPE,
		HeaderValue::from_static(mime::TEXT_PLAIN_UTF_8.as_ref()),
	)];
	match result {
		Ok(response) => response.into_response(),
		Err(err) => match err.downcast_ref::<IoError>() {
			Some(io_err) if io_err.kind() == IoErrorKind::NotFound => {
				(StatusCode::NOT_FOUND, headers, io_err.to_string()).into_response()
			},
			Some(io_err) if io_err.kind() == IoErrorKind::PermissionDenied => {
				(StatusCode::FORBIDDEN, headers, io_err.to_string()).into_response()
			},
			_ => (StatusCode::INTERNAL_SERVER_ERROR, headers, format!("{:?}", err)).into_response(),
		},
	}
}
pub struct ZipResponse {
	file_name: String,
	inner: ZipWriter<Cursor<Vec<u8>>>,
}
impl ZipResponse {
	pub fn new(file_name: String) -> Self {
		Self {
			file_name,
			inner: ZipWriter::new(Cursor::new(Vec::with_capacity(1024))),
		}
	}
}
impl Deref for ZipResponse {
	type Target = ZipWriter<Cursor<Vec<u8>>>;
	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}
impl DerefMut for ZipResponse {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}
impl IntoResponse for ZipResponse {
	fn into_response(self) -> Response {
		// Use a small initial capacity of 128 bytes like serde_json::to_vec
		// https://docs.rs/serde_json/1.0.82/src/serde_json/ser.rs.html#2189'
		let filename = self.file_name;
		ok_or_500_response(self.inner.finish().map(|zip_bytes| {
			(
				[
					(header::CONTENT_TYPE, HeaderValue::from_static("application/zip")),
					download_file_name_header(filename.as_str()),
				],
				Bytes::from(zip_bytes.into_inner()),
			)
				.into_response()
		}))
	}
}

pub fn download_file_name_header(filename: &str) -> (HeaderName, HeaderValue) {
	// Filename encoding ideas taken from https://stackoverflow.com/questions/93551/how-to-encode-the-filename-parameter-of-content-disposition-header-in-http
	(
		header::CONTENT_DISPOSITION,
		HeaderValue::from_str(&format!(
			"attachment; filename=\"{}\"; filename*=utf-8''{}",
			regex_replace_all!(r#"[^a-zA-Z0-9._\-+,@£$€!½§~'=()\[\]{}]"#, filename, "_"),
			url_encor::encode(filename)
		))
		.expect("filename should being safe should already have been validated"),
	)
}
