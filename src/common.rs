// Below copyright notice applies only to elega_core:
// 
// Copyright © 2021 Elega Corporation.
// 
// Permission is hereby granted, free of charge, to any person obtaining a 
// copy of this software and associated documentation files (the “Software”), 
// to deal in the Software without restriction, including without limitation 
// the rights to use, copy, modify, merge, publish, distribute, sublicense, 
// and/or sell copies of the Software, and to permit persons to whom the 
// Software is furnished to do so, subject to the following conditions:
// 
// The above copyright notice and this permission notice shall be included 
// in all copies or substantial portions of the Software.
// 
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS 
// OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, 
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL 
// THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER 
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING 
// FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER 
// DEALINGS IN THE SOFTWARE.

// Context and TemporaryStorage are based on ideas that come standard with Thekla Inc's 
// Jai programming language. See Jonathan Blow's session on discussing how Jai implemented
// the overall context and temporary storage concepts as a part of its normal language features
// in this video: 
// https://www.youtube.com/watch?v=SSVHWrYG974
// 
// The below code does not perfectly replicate what is done by Blow in the above link, and is 
// not meant to precisely. Instead, these are meant to serve as useful structures as they apply
// to projects using this code written in Rust. The below code likely does not make use of 
// pointers or dynamic allocation in the same way as Blow does so in the Jai language.
// 
// 																		-Scott L.

#![allow(dead_code)]

use std::{fs as filesystem};
use std::path::Path;
use std::env::current_dir as current_directory;

use chrono::{DateTime, Local};

pub struct Context
{
	pub storage: TemporaryStorage,
	pub logger: Logger,
}

const DEFAULT_STORAGE_SIZE: usize = 40960;
const DEFAULT_HEAPBLOCK_SIZE: usize = 40960000; // 40.96MB
const DEFAULT_ALLOCATOR_SIZE: usize = 7168;
const DEFAULT_LOGGING_SIZE:usize = 4096;
const NULL: u8 = b'\0';

const LOG_LEVEL_INFO: u8 = 0x0;
const LOG_LEVEL_ERROR: u8 = 0x1;
const LOG_LEVEL_VERBOSE: u8 = 0x2;

// TODO: Complete a macro that allows heap allocated strings to be 
// merged together into a value similar to how print! can accomplish 
// this but send to stdout. 
macro_rules! string_merge {
	($e:expr, String) => {
		let string_message: &String = &$e;
		

	};
}

pub struct LogMessage
{
	time: DateTime<Local>,
	level: u8,
	message: String,
}

pub struct Logger
{
	// Overall configuration of the logger
	pub print_all_on: bool,
	pub file_path: String,
	pub print_asap: bool, // Whether to print as soon as possible

	// Enable flags for different settings of log
	// messages at different levels
	// pub print_info: bool,
	// pub save_info: bool,
// 
	// pub print_error: bool,
	// pub save_error: bool,
// 
	// pub print_verbose: bool,
	// pub save_verbose: bool,

	// Storage structures for the log message strings
	messages: Vec<LogMessage>
}

impl Logger
{
	pub fn new() -> Logger
	{
		Logger
		{
			print_all_on: false,
			file_path: String::new(),
			print_asap: false,

			// print_info: false,
			// save_info: false,
			// print_error: false,
			// save_error: false, 
			// print_verbose: false,
			// save_verbose: false,
			
			messages: Vec::with_capacity(DEFAULT_LOGGING_SIZE),
		}
	}

	pub fn log(&mut self, message: &str, level: u8)
	{
		let message_to_log = (*message).to_string(); // Whatttt?
		if self.print_asap { print!("{}", message_to_log); }

		let log_message: LogMessage = LogMessage
		{ time: Local::now(), level: level, message: String::from(message_to_log) };

		self.messages.push(log_message); 
	}

	pub fn log_info(&mut self, message: &str)
	{
		self.log(message, LOG_LEVEL_INFO);
	}

	pub fn log_error(&mut self, message: &str)
	{
		self.log(message, LOG_LEVEL_ERROR);
	}

	pub fn log_verbose(&mut self, message: &str)
	{
		self.log(message, LOG_LEVEL_VERBOSE);
	}

	pub fn publish(&mut self)
	{
		if self.file_path.len() == 0
		{
			print!("The logger file path is not set, so there's nowhere to save the log statements. \
			You might want to set the file_path property of the Logger struct to save a file somewhere.\n");
		}

		let mut log_file_content: String = String::new();
		for message in &self.messages
		{
			log_file_content.push_str(&message.time.to_rfc3339());

			log_file_content.push(' ');
			log_file_content.push('|');
			log_file_content.push(' ');

			if message.level == LOG_LEVEL_INFO { log_file_content.push_str("INFO"); }
			else if message.level == LOG_LEVEL_ERROR { log_file_content.push_str("ERROR"); }
			else if message.level == LOG_LEVEL_VERBOSE { log_file_content.push_str("VERBOSE"); }
			else { log_file_content.push_str("UNKNOWN"); }

			log_file_content.push(' ');
			log_file_content.push('|');
			log_file_content.push(' ');

			log_file_content.push_str(&message.message);
		}

		filesystem::write(&self.file_path, log_file_content);
	}
}

pub struct HeapStorageBlock
{
	pub data: Vec<u8>,
	pub allocator: Allocator,
	pub occupied: i32,
	pub max_usage: i32,
}

impl HeapStorageBlock
{
	pub fn new() -> HeapStorageBlock
	{
		HeapStorageBlock 
		{
			data: Vec::with_capacity(DEFAULT_HEAPBLOCK_SIZE),
			allocator: Allocator { index: 0, data: [NULL; DEFAULT_ALLOCATOR_SIZE] },
			occupied: 0,
			max_usage: 0
		}
	}

	fn bump(&mut self)
	{
		self.allocator.index += 1;
		self.occupied += 1;
		self.max_usage += 1;
	}

	pub fn add_byte(&mut self, data_being_added: &u8) -> usize
	{
		let data_at_index = self.allocator.index;
		self.data.push(*data_being_added);
		self.bump();
		return data_at_index;
	}

	pub fn add_byte_vec(&mut self, data_being_added: &Vec<u8>) -> usize
	{
		let data_at_index = self.allocator.index;
		for data_item in data_being_added 
		{ 
			self.data.push(*data_item);
			self.allocator.index += 1;
			self.bump();
		}

		return data_at_index;
	}
}

pub struct Allocator
{
	pub index: usize,
	pub data: [u8; DEFAULT_ALLOCATOR_SIZE],
}

pub struct TemporaryStorage
{
	pub data: [u8; DEFAULT_STORAGE_SIZE],
	pub occupied: i32,
	pub max_usage: i32,
	pub allocator: Allocator,
}

impl TemporaryStorage
{
	pub fn new() -> TemporaryStorage
	{
		TemporaryStorage
		{
			data: [NULL; DEFAULT_STORAGE_SIZE],
			occupied: 0,
			max_usage: 0,
			allocator: Allocator{index: 0, data: [NULL; DEFAULT_ALLOCATOR_SIZE]}
		}
	}

	pub fn reset_storage(&mut self)
	{
		self.occupied = 0;
		self.max_usage = 0;
		self.allocator.index = 0;
	}

	fn bump(&mut self)
	{
		self.allocator.index += 1;
		self.occupied += 1;
	}

	pub fn add_byte(&mut self, data_being_added: &u8) -> usize
	{
		let data_at_index = self.allocator.index;
		self.data[self.allocator.index] = *data_being_added;
		self.bump();
		return data_at_index;
	}

	pub fn add_byte_vec(&mut self, data_being_added: &Vec<u8>) -> usize
	{
		let data_at_index = self.allocator.index;
		for data_item in data_being_added 
		{ 
			self.data[self.allocator.index] = *data_item;
			self.bump();
		}

		return data_at_index;
	}
}