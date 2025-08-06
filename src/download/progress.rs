// Copyright 2025 dentsusoken
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::ProgressReporter;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

pub struct IndicatifProgressReporter {
    progress_bar: Option<ProgressBar>,
}

impl IndicatifProgressReporter {
    pub fn new() -> Self {
        Self { progress_bar: None }
    }
}

impl Default for IndicatifProgressReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl ProgressReporter for IndicatifProgressReporter {
    fn on_start(&mut self, total_bytes: u64) {
        let pb = if total_bytes > 0 {
            let pb = ProgressBar::new(total_bytes);
            pb.set_style(
                ProgressStyle::default_bar()
                    .template(
                        "{msg}\n{spinner:.green} [{elapsed_precise}] [{bar:25.cyan/blue}] \
                         {bytes}/{total_bytes} ({bytes_per_sec}, {eta})",
                    )
                    .unwrap()
                    .progress_chars("█▓░"),
            );
            pb.set_message("Downloading JDK");
            pb
        } else {
            let pb = ProgressBar::new_spinner();
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template(
                        "{msg}\n{spinner:.green} [{elapsed_precise}] {bytes} ({bytes_per_sec})",
                    )
                    .unwrap(),
            );
            pb.set_message("Downloading JDK (size unknown)");
            pb
        };

        pb.enable_steady_tick(Duration::from_millis(100));
        self.progress_bar = Some(pb);
    }

    fn on_progress(&mut self, bytes_downloaded: u64) {
        if let Some(pb) = &self.progress_bar {
            pb.set_position(bytes_downloaded);
        }
    }

    fn on_complete(&mut self) {
        if let Some(pb) = &self.progress_bar {
            pb.finish_with_message("Download complete");
        }
    }
}
