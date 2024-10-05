use std::{
	borrow::Cow,
	fmt::Display,
	io::{Error as IoError, ErrorKind as IoErrorKind},
	str::FromStr,
};

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MmcPackComponent {
	pub uid: String,
	pub version: String,
	#[serde(rename(serialize = "dependencyOnly", deserialize = "dependency_only"), default)]
	pub dependency_only: bool,
}

#[derive(Debug, Default, Serialize_repr, Deserialize_repr, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MmcPackVersion {
	#[default]
	V1 = 1,
}
#[derive(Debug, Default, Serialize)]
pub struct MmcPack<'a> {
	pub components: &'a [MmcPackComponent],
	#[serde(rename = "formatVersion")]
	pub format_version: MmcPackVersion,
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
	pub minecraft_servers: Vec<MinecraftClientServerListInfo>,
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
pub struct PackwizIndex<'a> {
	#[serde(rename = "hash-format")]
	pub hash_format: PackwizHashFormat,
	pub files: Vec<PackwizIndexFile<'a>>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PackwizIndexFile<'a> {
	pub file: Cow<'a, str>,
	pub hash: Cow<'a, str>,
	pub metafile: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct PackwizMetadata<'a> {
	pub name: Cow<'a, str>,
	pub author: Cow<'a, str>,
	pub version: Cow<'a, str>,
	#[serde(rename = "pack-format")]
	pub pack_format: PackwizFormatVersion,
	pub versions: PackwizMetadataVersions<'a>,
	pub index: PackwizMetadataIndex<'a>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PackwizMetadataVersions<'a> {
	pub minecraft: Cow<'a, str>,
	pub fabric: Cow<'a, str>,
}
#[derive(Debug, Serialize, Clone)]
pub struct PackwizMetadataIndex<'a> {
	pub file: Cow<'a, str>,
	#[serde(rename = "hash-format")]
	pub hash_format: PackwizHashFormat,
	pub hash: Cow<'a, str>,
}

#[derive(Debug, Serialize, Clone)]
pub struct PackwizMod<'a> {
	pub name: &'a str,
	pub filename: Cow<'a, str>,
	pub side: PackwizModSide,
	pub download: PackwizModDownload<'a>,
}
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum PackwizModSide {
	#[serde(rename = "server")]
	Server,
	#[serde(rename = "client")]
	Client,
	#[serde(rename = "both")]
	Both,
}
impl PackwizModSide {
	pub fn all() -> impl DoubleEndedIterator<Item = PackwizModSide> {
		static DIRECTIONS: [PackwizModSide; 3] = [PackwizModSide::Server, PackwizModSide::Client, PackwizModSide::Both];
		DIRECTIONS.clone().into_iter()
	}
}

impl Display for PackwizModSide {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			PackwizModSide::Server => f.write_str("server"),
			PackwizModSide::Client => f.write_str("client"),
			PackwizModSide::Both => f.write_str("both"),
		}
	}
}
impl FromStr for PackwizModSide {
	type Err = IoError; // This is a hack, but w/e
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"server" => Ok(PackwizModSide::Server),
			"client" => Ok(PackwizModSide::Client),
			"both" => Ok(PackwizModSide::Both),
			_ => Err(IoError::new(
				IoErrorKind::NotFound,
				"PackwizModSide \"{s}\" should be \"client\" \"server\" or \"both\"",
			)),
		}
	}
}

#[derive(Debug, Serialize, Clone)]
pub struct PackwizModDownload<'a> {
	pub url: Cow<'a, str>,
	#[serde(rename = "hash-format")]
	pub hash_format: PackwizHashFormat,
	pub hash: Cow<'a, str>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MinecraftClientServerList {
	pub servers: Vec<MinecraftClientServerListInfo>,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MinecraftClientServerListInfo {
	pub name: String,
	pub ip: String,
}
