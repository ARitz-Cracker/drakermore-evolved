use std::{
	borrow::Cow,
	io::{Error as IoError, ErrorKind as IoErrorKind, Write},
	path::{Path, PathBuf},
	sync::LazyLock,
};

use axum::{
	extract::Path as AxumPath,
	http::{header, HeaderValue},
	response::Response,
	routing::get,
	Router,
};
use bpaf::Bpaf;
use bytes::Bytes;
use cached_hasher::get_hash_from_file;
use crab_nbt::{Nbt, NbtCompound, NbtTag};
use responses::{download_file_name_header, ok_or_anyhow_response, ZipResponse};
use schemas::{
	DrakermoreModConfig, MmcPack, PackwizFormatVersion, PackwizHashFormat, PackwizIndex, PackwizIndexFile,
	PackwizMetadata, PackwizMetadataIndex, PackwizMetadataVersions, PackwizMod, PackwizModDownload, PackwizModSide,
};
use sha2::{Digest, Sha512};
use tokio::fs;
use zip::write::SimpleFileOptions;

const PACKWIZ_INSTALLER_BOOTSTRAP_JAR: &'static [u8] =
	include_bytes!("../baked_in_files/packwiz-installer-bootstrap.jar");

mod cached_hasher;
mod responses;
mod schemas;

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct CliOptions {
	#[bpaf(short, long)]
	/// Path to drakermore config file
	pub config: PathBuf,
	#[bpaf(short, long)]
	/// Path to where the mods where downloaded by the scraper
	pub download_dir: PathBuf,
	#[bpaf(short, long, fallback("0.0.0.0:3000".to_string()))]
	/// Address and port to bind to, defaults to "0.0.0.0:3000"
	pub bind: String,
	#[bpaf(short('p'), long)]
	/// The prefix to use for URLs
	pub url_prefix: String, // Note: https://stackoverflow.com/questions/33218367/
}

// CLI options as a LazyLock so it's accessible globally
const CLI_OPTIONS: LazyLock<CliOptions> = LazyLock::new(|| {
	let mut options = cli_options().run();
	while options.url_prefix.ends_with('/') {
		options.url_prefix.pop();
	}
	options
});

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();
	println!("Pre-hashing .jar files...");
	pw_index_string().await?;
	// build our application with a route
	let app = Router::new()
		// `GET /` goes to `root`
		.route("/", get(root))
		.route("/mmc_pack.zip", get(get_mmc_zip))
		// `POST /users` goes to `create_user`
		.route("/jars/:side/:jar_file", get(get_mod_jar))
		.route("/packwiz/pack.toml", get(get_pw_pack))
		.route("/packwiz/index.toml", get(get_pw_index))
		.route("/packwiz/mods/:jar_metadata", get(get_pw_mod_metadata));

	// run our app with hyper, listening globally on port 3000
	println!("Listening to {}...", CLI_OPTIONS.bind);
	let listener = tokio::net::TcpListener::bind(&CLI_OPTIONS.bind).await?;
	axum::serve(listener, app).await?;

	Ok(())
}

// basic handler that responds with a static string
async fn root() -> &'static str {
	"Hello, world! This is drakermore-evolved (or drakermost?)"
}

async fn pw_mod_metadata_string(realm: PackwizModSide, jar_file_name: PathBuf) -> anyhow::Result<String> {
	let jar_file_name_str = jar_file_name.to_string_lossy();
	if !jar_file_name_str.ends_with(".jar") {
		anyhow::bail!("attempted to show mod metadata for {realm}/{jar_file_name_str} which doesn't end in \".jar\"");
	}
	let mut jar_full_path = CLI_OPTIONS.download_dir.canonicalize()?;
	jar_full_path.push(realm.to_string());
	// let mut jar_name_path = jar_path.clone();
	jar_full_path.push(format!("{jar_file_name_str}.name.txt"));

	let mod_name = fs::read_to_string(&jar_full_path)
		.await
		.map(|mut string| {
			string.truncate(string.trim_end().len());
			string
		})
		.unwrap_or(jar_file_name_str[0..(jar_file_name_str.len() - 4)].into());

	jar_full_path.pop();
	jar_full_path.push(&jar_file_name);

	Ok(toml::to_string_pretty(&PackwizMod {
		download: PackwizModDownload {
			url: format!("{}/jars/{realm}/{jar_file_name_str}", &CLI_OPTIONS.url_prefix).into(),
			hash_format: PackwizHashFormat::Sha512,
			hash: Cow::Borrowed(&get_hash_from_file(&jar_full_path).await?),
		},
		name: &mod_name,
		filename: jar_file_name_str,
		side: realm,
	})?)
}

async fn pw_index_string() -> anyhow::Result<String> {
	let mut jar_full_path = CLI_OPTIONS.download_dir.canonicalize()?;
	let mut result: Vec<PackwizIndexFile<'_>> = Vec::new();
	for realm in PackwizModSide::all() {
		jar_full_path.push(realm.to_string());
		let mut dir_reader = fs::read_dir(&jar_full_path).await?;
		while let Some(dir_entry) = dir_reader.next_entry().await? {
			let jar_file_name = dir_entry.file_name();
			let jar_file_name_str = jar_file_name.to_string_lossy();
			if !jar_file_name_str.ends_with(".jar") || !dir_entry.file_type().await?.is_file() {
				continue;
			}
			let jar_file_name_str = &jar_file_name_str[0..(jar_file_name_str.len() - 4)];
			result.push(PackwizIndexFile {
				file: format!("mods/{jar_file_name_str}.pw.toml").into(),
				hash: hex::encode(Sha512::digest(
					pw_mod_metadata_string(realm, jar_file_name.into()).await?,
				))
				.into(),
				metafile: true,
			});
		}
		jar_full_path.pop();
	}
	Ok(toml::to_string_pretty(&PackwizIndex {
		hash_format: PackwizHashFormat::Sha512,
		files: result,
	})?)
}

async fn get_pw_pack() -> Response {
	ok_or_anyhow_response(
		async {
			let modpack: DrakermoreModConfig =
				toml::from_str(&String::from_utf8(fs::read(&CLI_OPTIONS.config).await?)?)?;

			Ok(toml::to_string_pretty(&PackwizMetadata {
				name: modpack.name.into(),
				author: modpack.pack_author.into(),
				version: modpack.pack_version.into(),
				pack_format: PackwizFormatVersion::V1_1_0,
				versions: PackwizMetadataVersions {
					minecraft: modpack.minecraft_version.into(),
					fabric: modpack.fabric_loader_version.into(),
				},
				index: PackwizMetadataIndex {
					file: "index.toml".into(),
					hash_format: PackwizHashFormat::Sha512,
					hash: hex::encode(Sha512::digest(pw_index_string().await?)).into(),
				},
			})?)
		}
		.await,
	)
}
async fn get_pw_index() -> Response {
	ok_or_anyhow_response(pw_index_string().await)
}
async fn find_jar_realm(jar_file_name: &Path) -> anyhow::Result<Option<PackwizModSide>> {
	let mut jar_full_path = CLI_OPTIONS.download_dir.canonicalize()?;
	for realm in PackwizModSide::all() {
		jar_full_path.push(realm.to_string());
		jar_full_path.push(jar_file_name);
		if fs::try_exists(&jar_full_path).await? {
			return Ok(Some(realm));
		}
		jar_full_path.pop();
		jar_full_path.pop();
	}
	Ok(None)
}
async fn get_pw_mod_metadata(AxumPath(mut jar_file_name_str): AxumPath<String>) -> Response {
	ok_or_anyhow_response(
		async {
			if !jar_file_name_str.ends_with(".pw.toml") {
				return Err(IoError::new(
					IoErrorKind::NotFound,
					format!("cannot find {jar_file_name_str} in this folder"),
				)
				.into());
			}
			jar_file_name_str.truncate(jar_file_name_str.len() - "pw.toml".len());
			jar_file_name_str.push_str("jar");
			let jar_file_name = PathBuf::from(jar_file_name_str.clone());
			Ok(pw_mod_metadata_string(
				find_jar_realm(&jar_file_name)
					.await?
					.ok_or_else(|| IoError::new(IoErrorKind::NotFound, format!("Cannot find {jar_file_name_str}")))?,
				jar_file_name,
			)
			.await?)
		}
		.await,
	)
}

async fn get_mmc_zip() -> Response {
	ok_or_anyhow_response(
		async {
			let modpack: DrakermoreModConfig =
				toml::from_str(&String::from_utf8(fs::read(&CLI_OPTIONS.config).await?)?)?;

			let zip_options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

			let mut response = ZipResponse::new(format!("{}.zip", modpack.name));
			response.start_file("mmc-pack.json", zip_options.clone())?;
			response.write(&serde_json::to_vec(&MmcPack {
				components: &modpack.mmc_pack_components,
				..Default::default()
			})?)?;
			response.start_file("instance.cfg", zip_options.clone())?;
			response.write(
				format!(
					"InstanceType=OneSix
OverrideCommands=true
PreLaunchCommand=\"$INST_JAVA\" -jar packwiz-installer-bootstrap.jar {}/packwiz/pack.toml
name=${}
",
					CLI_OPTIONS.url_prefix, modpack.name
				)
				.as_bytes(),
			)?;

			response.start_file(
				".minecraft/packwiz-installer-bootstrap.jar",
				// .jar files are already zipped
				SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
			)?;
			response.write(PACKWIZ_INSTALLER_BOOTSTRAP_JAR)?;

			response.start_file(
				".minecraft/servers.dat",
				// .jar files are already zipped
				SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored),
			)?;
			response.write(
				&Nbt::new(
					"".into(),
					NbtCompound::from_iter([(
						"servers".to_owned(),
						NbtTag::List(
							modpack
								.minecraft_servers
								.into_iter()
								.map(|server| {
									NbtTag::Compound(NbtCompound::from_iter([
										("name".to_owned(), NbtTag::String(server.name)),
										("ip".to_owned(), NbtTag::String(server.ip)),
										("hidden".to_owned(), NbtTag::Byte(0)),
									]))
								})
								.collect::<Vec<_>>(),
						),
					)]),
				)
				.write(),
			)?;

			Ok(response)
		}
		.await,
	)
}

async fn get_mod_jar(AxumPath((realm, jar_file_name)): AxumPath<(PackwizModSide, PathBuf)>) -> Response {
	ok_or_anyhow_response(
		async {
			let mut jar_path = CLI_OPTIONS.download_dir.canonicalize()?;
			jar_path.push(realm.to_string());
			jar_path.push(&jar_file_name);
			Ok((
				[
					(
						header::CONTENT_TYPE,
						HeaderValue::from_static("application/java-archive"),
					),
					download_file_name_header(&jar_file_name.as_os_str().to_string_lossy()),
				],
				// TODO: Do we have to buffer the entire file?
				Bytes::from(fs::read(jar_path).await?),
			))
		}
		.await,
	)
}
