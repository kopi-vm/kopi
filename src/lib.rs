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

pub mod api;
pub mod archive;
pub mod cache;
pub mod commands;
pub mod config;
pub mod doctor;
pub mod download;
pub mod error;
pub mod installation;
pub mod logging;
pub mod metadata;
pub mod models;
pub mod platform;
pub mod security;
pub mod shim;
pub mod storage;
#[cfg(test)]
pub mod test;
pub mod uninstall;
pub mod user_agent;
pub mod version;
