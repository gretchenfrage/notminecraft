
use reqwest::{
    Client,
    IntoUrl,
};
use bytes::Bytes;
use anyhow::{
    Result,
    ensure,
};


/// Perform an HTTP GET request, ensure the returned status code is OK, and
/// download all bytes in memory.
pub async fn get_success<U>(client: &mut Client, url: U) -> Result<Bytes>
where
    U: IntoUrl,
{
    let url = url.into_url()?;
    let response = client.get(url.clone()).send().await?;
    ensure!(
        response.status().is_success(),
        "http status code {:?} getting {:?}",
        response.status(),
        url,
    );
    Ok(response.bytes().await?)
}
