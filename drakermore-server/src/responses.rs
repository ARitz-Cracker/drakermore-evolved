use std::{
	io::{Cursor, Error as IoError, ErrorKind as IoErrorKind},
	ops::{Deref, DerefMut},
};

use axum::{
	http::{header, HeaderValue, StatusCode},
	response::{IntoResponse, Response},
};
use bytes::Bytes;
use lazy_regex::regex_replace_all;
use serde::Serialize;
use zip::ZipWriter;

// Heavily inspired from https://docs.rs/axum/0.7.7/src/axum/json.rs.html#181
pub struct TomlResponse<T> {
	inner: T,
}
impl<T> TomlResponse<T> {
	pub fn new(inner: T) -> Self {
		Self { inner }
	}
}

impl<T> IntoResponse for TomlResponse<T>
where
	T: Serialize,
{
	fn into_response(self) -> Response {
		// Use a small initial capacity of 128 bytes like serde_json::to_vec
		// https://docs.rs/serde_json/1.0.82/src/serde_json/ser.rs.html#2189
		let mut output = String::with_capacity(128);
		let serializer = toml::Serializer::pretty(&mut output);
		ok_or_500_response(self.inner.serialize(serializer))
	}
}
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
pub struct ZipResponse<'name> {
	file_name: &'name str,
	inner: ZipWriter<Cursor<Vec<u8>>>,
}
impl<'name> ZipResponse<'name> {
	pub fn new(file_name: &'name str) -> Self {
		Self {
			file_name,
			inner: ZipWriter::new(Cursor::new(Vec::with_capacity(1024))),
		}
	}
}
impl Deref for ZipResponse<'_> {
	type Target = ZipWriter<Cursor<Vec<u8>>>;
	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}
impl DerefMut for ZipResponse<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inner
	}
}
impl IntoResponse for ZipResponse<'_> {
	fn into_response(self) -> Response {
		// Use a small initial capacity of 128 bytes like serde_json::to_vec
		// https://docs.rs/serde_json/1.0.82/src/serde_json/ser.rs.html#2189'
		let filename = self.file_name;
		ok_or_500_response(self.inner.finish().map(|zip_bytes| {
			(
				[
					(header::CONTENT_TYPE, HeaderValue::from_static("application/zip")),
					// Filename encoding ideas taken from https://stackoverflow.com/questions/93551/how-to-encode-the-filename-parameter-of-content-disposition-header-in-http
					(
						header::CONTENT_DISPOSITION,
						HeaderValue::from_str(&format!(
							"attachment; filename=\"{}\"; filename*=utf-8''{}",
							regex_replace_all!(r#"[^a-zA-Z0-9._\-+,@£$€!½§~'=()\[\]{}]"#, filename, "_"),
							url_encor::encode(filename)
						))
						.expect("filename should being safe should already have been validated"),
					),
				],
				Bytes::from(zip_bytes.into_inner()),
			)
				.into_response()
		}))
	}
}
