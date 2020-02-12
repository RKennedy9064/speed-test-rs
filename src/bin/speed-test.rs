use indicatif::ProgressBar;
use speed_test::{SpeedTest, SpeedTestResult, TargetDownloadInformation};

#[tokio::main]
async fn main() -> SpeedTestResult<()> {
    let mut speed_test = SpeedTest::new("YXNkZmFzZGxmbnNkYWZoYXNkZmhrYWxm");
    let results = speed_test.measure_download_speed().await?;

    Ok(())
}
