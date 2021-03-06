use core::ops::Deref;
use diesel::prelude::*;
use reqwest;
use reqwest::header::{Authorization, Basic};
use std::io;
use std::thread;
use std::time;

use db::{ConnectionSource, DB};
use db::ddns_config;
use errors;

#[derive(Clone, Debug, Deserialize, Insertable, PartialEq, Queryable, Serialize)]
#[table_name="ddns_config"]
pub struct DDNSConfig {
	pub host: String,
	pub username: String,
	pub password: String,
}

pub trait DDNSConfigSource {
	fn get_ddns_config(&self) -> errors::Result<DDNSConfig>;
}

impl DDNSConfigSource for DB {
	fn get_ddns_config(&self) -> errors::Result<DDNSConfig> {
		use self::ddns_config::dsl::*;
		let connection = self.get_connection();
		Ok(ddns_config
		       .select((host, username, password))
		       .get_result(connection.deref())?)
	}
}

#[derive(Debug)]
enum DDNSError {
	InternalError(errors::Error),
	IoError(io::Error),
	ReqwestError(reqwest::Error),
	UpdateError(reqwest::StatusCode),
}

impl From<io::Error> for DDNSError {
	fn from(err: io::Error) -> DDNSError {
		DDNSError::IoError(err)
	}
}

impl From<errors::Error> for DDNSError {
	fn from(err: errors::Error) -> DDNSError {
		DDNSError::InternalError(err)
	}
}

impl From<reqwest::Error> for DDNSError {
	fn from(err: reqwest::Error) -> DDNSError {
		DDNSError::ReqwestError(err)
	}
}

const DDNS_UPDATE_URL: &'static str = "https://ydns.io/api/v1/update/";


fn update_my_ip<T>(config_source: &T) -> Result<(), DDNSError>
	where T: DDNSConfigSource
{
	let config = config_source.get_ddns_config()?;
	if config.host.len() == 0 || config.username.len() == 0 {
		info!("Skipping DDNS update because credentials are missing");
		return Ok(());
	}

	let full_url = format!("{}?host={}", DDNS_UPDATE_URL, &config.host);
	let auth_header = Authorization(Basic {
	                                    username: config.username.clone(),
	                                    password: Some(config.password.to_owned()),
	                                });
	let client = reqwest::Client::new()?;
	let res = client
		.get(full_url.as_str())
		.header(auth_header)
		.send()?;
	if !res.status().is_success() {
		return Err(DDNSError::UpdateError(*res.status()));
	}
	Ok(())
}

pub fn run<T>(config_source: &T)
	where T: DDNSConfigSource
{
	loop {
		if let Err(e) = update_my_ip(config_source) {
			error!("Dynamic DNS update error: {:?}", e);
		}
		thread::sleep(time::Duration::from_secs(60 * 30));
	}
}
