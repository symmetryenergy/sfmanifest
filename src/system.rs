// ORCHESTRATION OF COMMANDS
use std::process::Command;

// sfmanifest
use crate::Context;
use crate::ToolContext;

// ENVIRONMENT 
use std::env::consts::OS as current_operating_system;

pub fn run_command(general_context: &mut Context, 
	tool_context: &mut ToolContext,
	directory: &String, 
	command: &String) -> (String, String)
{
	let run_command_message = format!("Running command: {}\n\n", command);
	
	general_context.logger.log_info(&run_command_message);

	let mut shell_program: String = String::new();
	let error_message = "failed to execute process";
	let mut first_argument: String = String::new();

	if current_operating_system == "linux"
	{
		shell_program = String::from("sh");
		first_argument = String::from("-c");
	}

	if current_operating_system == "windows"
	{
		shell_program = String::from("cmd");
		first_argument = String::from("/C");
	}

	let output = Command::new(shell_program)
		.arg(first_argument)
		.arg(command)
		.current_dir(directory)
		.output()
		.expect(error_message);

	let mut standard_out_as_string: String = String::new();
	let mut standard_error_as_string: String = String::new();


	for byte in output.stdout
	{
		let character = byte as char;

		if tool_context.printing_on
		{ print!("{}", character); }

		standard_out_as_string.push(character);
	}

	print!("\n");
	
	for byte in output.stderr
	{
		let character = byte as char;

		if tool_context.printing_on
		{ print!("{}", character); }

		standard_error_as_string.push(character);
	}

	return (standard_out_as_string, standard_error_as_string);

}