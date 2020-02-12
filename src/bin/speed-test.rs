use indicatif::{ProgressBar, ProgressStyle};
use speed_test::{SpeedTest, SpeedTestEvents, SpeedTestResult, TargetDownloadInformation};
use std::cmp::min;
use std::thread;
use std::time::Duration;

#[derive(Debug, Default)]
struct SpeedTestProgressBar {
    progress_bar: Option<ProgressBar>,
}

impl SpeedTestEvents for SpeedTestProgressBar {
    fn on_download(&mut self, target_download_information: &TargetDownloadInformation) {
        let progress_bar = ProgressBar::new(target_download_information.total_bytes);
        progress_bar.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .progress_chars("#>-"));

        self.progress_bar = Some(ProgressBar::new(target_download_information.total_bytes));
    }

    fn on_downloading(&mut self, target_download_information: &TargetDownloadInformation) {
        if let Some(progress_bar) = &mut self.progress_bar {
            progress_bar.set_position(target_download_information.bytes_downloaded);
        }
    }

    fn on_downloaded(&mut self, _target_download_information: &TargetDownloadInformation) {
        if let Some(progress_bar) = &self.progress_bar {
            progress_bar.finish_with_message("Ending");
        }
    }
}

#[tokio::main]
async fn main() -> SpeedTestResult<()> {
    let mut speed_test = SpeedTest::new("YXNkZmFzZGxmbnNkYWZoYXNkZmhrYWxm");
    let speed_test_progress_bar: SpeedTestProgressBar = Default::default();
    speed_test.add_events_hook(speed_test_progress_bar);
    let results = speed_test.measure_download_speed().await?;

    Ok(())
}
