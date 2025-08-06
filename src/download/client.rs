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

use crate::error::Result;
use crate::user_agent;
use attohttpc::{Response, Session};
use std::io::{self, Read};
use std::time::Duration;

pub trait HttpClient: Send + Sync {
    fn get(&self, url: &str, headers: Vec<(String, String)>) -> Result<Box<dyn HttpResponse>>;

    fn set_timeout(&mut self, timeout: Duration);
}

pub trait HttpResponse: Read + Send {
    fn status(&self) -> u16;

    fn header(&self, name: &str) -> Option<&str>;

    fn final_url(&self) -> Option<&str>;
}

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);

pub struct AttohttpcClient {
    timeout: Duration,
    user_agent: String,
}

impl AttohttpcClient {
    pub fn new() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            user_agent: user_agent::download_client(),
        }
    }
}

impl Default for AttohttpcClient {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpClient for AttohttpcClient {
    fn get(&self, url: &str, headers: Vec<(String, String)>) -> Result<Box<dyn HttpResponse>> {
        // Create a new session for each request
        let mut session = Session::new();
        session.proxy_settings(attohttpc::ProxySettings::from_env());

        // Build request with method chaining to avoid lifetime issues
        let mut request_builder = session
            .get(url)
            .timeout(self.timeout)
            .header("User-Agent", &self.user_agent)
            .follow_redirects(true);

        // For Range header specifically, we can use a match pattern
        // This avoids the generic loop that causes lifetime issues
        for (key, value) in headers {
            match key.as_str() {
                "Range" => {
                    // Range header is the only custom header we use for resume
                    let range_value = value.clone();
                    request_builder = request_builder.header("Range", range_value);
                }
                _ => {
                    // For other headers, we can add them as needed
                    // Currently, we only use Range header for resume functionality
                }
            }
        }

        let response = request_builder.send()?;
        Ok(Box::new(AttohttpcResponse { response }))
    }

    fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }
}

struct AttohttpcResponse {
    response: Response,
}

impl Read for AttohttpcResponse {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.response.read(buf)
    }
}

impl HttpResponse for AttohttpcResponse {
    fn status(&self) -> u16 {
        self.response.status().as_u16()
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.response.headers().get(name)?.to_str().ok()
    }

    fn final_url(&self) -> Option<&str> {
        Some(self.response.url().as_ref())
    }
}
