use std::borrow::Cow;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MmcPackComponent {
	pub uid: String,
	pub version: String,
	#[serde(rename(serialize = "dependencyOnly", deserialize = "dependency_only"), default)]
	pub dependency_only: bool,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum MmcPackVersion {
	#[default]
	V1 = 1,
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MmcPack {
	components: Vec<MmcPackComponent>,
	#[serde(rename = "formatVersion")]
	format_version: MmcPackVersion,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ModRepo {
	#[serde(rename = "curseforge")]
	Curseforge,
	#[serde(rename = "modrinth")]
	Modrinth,
}
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum ModRepoChannel {
	#[serde(rename = "release")]
	Release,
	#[serde(rename = "beta")]
	Beta,
	#[serde(rename = "alpha")]
	Alpha,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModListItem {
	pub id: String,
	pub repo: ModRepo,
	pub channel: ModRepoChannel,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DrakermoreModConfig {
	pub name: String,
	pub minecraft_version: String,
	pub pack_author: String,
	pub pack_version: String,
	pub fabric_loader_version: String,
	pub mmc_pack_components: Vec<MmcPackComponent>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum PackwizFormatVersion {
	#[default]
	#[serde(rename = "packwiz:1.1.0")]
	V1_1_0,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum PackwizHashFormat {
	#[serde(rename = "sha256")]
	Sha256,
	#[serde(rename = "sha512")]
	Sha512,
}

#[derive(Debug, Serialize, Clone)]
pub struct PackwizMetadata<'a> {
	name: &'a str,
	author: &'a str,
	version: &'a str,
	#[serde(rename = "pack-format")]
	pack_format: PackwizFormatVersion,
	versions: PackwizMetadataVersions<'a>,
	index: PackwizMetadataIndex<'a>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PackwizMetadataVersions<'a> {
	minecraft: &'a str,
	fabric: &'a str,
}
#[derive(Debug, Serialize, Clone)]
pub struct PackwizMetadataIndex<'a> {
	file: Cow<'a, str>,
	#[serde(rename = "hash-format")]
	hash_format: PackwizHashFormat,
	hash: Cow<'a, str>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PackwizMod<'a> {
	name: &'a str,
	filename: Cow<'a, str>,
	side: PackwizModSide,
	download: PackwizModDownload<'a>,
}
#[derive(Debug, Serialize, Clone)]
pub enum PackwizModSide {
	#[serde(rename = "server")]
	Server,
	#[serde(rename = "client")]
	Client,
	#[serde(rename = "both")]
	Both,
}
impl ToString for PackwizModSide {
	fn to_string(&self) -> String {
		match self {
			PackwizModSide::Server => "server".into(),
			PackwizModSide::Client => "client".into(),
			PackwizModSide::Both => "both".into(),
		}
	}
}

#[derive(Debug, Serialize, Clone)]
pub struct PackwizModDownload<'a> {
	url: Cow<'a, str>,
	#[serde(rename = "hash-format")]
	hash_format: PackwizHashFormat,
	hash: Cow<'a, str>,
}
