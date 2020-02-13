use indicatif::{ProgressBar, ProgressStyle};
use speed_test::{SpeedTest, SpeedTestEvents, SpeedTestResult, TargetDownloadInformation};

#[derive(Debug)]
struct SpeedTestProgressBar {
    progress_bar: ProgressBar,
}

impl SpeedTestProgressBar {
    pub fn new() -> SpeedTestProgressBar {
        let progress_bar = ProgressBar::new(0);
        progress_bar.set_style(ProgressStyle::default_bar()
            .template("{msg:>12.cyan.bold} {spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
            .progress_chars("#>-"));

        SpeedTestProgressBar { progress_bar }
    }
}

impl SpeedTestEvents for SpeedTestProgressBar {
    fn on_download(&mut self, target_download_information: &TargetDownloadInformation) {
        self.progress_bar
            .set_length(target_download_information.total_bytes);
    }

    fn on_downloading(&mut self, target_download_information: &TargetDownloadInformation) {
        self.progress_bar
            .set_position(target_download_information.bytes_downloaded);
    }

    fn on_downloaded(&mut self, _target_download_information: &TargetDownloadInformation) {
        self.progress_bar.finish_with_message("Finished");
    }
}

#[tokio::main]
async fn main() -> SpeedTestResult<()> {
    let mut speed_test = SpeedTest::new("YXNkZmFzZGxmbnNkYWZoYXNkZmhrYWxm");
    let speed_test_progress_bar = SpeedTestProgressBar::new();

    speed_test.add_events_hook(speed_test_progress_bar);
    speed_test.measure_download_speed().await?;

    Ok(())
}
