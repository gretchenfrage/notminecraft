
use super::jar_reader::JarReader;
use std::{
	path::Path,
	env::{
		var,
		VarError,
	},
	io::{
		stdout,
		Write,
	},
};
use tokio::{
	io::{
		stdin,
		BufReader,
		AsyncBufReadExt,
		AsyncWriteExt,
	},
	fs::File,
};
use reqwest::get;
use futures_util::stream::StreamExt;
use anyhow::{bail, ensure, Result};


const DOWNLOAD_LINK: &'static str =
	"https://piston-data.mojang.com/v1/objects/76d35cb452e739bd4780e835d17faf0785d755f9/client.jar";
const ENV_VAR_KEY: &'static str =
	"MINECRAFT_JAR";
const DEFAULT_PATH: &'static str =
	"minecraft-b1.0.2.jar";

pub async fn jar_acquire() -> Result<JarReader> {
	let val = match var(ENV_VAR_KEY) {
		Ok(val) => Some(val),
		Err(VarError::NotPresent) => None,
		Err(_) => bail!("env var read error"),
	};

	if let Some(val) = val {
		Ok(JarReader::new(&val).await?)
	} else {
		if Path::new(DEFAULT_PATH).exists() {
			Ok(JarReader::new(DEFAULT_PATH).await?)
		} else {
			println!("{} not detected, auto-download?", DEFAULT_PATH);
			println!("will download from {}", DOWNLOAD_LINK);
			
			let stdout = stdout();

			let mut buf = String::new();
			let mut input = BufReader::new(stdin());
			loop {
				{
					let mut stdout = stdout.lock();
					stdout.write_all(b"[y/n] ").expect("stdout write fail");
					stdout.flush().expect("stdout flush fail");
				}

				let n = input.read_line(&mut buf).await?;
				ensure!(n != 0, "stdin closed");

				match buf.as_str() {
					"y\n" => break,
					"n\n" => bail!("could not acquire {}", DEFAULT_PATH),
					_ => {
						println!("invalid input {:?}", buf);
						buf.clear();
					}
				}
			}

			info!("downloading minecraft-b1.0.2.jar");

			{
				let mut download = get(DOWNLOAD_LINK).await?.bytes_stream();
				let mut file = File::create(DEFAULT_PATH).await?;

				while let Some(chunk) = download.next().await {
					let chunk = chunk?;
					file.write_all(&chunk).await?;
				}
			}

			Ok(JarReader::new(DEFAULT_PATH).await?)
		}
	}
}
