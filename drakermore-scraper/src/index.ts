

import { promises as fs } from 'fs';
import TOML from '@iarna/toml';
import { program as commandProgram } from "commander";
import path from "path";
import puppeteer from 'puppeteer';

commandProgram
	.requiredOption("-c, --config <PATH>", "Path to mod list config")
	.requiredOption("-d, --download-dir <PATH>", "Path to save the .jar files")
	.requiredOption("-m, --mod-dir <PATH>", "Minecraft server mod folder")
	.option("--not-headless", "Allows you to see what the scraper is doing")
	.parse();
const {
	config: configArg,
	downloadDir: downloadDirArg,
	modDir: serverModDirArg,
	notHeadless: notHeadlessArg,

} = commandProgram.opts();
function sleep(ms: number): Promise<void> {
	return new Promise(resolve => setTimeout(resolve, ms));
}

const URL_PREFIX = {
	"curseforge": "https://www.curseforge.com/minecraft/mc-mods/",
	"modrinth": "https://modrinth.com/mod/"
} as const;
const URL_SUFFIX = {
	"curseforge": "/files/all",
	"modrinth": "/versions"
} as const;
const URL_REQUIRED_PARAMS = {
	"curseforge": [
		["page", "1"], // Because this isn't the default for some reason
		["pageSize", "50"], // Default is 20, might as well do more
		["gameVersionTypeId", "4"] // Selects fabric loader
	] as [string, string][],
	"modrinth": [["l", "fabric"]] as [string, string][] // Selects fabric loader
} as const;
const URL_MINECRAFT_VERSION_PARAM = {
	"curseforge": "version",
	"modrinth": "g"
} as const;
const URL_CHANNEL_PARAM = {
	"modrinth": "c"
} as const;


type ModConfig = {
	minecraft_version: string,
	mod_list: ModListItem[]
}
type ModListItem = {
	id: string
	repo: "curseforge" | "modrinth",
	channel: "alpha" | "beta" | "release"
}
async function readModConfig(filePath: string) {
	filePath = path.resolve(filePath);
	const maybeModConfig = TOML.parse((await fs.readFile(filePath)).toString("utf8"));
	if (
		typeof maybeModConfig.minecraft_version != "string" ||
		!/^\d+\.\d+\.\d+$/.test(maybeModConfig.minecraft_version)
	) {
		throw new Error(`${filePath}: Invalid minecraft_version: ${maybeModConfig.minecraft_version}`);
	}
	//mod_list
	if (!Array.isArray(maybeModConfig.mod_list)) {
		throw new Error(`${filePath}: mod_list is not an array`);
	}
	for (let i = 0; i < maybeModConfig.mod_list.length; i += 1) {
		const maybeModListItem = maybeModConfig.mod_list[i] as any;
		if (!maybeModListItem) {
			throw new Error(`${filePath}: mod_list[${i}] is a falsy value`);
		}
		if (typeof maybeModListItem.id != "string" || !maybeModListItem.id) {
			throw new Error(`${filePath}: invalid mod_list[${i}].id`);
		}
		switch (maybeModListItem.repo) {
			case "curseforge":
			case "modrinth":
				break;
			default:
				throw new Error(`${filePath}: invalid mod_list[${i}].repo: ${maybeModListItem.repo}`);
		}
		switch (maybeModListItem.channel) {
			case "alpha":
			case "beta":
			case "release":
				break;
			default:
				throw new Error(`${filePath}: invalid mod_list[${i}].repo: ${maybeModListItem.repo}`);
		}
	}
	return maybeModConfig as ModConfig;
}

const baseDownloadDir = path.resolve(downloadDirArg);
const browserDownloadDir = path.resolve(baseDownloadDir, "temp");
const serverModDownloadDir = path.resolve(baseDownloadDir, "server");
const sharedModDownloadDir = path.resolve(baseDownloadDir, "both");
const clientModDownloadDir = path.resolve(baseDownloadDir, "client");

for (const downloadDir of [browserDownloadDir, serverModDownloadDir, sharedModDownloadDir, clientModDownloadDir]) {
	await fs.rm(downloadDir, { force: true, recursive: true });
	await fs.mkdir(downloadDir);
}

const modListConfig = await readModConfig(configArg);

const browser = await puppeteer.launch({
	headless: !notHeadlessArg
});
const page = (await browser.pages())[0] ?? await browser.newPage();
const debugSession = await page.createCDPSession();
await debugSession.send('Browser.setDownloadBehavior', {
	behavior: "allow",
	downloadPath: browserDownloadDir,
	eventsEnabled: true
});
function waitUntilDownload(page: puppeteer.Page): Promise<void> {
	return new Promise((resolve, reject) => {
		debugSession.on("Browser.downloadProgress", e => { // or 'Browser.downloadProgress'
			if (e.state === "completed") {
				resolve();
			} else if (e.state === "canceled") {
				reject();
			}
		});
	});
}

for (let i = 0; i < modListConfig.mod_list.length; i += 1) {
	const mod = modListConfig.mod_list[i];
	console.info("Downloading mod " + (i + 1) + "/" + modListConfig.mod_list.length);
	const searchParams = URL_REQUIRED_PARAMS[mod.repo].concat(
		[[URL_MINECRAFT_VERSION_PARAM[mod.repo], modListConfig.minecraft_version]],
		// TODO: Prioritize new full releases over beta releases
		mod.repo == "modrinth" ? [[URL_CHANNEL_PARAM[mod.repo], mod.channel]] : []
	)
	const downloadPageUrl = new URL(
		mod.id + URL_SUFFIX[mod.repo] + "?" + new URLSearchParams(searchParams), URL_PREFIX[mod.repo]
	) + "";
	console.info("Going to", downloadPageUrl);
	await page.goto(downloadPageUrl);
	switch (mod.repo) {
		case "curseforge": {
			const downloadSelector = `a[href^="/minecraft/mc-mods/${mod.id}/download/"]`;
			// TODO: Prioritize new full releases over beta releases
			const latestDownloadSelector = downloadSelector + `:is(.file-row:has(.channel-tag.${mod.channel}) a)`;
			while (true) {
				await page.waitForSelector(downloadSelector);
				if (await page.$(latestDownloadSelector)) {
					const modName = await page.$eval(".project-header > .name-container > h1", (el) => el.innerText);
					console.info("Mod name:", modName);
					console.info("Mod realm: unknown - assuming both");
					await page.click(`.kebab-menu button:is(.file-row:has(.channel-tag.${mod.channel}) button)`);
					await sleep(123);
					const downloadPromise = waitUntilDownload(page);
					await page.click(latestDownloadSelector);
					console.info("Waiting for download to complete...");
					await downloadPromise;
					const [downloadFileName] = await fs.readdir(browserDownloadDir);
					console.info("Downloaded file:", downloadFileName);
					await Promise.all([
						fs.rename(
							path.resolve(browserDownloadDir, downloadFileName),
							path.resolve(sharedModDownloadDir, downloadFileName),
						),
						fs.writeFile(
							path.resolve(sharedModDownloadDir, downloadFileName + ".name.txt"),
							modName + "\n"
						)
					]);
					break;
				}
				console.info(latestDownloadSelector + " not found, going to next page...");
				await page.click("button.btn-single-icon.btn-next");
				await sleep(813);
			}
			break;
		}
		case "modrinth": {
			const downloadSelector = "a[href^=\"https://cdn.modrinth.com/data/\"][aria-label=\"Download\"]";
			await page.waitForSelector(downloadSelector);
			const modName = await page.$eval("h1", (el) => el.innerText);
			const modRealm = await page.$eval("section:last-child > h3 + div.tag-list", el => {
				const text = el.innerText.toLowerCase();
				if (text.includes("client and server")) {
					return "both";
				} else if (text.includes("server-side")) {
					if (text.includes("client-side")) {
						return "both";
					}
					return "server";
				} else if (text.includes("client-side")) {
					return "client";
				}
				return "unknown - assuming both";
			});
			console.info("Mod name:", modName);
			console.info("Mod realm:", modRealm);
			const downloadPromise = waitUntilDownload(page);
			await page.click(downloadSelector);
			console.info("Waiting for download to complete...");
			await downloadPromise;
			const [downloadFileName] = await fs.readdir(browserDownloadDir);
			console.info("Downloaded file:", downloadFileName);
			switch (modRealm) {
				case "server":
					await Promise.all([
						fs.rename(
							path.resolve(browserDownloadDir, downloadFileName),
							path.resolve(serverModDownloadDir, downloadFileName),
						),
						fs.writeFile(
							path.resolve(serverModDownloadDir, downloadFileName + ".name.txt"),
							modName + "\n"
						)
					]);
					//
					break;
				case "client":
					await Promise.all([
						fs.rename(
							path.resolve(browserDownloadDir, downloadFileName),
							path.resolve(clientModDownloadDir, downloadFileName),
						),
						fs.writeFile(
							path.resolve(clientModDownloadDir, downloadFileName + ".name.txt"),
							modName + "\n"
						)
					]);
					//
					break;
				default:
					await Promise.all([
						fs.rename(
							path.resolve(browserDownloadDir, downloadFileName),
							path.resolve(sharedModDownloadDir, downloadFileName),
						),
						fs.writeFile(
							path.resolve(sharedModDownloadDir, downloadFileName + ".name.txt"),
							modName + "\n"
						)
					]);
				//
			}
			break;
		}
		default:
			throw new Error("this shouln't happen")
	}
}
await fs.rmdir(browserDownloadDir);
await browser.close();
console.info("Mod downloads completed!");

const serverModDir = path.resolve(serverModDirArg)
await fs.rm(serverModDir, { recursive: true, force: true });
await fs.mkdir(serverModDir);
for (const modFile of await fs.readdir(serverModDownloadDir)) {
	if (modFile.endsWith(".txt")) {
		continue;
	}
	await fs.symlink(path.resolve(serverModDownloadDir, modFile), path.resolve(serverModDir, modFile));
}
for (const modFile of await fs.readdir(sharedModDownloadDir)) {
	if (modFile.endsWith(".txt")) {
		continue;
	}
	await fs.symlink(path.resolve(sharedModDownloadDir, modFile), path.resolve(serverModDir, modFile));
}
console.info("Mods symlink'd to the server mod folder!");
console.info("Everything seems to be done!");
