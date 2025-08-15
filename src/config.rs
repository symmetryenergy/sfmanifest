
use crate::{Context, ToolContext};
use crate::current_operating_system;
use crate::slash;

// ENVIRONMENT
use std::env::current_exe;

// FILE SYSTEM
use std::fs as file_system;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

fn initialize_configurable_variables() -> Vec<String>
{
	let mut variable_names: Vec<String> = Vec::with_capacity(128);
	variable_names.push(String::from("bitbucket_username"));
	variable_names.push(String::from("bitbucket_app_password"));
	variable_names.push(String::from("bitbucket_workspace"));
	variable_names.push(String::from("bitbucket_repository"));
	variable_names.push(String::from("working_path"));
	return variable_names;
}

// Lists out the variables that can be configured within
// the configuration file
fn list_variables()
{
	let variables = initialize_configurable_variables();

	print!("\n\n==VARIABLES==\n");
	for variable in &variables 
	{ 
		print!("{}\n", variable); 
	}

	print!("\n\n");
}

fn read_arg(variable_key_value_string: &str) -> (String, String) {
	// We only want to split on the first occurance of '=' since a value such as a key might contain an '=' character.
    if let Some((key, value)) = variable_key_value_string.split_once("=") {
        (key.to_string(), value.to_string())
    } else {
        // If '=' is not found, treat the entire string as the key and assign an empty string as the value
        (variable_key_value_string.to_string(), String::new())
    }
}

pub fn config_root_path() -> String
{
	let mut length_of_exe_path_name: usize = 3;
	if current_operating_system == "windows" { length_of_exe_path_name = 7; }

	let mut config_path = current_exe()
		.unwrap_or_default()
		.display()
		.to_string();

	let mut config_path_revised: String = String::with_capacity(80);
	let mut characters_left: usize = config_path.len() - 1;
	for character in config_path.chars()
	{
		config_path_revised.push(character);
		if characters_left - length_of_exe_path_name == 0 { break; }
		characters_left -= 1;
	}

	config_path = config_path_revised;
	return config_path;
}

fn config_file_path() -> String
{
	let mut config_path = config_root_path();
	config_path.push_str("config.txt");
	return config_path;
}

fn get_config_file_content() -> String
{
	// Check if the configuration file exists
	let config_path = config_file_path();
	let config_path_exists: bool = Path::new(&config_path).exists();

	// Create the file if it doesn't exist.
	if !config_path_exists {
        // Create and initialize the file with default content if it doesn't exist
        let mut file = File::create(&config_path).unwrap();
        let default_content = r#"bitbucket_username=[enter value]
bitbucket_app_password=[enter value]
bitbucket_workspace=[enter value]
bitbucket_repository=[enter value]"#;

        file.write_all(default_content.as_bytes()).unwrap();
    }
	
	let mut config_file_content = String::with_capacity(2048);
	let mut config_file = File::open(config_path).unwrap();
	config_file.read_to_string(&mut config_file_content).unwrap();
	return config_file_content;
}

/// Prompts the user to enter their Bitbucket configuration values.
///
/// This function will prompt the user to enter their Bitbucket username, app password, workspace, and repository.
/// If these values are not already set in the `tool_context`'s configuration variables, the function will ask the user to enter them.
/// The entered values are then stored back into the `tool_context`'s configuration variables and written to a variable file.
///
/// # Arguments
///
/// * `_general_context` - A reference to the general context (currently unused).
/// * `tool_context` - A mutable reference to the tool context, which contains the configuration variables.
///
/// # Examples
///
/// ```no_run
/// let general_context = Context::new();
/// let mut tool_context = ToolContext::new();
/// prompt_for_config_values(&general_context, &mut tool_context);
/// ```
pub fn prompt_for_config_values(_general_context: &Context, tool_context: &mut ToolContext) 
{
	let mut bitbucket_username = tool_context.configuration_variables.get("bitbucket_username")
    	.unwrap_or(&String::from("[enter value]")).to_string();
	let mut bitbucket_app_password = tool_context.configuration_variables.get("bitbucket_app_password")
		.unwrap_or(&String::from("[enter value]")).to_string();
	let mut bitbucket_workspace = tool_context.configuration_variables.get("bitbucket_workspace")
		.unwrap_or(&String::from("[enter value]")).to_string();
	let mut bitbucket_repository = tool_context.configuration_variables.get("bitbucket_repository")
		.unwrap_or(&String::from("[enter value]")).to_string();

	if bitbucket_username == "[enter value]" { 
		print!("Please enter your Bitbucket username: ");
		bitbucket_username.clear();
		std::io::stdout().flush().unwrap();
		std::io::stdin().read_line(&mut bitbucket_username).unwrap();
	}

	if bitbucket_app_password == "[enter value]" {
		bitbucket_app_password.clear();
		print!("Please enter your Bitbucket app password: ");
		std::io::stdout().flush().unwrap();
		std::io::stdin().read_line(&mut bitbucket_app_password).unwrap();
	}

	if bitbucket_workspace == "[enter value]" {
		bitbucket_workspace.clear();
		print!("Please enter your Bitbucket workspace: ");
		std::io::stdout().flush().unwrap();
		std::io::stdin().read_line(&mut bitbucket_workspace).unwrap();
	}

	if bitbucket_repository == "[enter value]" {
		bitbucket_repository.clear();
		print!("Please enter your Bitbucket repository: ");
		std::io::stdout().flush().unwrap();
		std::io::stdin().read_line(&mut bitbucket_repository).unwrap();
	}

    println!("You entered: \nUsername: {}\nWorkspace: {}\nRepository: {}", 
        bitbucket_username.trim(), 
        bitbucket_workspace.trim(), 
        bitbucket_repository.trim());

	tool_context.configuration_variables.insert(String::from("bitbucket_username"), bitbucket_username.trim().to_string());
	tool_context.configuration_variables.insert(String::from("bitbucket_app_password"), bitbucket_app_password.trim().to_string());
	tool_context.configuration_variables.insert(String::from("bitbucket_workspace"), bitbucket_workspace.trim().to_string());
	tool_context.configuration_variables.insert(String::from("bitbucket_repository"), bitbucket_repository.trim().to_string());

	write_variable_file(_general_context, tool_context);
}

pub fn load_variables(_general_context: &Context, tool_context: &mut ToolContext)
{
	let config_file_content = get_config_file_content();

	if config_file_content.len() == 0
	{ return; }

	let config_file_content_lines: Vec<&str>= config_file_content.split("\n").collect();
	for line in &config_file_content_lines
	{
		// Used to avoid if there's a line that contains only a new
		// line character or new line plus space, or something similar
		if line.len() == 0 || line.len() == 1 { continue; }

		let (key, value) = read_arg(line);
		tool_context.configuration_variables.insert(key, value);
	}

	// If there is a different working path than the default entered within
	// the config parameters, then set that within ToolContext so that the 
	// program can run as though it's executing from a different folder.
	//
	// This would almost always be for running the program for someone's 
	// individual Salesforce directory at all times, no matter which currently
	// active directory they're in. 
	//
	// This is a special case that applies to the global running ToolContext
	// within the program, and not just a variable that is referenced within
	// one of the commands or something, so it makes some sense to have 
	// explicit handling for it here.
	if tool_context.configuration_variables.contains_key("working_path")
	{
		let working_path_as_entered = tool_context.configuration_variables.get("working_path").unwrap();

		if working_path_as_entered != &tool_context.working_path
		{ tool_context.working_path = working_path_as_entered.clone(); }
	}
}

fn set_variable(_general_context: &Context, 
	tool_context: &mut ToolContext,
	variable_argument: &String)
{
	let variable_arg_as_str = variable_argument.as_str();
	let (key, value) = read_arg(variable_arg_as_str);
	tool_context.configuration_variables.insert(key, value);

	write_variable_file(_general_context, tool_context);
}

fn write_variable_file(_general_context: &Context,
	tool_context: &mut ToolContext)
{
	let mut config_file_content: String = String::with_capacity(2048);
	for config_key in tool_context.configuration_variables.keys()
	{
		config_file_content.push_str(config_key);
		config_file_content.push('=');
		config_file_content.push_str(tool_context.configuration_variables.get_key_value(config_key).unwrap().1);
		config_file_content.push('\n');
	}

	let config_path = config_file_path();
	print!("config_path: {}\n", config_path);
	file_system::write(config_path, config_file_content).unwrap();
}

fn get_all(_general_context: &Context, tool_context: &mut ToolContext)
{
	let keys = tool_context.configuration_variables.keys();
	let keys_len = keys.len();
	print!("keys: {}\n", keys_len);
	for config_key in keys
	{
		let mut value: &String = &String::new();
		// Special exception case for bitbucket_app_password for security purposes
		if config_key == "bitbucket_app_password"
		{
			print!("{}=*******\n", config_key);
		}
		else
		{
			value = tool_context.configuration_variables.get_key_value(config_key).unwrap().1;
			print!("{}={}\n", config_key, value);
		}
		
	}
}

pub fn configure(general_context: &Context, tool_context: &mut ToolContext)
{
	if tool_context.command_parameters.contains_key("list_variables")
	{
		list_variables();
		return;
	}

	if tool_context.command_parameters.contains_key("get_all")
	{
		get_all(general_context, tool_context);
		tool_context.should_quit = true;
		return;
	}

	if tool_context.command_parameters.contains_key("variable_set")
	{
		let variable_arg = tool_context.command_parameters.get_key_value("variable_set").unwrap().1.clone();
		set_variable(general_context, tool_context, &variable_arg);
		tool_context.should_quit = true;
		return;
	}

	// Config commands should be completed by this point
	// and we should not allow the program to continue
	// once we go back into main
	tool_context.should_quit = true;
}