#![warn(clippy::all, rust_2018_idioms)]
#![warn(missing_debug_implementations, missing_copy_implementations)]

use futures_util::StreamExt;
use parking_lot::RwLock;
use reqwest::Client as ReqwestClient;
use serde::Deserialize;
use std::{convert::TryFrom, error::Error, fmt, sync::Arc, time::Instant};

pub type SpeedTestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub trait SpeedTestEvents {
    fn on_download(&mut self, target_download_information: &TargetDownloadInformation);
    fn on_downloading(&mut self, target_download_information: &TargetDownloadInformation);
    fn on_downloaded(&mut self, target_download_information: &TargetDownloadInformation);
}

pub struct SpeedTest {
    token: String,
    url_count: Option<u64>,
    client: Option<Client>,
    targets: Option<Vec<Target>>,
    hooks: Vec<Arc<RwLock<dyn SpeedTestEvents>>>,
}

impl fmt::Debug for SpeedTest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ token: {}, url_count: {:?}, client: {:?}, targets: {:?} }}",
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
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub time_elapsed: u128,
}

impl SpeedTest {
    pub fn new(token: &str) -> SpeedTest {
        SpeedTest {
            token: token.to_string(),
            url_count: None,
            client: None,
            targets: None,
            hooks: Vec::new(),
        }
    }

    fn get_reqwest_client(&self) -> SpeedTestResult<ReqwestClient> {
        ReqwestClient::builder().build().map_err(Into::into)
    }

    pub fn add_events_hook<S: SpeedTestEvents + 'static>(&mut self, hook: S) -> Arc<RwLock<S>> {
        let hook = Arc::new(RwLock::new(hook));
        self.hooks.push(hook.clone());
        hook
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

            for hook in &mut self.hooks {
                hook.write().on_download(&target_download_information);
            }

            let now = Instant::now();

            for target in targets {
                let mut stream = client.get(&target.url).send().await?.bytes_stream();

                while let Some(item) = stream.next().await {
                    target_download_information.bytes_downloaded += u64::try_from(item?.len())?;
                    target_download_information.time_elapsed = now.elapsed().as_nanos();

                    for hook in &mut self.hooks {
                        hook.write().on_downloading(&target_download_information);
                    }
                }
            }

            target_download_information.time_elapsed = now.elapsed().as_nanos();
        }

        for hook in &mut self.hooks {
            hook.write().on_downloaded(&target_download_information);
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

    #[test]
    fn speed_test_new_works() {
        let speed_test = SpeedTest::new(TOKEN);

        assert_eq!(speed_test.token, TOKEN);
        assert_eq!(speed_test.client, None);
        assert_eq!(speed_test.targets, None);
        assert_eq!(speed_test.url_count, None);
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
        let _target_download_information = speed_test.measure_download_speed().await?;

        Ok(())
    }

    #[tokio::test]
    async fn speed_test_add_hooks_works() -> SpeedTestResult<()> {
        let mut speed_test = SpeedTest::new(TOKEN);

        #[derive(Debug, Default)]
        struct Events {
            on_download: bool,
            on_downloading: bool,
            on_downloaded: bool,
        };

        impl SpeedTestEvents for Events {
            fn on_download(&mut self, _target_download_information: &TargetDownloadInformation) {
                self.on_download = true;
            }

            fn on_downloading(&mut self, _target_download_information: &TargetDownloadInformation) {
                self.on_downloading = true;
            }

            fn on_downloaded(&mut self, _target_download_information: &TargetDownloadInformation) {
                self.on_downloaded = true;
            }
        }

        let events: Events = Default::default();

        let events = speed_test.add_events_hook(events);
        speed_test.measure_download_speed().await?;

        assert_eq!(events.read().on_download, true);
        assert_eq!(events.read().on_downloading, true);
        assert_eq!(events.read().on_downloaded, true);

        Ok(())
    }
}
