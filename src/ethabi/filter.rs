// Copyright 2015-2020 Parity Technologies
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::ethabi::{Hash, Token};
use serde::{Serialize, Serializer};
use serde_json::Value;
use std::ops;

/// Raw topic filter.
#[derive(Debug, PartialEq, Default)]
pub struct RawTopicFilter {
	/// Topic.
	pub topic0: Topic<Token>,
	/// Topic.
	pub topic1: Topic<Token>,
	/// Topic.
	pub topic2: Topic<Token>,
}

/// Topic filter.
#[derive(Debug, PartialEq, Default)]
pub struct TopicFilter {
	/// Usually (for not-anonymous transactions) the first topic is event signature.
	pub topic0: Topic<Hash>,
	/// Second topic.
	pub topic1: Topic<Hash>,
	/// Third topic.
	pub topic2: Topic<Hash>,
	/// Fourth topic.
	pub topic3: Topic<Hash>,
}

impl Serialize for TopicFilter {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		vec![&self.topic0, &self.topic1, &self.topic2, &self.topic3].serialize(serializer)
	}
}

/// Acceptable topic possibilities.
#[derive(Debug, PartialEq)]
pub enum Topic<T> {
	/// Match any.
	Any,
	/// Match any of the hashes.
	OneOf(Vec<T>),
	/// Match only this hash.
	This(T),
}

impl<T> Topic<T> {
	/// Map
	pub fn map<F, O>(self, f: F) -> Topic<O>
	where
		F: Fn(T) -> O,
	{
		match self {
			Topic::Any => Topic::Any,
			Topic::OneOf(topics) => Topic::OneOf(topics.into_iter().map(f).collect()),
			Topic::This(topic) => Topic::This(f(topic)),
		}
	}

	/// Returns true if topic is empty (Topic::Any)
	pub fn is_any(&self) -> bool {
		match *self {
			Topic::Any => true,
			Topic::This(_) | Topic::OneOf(_) => false,
		}
	}
}

impl<T> Default for Topic<T> {
	fn default() -> Self {
		Topic::Any
	}
}

impl<T> From<Option<T>> for Topic<T> {
	fn from(o: Option<T>) -> Self {
		match o {
			Some(topic) => Topic::This(topic),
			None => Topic::Any,
		}
	}
}

impl<T> From<T> for Topic<T> {
	fn from(topic: T) -> Self {
		Topic::This(topic)
	}
}

impl<T> From<Vec<T>> for Topic<T> {
	fn from(topics: Vec<T>) -> Self {
		Topic::OneOf(topics)
	}
}

impl<T> Into<Vec<T>> for Topic<T> {
	fn into(self: Self) -> Vec<T> {
		match self {
			Topic::Any => vec![],
			Topic::This(topic) => vec![topic],
			Topic::OneOf(topics) => topics,
		}
	}
}

impl Serialize for Topic<Hash> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let value = match *self {
			Topic::Any => Value::Null,
			Topic::OneOf(ref vec) => {
				let v = vec.iter().map(|h| format!("0x{:x}", h)).map(Value::String).collect();
				Value::Array(v)
			}
			Topic::This(ref hash) => Value::String(format!("0x{:x}", hash)),
		};
		value.serialize(serializer)
	}
}

impl<T> ops::Index<usize> for Topic<T> {
	type Output = T;

	fn index(&self, index: usize) -> &Self::Output {
		match *self {
			Topic::Any => panic!("Topic unavailable"),
			Topic::This(ref topic) => {
				if index != 0 {
					panic!("Topic unavailable");
				}
				topic
			}
			Topic::OneOf(ref topics) => topics.index(index),
		}
	}
}
