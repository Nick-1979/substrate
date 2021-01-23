// This file is part of Substrate.

// Copyright (C) 2020 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Types for working with tracing data

use serde::{Serialize, Deserialize};

use std::collections::HashMap;
use std::time::Duration;

/// Container for all related spans and events for the block being traced.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BlockTrace {
	/// Hash of the block being traced
	pub block_hash: String,
	/// Parent hash
	pub parent_hash: String,
	/// Module targets that were recorded by the tracing subscriber
	pub tracing_targets: String,
	/// Vec of tracing spans
	pub spans: Vec<Span>,
	/// Vec of tracing events
	pub events: Vec<Event>,
}

/// Represents a tracing event, complete with values
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Event {
	/// Event name
	pub name: String,
	/// Event target
	pub target: String,
	/// Timestamp relative to start of the tracing scope
	pub rel_timestamp: Duration,
	/// Associated `Values` of the Event
	pub values: Values,
	/// Parent id, if it exists
	pub parent_id: Option<u64>,
}

/// Represents a single instance of a tracing span
///
/// Exiting a span does not imply that the span will not be re-entered,
/// so there is a complete record of all entry & exit times
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Span {
	/// id for this span
	pub id: u64,
	/// id of the parent span, if any
	pub parent_id: Option<u64>,
	/// Name of this span
	pub name: String,
	/// Target, typically module
	pub target: String,
	/// Line number in source
	pub line: u32,
	/// List of timestamps when the span was entered
	pub entered: Vec<Duration>,
	/// List of timestamps when the span was exited
	pub exited: Vec<Duration>,
	/// Values recorded to this span
	pub values: Values,
}

/// Holds associated values for a tracing span
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Values {
	/// HashMap of `bool` values
	pub bool_values: HashMap<String, bool>,
	/// HashMap of `i64` values
	pub i64_values: HashMap<String, i64>,
	/// HashMap of `u64` values
	pub u64_values: HashMap<String, u64>,
	/// HashMap of `String` values
	pub string_values: HashMap<String, String>,
}
