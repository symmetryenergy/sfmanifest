// ENVIRONMENT
use std::env::args as command_line_arguments;
use std::env::current_dir as current_working_directory;
use std::env::consts::OS as current_operating_system;

// TIME
use std::time::{Instant,Duration};

// MODULES
mod bitbucket;
mod common;
mod config;
mod manifest;
mod options;
mod system;

// ELEGA CORE
use common::{Context, Logger, TemporaryStorage};

// JSON handling, used to parse sfdx-project.json
use serde_json::{json, Value};

// COLLECTION TYPES
use std::collections::{HashSet, HashMap};

use crate::options::Automation;

#[derive(Clone)]
pub struct ToolContext
{
	should_quit: bool,

	printing_on: bool,

	working_path: String,

	command_parameters: HashMap<String, String>,
	configuration_variables: HashMap<String, String>,

	time_snapshots: Vec<String>, // Captures performance related information and prints at end of program
}

impl ToolContext
{
	pub fn new() -> ToolContext
	{
		ToolContext
		{
			should_quit: false,

			printing_on: true,

			working_path: current_working_directory().unwrap().display().to_string(),

			command_parameters: HashMap::new(),
			configuration_variables: HashMap::new(),

			time_snapshots: Vec::with_capacity(64)
		}
	}
}

fn slash() -> char
{
	if current_operating_system == "linux" { return '/'; }
	else { return '\\'; }
}

fn configure_general_context() -> Context
{
	let mut context_logger: Logger = Logger::new();
	context_logger.print_all_on = true;
	context_logger.print_asap = true;

	let mut logging_directory = current_working_directory()
		.unwrap()
		.display()
		.to_string();

	logging_directory.push(slash());
	logging_directory.push_str("log.txt");

	context_logger.file_path = logging_directory;

	return Context{storage: TemporaryStorage::new(), logger: context_logger};
}

pub fn configure_tool_context(tool_context: &mut ToolContext,
	options: &options::Opt)
{
	if options.list_supported_mode
	{
		manifest::list_supported_metadata(tool_context);
		tool_context.should_quit = true;
		return;
	}

	// BITBUCKET USER
	let user_key: String = String::from("bbuser");
	let user_available: bool = options.bitbucket_user.is_some();

	if user_available
	{
		let user_value: String = options.bitbucket_user.clone().unwrap();
		tool_context.command_parameters.insert(user_key, user_value);
	}

	// COMPARISON BRANCH
	let branch_key: String = String::from("branch");
	tool_context.command_parameters.insert(branch_key, options.branch.clone());

	// STRING ONLY PRINTING
	let string_only_key: String = String::from("stringonly");

	if options.string_only
	{
		tool_context.command_parameters.insert(string_only_key, String::from("--string-only"));
	}

	// NO CLEAN?
	let no_clean_key: String = String::from("noclean");

	if options.no_clean
	{
		tool_context.command_parameters.insert(no_clean_key, String::from("--noclean"));
	}

	// SUPPORTED
	let supported_key: String = String::from("supported");

	if options.list_supported_mode
	{
		tool_context.command_parameters.insert(supported_key, String::from("--supported"));
	}

	// GIT
	let git_key: String = String::from("git");

	if options.automation == Automation::Git
	{
		tool_context.command_parameters.insert(git_key, String::from("--git"));
	}

	// CONFIG SET
	let config_set_key: String = String::from("variable_set");
	let variable_to_set_available: bool = options.config_set.is_some();

	if variable_to_set_available
	{
		let variable_set_value: String = options.config_set.clone().unwrap();
		tool_context.command_parameters.insert(config_set_key, variable_set_value);
	}
	
	// CONFIG GET ALL
	let config_get_all_key: String = String::from("get_all");
	if options.config_get_all
	{
		tool_context.command_parameters.insert(config_get_all_key, String::from("--get-all"));
	}

	// FEATURE
	let feature_key: String = String::from("feature");
	let feature_available: bool = options.feature.is_some();
	
	if feature_available
	{
		let feature: String = options.feature.clone().unwrap();
		tool_context.command_parameters.insert(feature_key, feature);
	}
}

fn main() 
{
	let start_time: Instant = Instant::now(); // Begin tracking program run time

	// Command line arguments and program configuration
	let options: options::Opt = options::Opt::new();

	// General context is used for the logger and may apply to usage of the
	// TemporaryStorage struct, which can be used to hold bytes on the stack
	let general_context: &mut Context = &mut configure_general_context();

	// The ToolContext instance gets carried throughout the program just like the
	// general context does... but it serves the purpose of holding all the config
	// delivered from command line arguments regarding what commands are being run.
	let tool_context: &mut ToolContext = &mut ToolContext::new();

	configure_tool_context(tool_context, &options);

	if tool_context.should_quit
	{ return; }

	// A configuration file at the location of the .exe is created to store
	// values such as the bitbucket_username (which is used in manifest command), 
	// or other useful parameters that apply to other commands.
	config::load_variables(general_context, tool_context);

	// If there are configuration commands to run, we're going to pause here
	// to run them and then exit
	config::configure(general_context, tool_context);

	if tool_context.should_quit
	{ return; }

	// Assuming either config.txt has loaded everything needed OR everything has
	// been specified in command line args necessary for running, one last check
	// will take place for checking config variables and will prompt the user to
	// enter them if they're not in-memory.
	config::prompt_for_config_values(general_context, tool_context);

	// Main logic for manifest generation finally proceeds!
	manifest::generate_manifest(general_context, tool_context);

	// The total run time of interest ends here, and the * 1000.0 converts this from f64 
	// seconds expressed as milliseconds.
	let total_time: f64 = start_time.elapsed().as_secs_f64() * 1000.0;

	let total_time_message = format!("Program completed in {}ms\n", total_time);
	tool_context.time_snapshots.push(total_time_message);

	// Print performance info based on whatever was pushed into the Vec<String> on the 
	// tool_context.time_snapshots collection
	general_context.logger.log_info("\n\n== Time Snapshots ==\n\n");
	for time_snapshot in &tool_context.time_snapshots
	{
		general_context.logger.log_info(time_snapshot);
	}

	// This can be commented out or otherwise flagged into a paremeter if it is not necessary
	// to create a log.txt file at the end of the run to hold whatever was printed to the
	// terminal from the general context logger.
	general_context.logger.publish();

}
