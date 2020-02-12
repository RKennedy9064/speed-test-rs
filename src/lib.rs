#![warn(clippy::all, rust_2018_idioms)]
#![warn(missing_debug_implementations, missing_copy_implementations)]

use futures_util::StreamExt;
use reqwest::Client as ReqwestClient;
use serde::Deserialize;
use std::{convert::TryFrom, error::Error, fmt, time::Instant};

pub type SpeedTestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Default)]
pub struct SpeedTest {
    token: String,
    url_count: Option<u64>,
    client: Option<Client>,
    targets: Option<Vec<Target>>,
    on_download: Option<fn(&TargetDownloadInformation) -> TargetDownloadInformation>,
    on_downloading: Option<fn(&TargetDownloadInformation)>,
}

impl fmt::Debug for SpeedTest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SpeedTest {{ token: {}, url_count: {:?}, client: {:?}, targets: {:?} }}",
            self.token, self.url_count, self.client, self.targets
        )
    }
}

#[derive(Debug, PartialEq, Deserialize)]
struct Client {
    asn: String,
    isp: String,
    location: Location,
    ip: String,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Location {
    country: String,
    city: String,
}

#[derive(Debug, PartialEq, Deserialize)]
struct Target {
    url: String,
    location: Location,
    name: String,
}

#[derive(Debug, Default, Copy, Clone)]
pub struct TargetDownloadInformation {
    bytes_downloaded: u64,
    total_bytes: u64,
    time_elapsed: u128,
}

impl SpeedTest {
    pub fn new(token: &str) -> SpeedTest {
        SpeedTest {
            token: token.to_string(),
            ..Default::default()
        }
    }

    pub fn set_on_download(&mut self, on_download: fn(&TargetDownloadInformation) -> TargetDownloadInformation) {
        println!("{:?}", on_download as usize);
        self.on_download = Some(on_download);
    }

    pub fn set_on_downloading(&mut self, on_downloading: fn(&TargetDownloadInformation)) {
        self.on_downloading = Some(on_downloading);
    }

    fn get_reqwest_client(&self) -> SpeedTestResult<ReqwestClient> {
        ReqwestClient::builder().build().map_err(Into::into)
    }

    pub async fn measure_download_speed(&mut self) -> SpeedTestResult<TargetDownloadInformation> {
        let client = self.get_reqwest_client()?;
        let mut target_download_information: TargetDownloadInformation = Default::default();

        self.setup_api().await?;

        if let Some(targets) = &self.targets {
            for target in targets {
                let content_length = client
                    .head(&target.url)
                    .send()
                    .await?
                    .content_length()
                    .ok_or_else(|| format!("Could not read content-length from {}", target.name))?;
                target_download_information.total_bytes += content_length;
            }

            if let Some(on_download) = self.on_download {
                on_download(&target_download_information);
            }

            let now = Instant::now();

            for target in targets {
                let mut stream = client.get(&target.url).send().await?.bytes_stream();

                while let Some(item) = stream.next().await {
                    target_download_information.bytes_downloaded += u64::try_from(item?.len())?;

                    if let Some(on_downloading) = self.on_downloading {
                        on_downloading(&target_download_information);
                    }
                }
            }

            target_download_information.time_elapsed = now.elapsed().as_nanos();
        }

        Ok(target_download_information)
    }

    async fn setup_api(&mut self) -> SpeedTestResult<()> {
        let url_count = match self.url_count {
            Some(url_count) => url_count,
            None => 5,
        };

        let url = format!(
            "https://api.fast.com/netflix/speedtest/v2?=https=true&token={}&urlCount={}",
            self.token, url_count
        );

        #[derive(Deserialize)]
        struct Response {
            client: Client,
            targets: Vec<Target>,
        }

        let response = reqwest::get(&url).await?.json::<Response>().await?;

        self.client = Some(response.client);
        self.targets = Some(response.targets);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOKEN: &str = "YXNkZmFzZGxmbnNkYWZoYXNkZmhrYWxm";

    fn on_download(target_download_information: &TargetDownloadInformation) -> TargetDownloadInformation {
        *target_download_information
    }

    fn on_downloading(_target_download_information: &TargetDownloadInformation) {}

    #[test]
    fn speed_test_new_works() {
        let speed_test = SpeedTest::new(TOKEN);

        assert_eq!(speed_test.token, TOKEN);
        assert_eq!(speed_test.client, None);
        assert_eq!(speed_test.targets, None);
        assert_eq!(speed_test.url_count, None);
    }

    #[test]
    fn speed_test_set_on_download_works() {
        let mut speed_test = SpeedTest::new(TOKEN);
        speed_test.set_on_download(on_download);

        assert_eq!(speed_test.on_download.is_some(), true);
    }

    #[test]
    fn speed_test_set_on_downloading_works() {
        let mut speed_test = SpeedTest::new(TOKEN);
        speed_test.set_on_downloading(on_downloading);

        assert_eq!(speed_test.on_downloading.is_some(), true);
    }

    #[tokio::test]
    async fn speed_test_setup_api_works() -> SpeedTestResult<()> {
        let mut speed_test = SpeedTest::new(TOKEN);
        speed_test.setup_api().await?;

        assert_eq!(speed_test.client.is_some(), true);
        assert_eq!(speed_test.targets.is_some(), true);

        Ok(())
    }

    #[tokio::test]
    async fn speed_test_measure_download_speed_works() -> SpeedTestResult<()> {
        let mut speed_test = SpeedTest::new(TOKEN);
        speed_test.set_on_download(on_download);
        speed_test.set_on_downloading(on_downloading);

        let _target_download_information = speed_test.measure_download_speed().await?;

        Ok(())
    }
}
