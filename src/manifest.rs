

use std::thread::current;
use std::time::{Instant};

// FILE SYSTEM
use std::fs as file_system;

// ENVIRONMENT
use std::env::current_dir as current_working_directory;
use std::env::join_paths;
use tokio::runtime::Runtime;
use std::env::consts::OS as current_operating_system;

// COLLECTIONS
use std::collections::{HashMap, HashSet};

// ELEGA CORE
use crate::common::{Context};

// MULTI-CORE PARALLELISM
use rayon::prelude::*;

// ToolContext carries the main command line arguments and other
// input parameters
use crate::system::run_command;
use crate::configure_general_context;
use crate::ToolContext;
use crate::slash;
use crate::bitbucket::Bitbucket;

const MAXIMUM_DIFF_FILE_SIZE: usize = 5000;
const DEFAULT_COMPARE_BRANCH: &str = "qa";
const FEATURE_BRANCH_TEMP_FOLDER: &str = "_feature_branch_temp";
const COMPARE_BRANCH_TEMP_FOLDER: &str = "_compare_branch_temp";

const WHITESPACE: char = ' ';

pub struct ManifestBundle
{
	pub manifest: String,
	pub destructive_manifest: String,
}

impl ManifestBundle
{
	pub fn new() -> ManifestBundle
	{
		ManifestBundle { manifest: String::new(), destructive_manifest: String::new() }
	}
}

// Each metadata bucket contains a key it is identified as 
// in the file system, its name in a package.xml file, 
// and a list of files identified from a git diff
pub struct MetadataBucket
{
	pub file_path_name: String,
	pub package_xml_name: String,
	pub files: HashSet<String>,
	pub destructive_files: HashSet<String>,
	pub bundle: bool,
}

impl MetadataBucket
{
	pub fn new(file_path_name: &str, package_xml_name: &str, bundle: bool) -> MetadataBucket
	{
		MetadataBucket
		{
			file_path_name: String::from(file_path_name),
			package_xml_name: String::from(package_xml_name),
			files: HashSet::with_capacity(64),
			destructive_files: HashSet::with_capacity(64),
			
			// In the case of bundles, we take the name of the preceding folder and not the file,
			// such as lwc/ComponentName/componentName.js
			//
			// We'd ignore the .js file above and simply take 'ComponentName' as the bundle name
			// to retrieve, and that's what makes its way into the manifest.
			bundle: bundle, 
		}
	}
}

pub struct RepositoryInfo
{
	pub folder_name: String,
	pub branch_name: String,
	pub folder_path_as_string: String,
}

fn create_new_folder(working_path: &String,
	folder_name: &String) -> String
{
	let mut current_working_dir = working_path.clone();
	current_working_dir.push('/');
	let os_string = join_paths([current_working_dir.clone(),folder_name.to_string()]).unwrap();
	let mut path = String::from(os_string.to_str().unwrap());
	
	if current_operating_system == "linux" { path = path.replace(":", ""); }
	else if current_operating_system == "windows" { path = path.replace(";", ""); }

	let path_cloned = path.clone();
	print!("path_cloned: {}\n", path_cloned);
	let _feature_folder_result = file_system::create_dir(path).unwrap_or_default();
	return String::from(path_cloned);
}

fn run_pull(tool_context: &mut ToolContext,
	repo_path: &String, branch_name: &String)
{
	let general_context = &mut configure_general_context();
	general_context.logger.file_path = general_context.logger.file_path.replace("log.txt", "git_log.txt");
	
	let bitbucket_username: &String = tool_context.configuration_variables.get_key_value("bitbucket_username").unwrap().1;
	let bitbucket_workspace: &String = tool_context.configuration_variables.get_key_value("bitbucket_workspace").unwrap().1;
	let bitbucket_repository: &String = tool_context.configuration_variables.get_key_value("bitbucket_repository").unwrap().1;

	let git_init_command: &String = &String::from("git init");
	let origin_url: String = format!("https://{}@bitbucket.org/{}/{}.git", bitbucket_username, 
		bitbucket_workspace, 
		bitbucket_repository);
	let git_remote_add_origin_command = &format!("git remote add origin {}", origin_url);
	
	let git_fetch_command = &String::from("git fetch");
	let git_checkout_branch_command = &format!("git checkout -q {}", branch_name);

	print!("repo_path: {}\n", repo_path);

	// Empty ToolContext that's created as a part of reqeuired arguments...
	// but this isn't used in this case and doesn't really matter for our
	// purposes
	let empty_tool_context: &mut ToolContext = &mut ToolContext::new();

	run_command(general_context, empty_tool_context, repo_path, git_init_command);
	run_command(general_context, empty_tool_context, repo_path, git_remote_add_origin_command);
	run_command(general_context, empty_tool_context, repo_path, git_fetch_command);
	run_command(general_context, empty_tool_context, repo_path, git_checkout_branch_command);
}

pub fn pull_branch_details(tool_context: &mut ToolContext,
	bitbucket_username: String, 
	repository_info: &RepositoryInfo)
{
	let working_path: &String = &tool_context.working_path;
	create_new_folder(working_path, &repository_info.folder_name);
	run_pull(tool_context, &repository_info.folder_path_as_string, &repository_info.branch_name);
}

fn branch_names(general_context: &mut Context, tool_context: &mut ToolContext) -> (String, String)
{
	// First, determine the feature branch and compare branch. How the feature branch differs from the compare branch
	// determines which files will make their way into a manifest
	let mut feature_branch: &String = &String::from("");
	let (standard_out_from_git, standard_error_from_git) = run_command(
		general_context, 
		tool_context,
		&tool_context.working_path.clone(), //  TODO: See if clone is avoidable
		&String::from("git symbolic-ref --short -q HEAD")
	);
	let feature_branch_from_git = &standard_out_from_git.clone();

	if tool_context.command_parameters.contains_key("feature")
	{
		feature_branch = &tool_context.command_parameters.get_key_value("feature").unwrap().1;
	}
	else // If no branch specified in argument, check current working directory for branch using 'git branch'
	{
		if feature_branch_from_git.len() > 0
		{
			feature_branch = &feature_branch_from_git;
		}
		
		if standard_error_from_git.len() > 0
		{
			print!("WARNING: An error was encountered when trying to retrieve the current branch.\n\n{}\n", standard_error_from_git);
		}
	}
	print!("feature branch: {}\n", feature_branch);

	let mut compare_branch: &String = &String::from(DEFAULT_COMPARE_BRANCH); // Default
	if tool_context.command_parameters.contains_key("branch")
	{
		compare_branch = &tool_context.command_parameters.get_key_value("branch").unwrap().1;
	}
	print!("compare_branch: {}\n", compare_branch);

	return (feature_branch.clone(), compare_branch.clone());
}

fn initialize_repository_information(general_context: &mut Context,
	tool_context: &mut ToolContext,
	feature_branch: &String,
	compare_branch: &String) -> ([RepositoryInfo; 2], String, String)
{
	let file_setup_start_time: Instant = Instant::now();

	let mut feature_branch_folder_name: String = String::with_capacity(1 + FEATURE_BRANCH_TEMP_FOLDER.len());
	feature_branch_folder_name.push(slash());
	feature_branch_folder_name.push_str(FEATURE_BRANCH_TEMP_FOLDER);

	let mut compare_branch_folder_name = String::with_capacity(1 + COMPARE_BRANCH_TEMP_FOLDER.len());
	compare_branch_folder_name.push(slash());
	compare_branch_folder_name.push_str(COMPARE_BRANCH_TEMP_FOLDER);

	let mut feature_branch_path = String::from(join_paths([tool_context.working_path.clone(), 
		feature_branch_folder_name.clone()])
		.unwrap() // At this point, successful PathBuf created
		.as_os_str() // OsString is an ASCII string that is not formatted as UTF-8
		.to_str() // Converts to str type
		.unwrap()); // Success converting to str type (or not, in which case panic)

	let mut compare_branch_path = String::from(join_paths([tool_context.working_path.clone(),
		compare_branch_folder_name.clone()])
		.unwrap()
		.as_os_str()
		.to_str()
		.unwrap());

	if current_operating_system == "linux"
	{
		// Remove trailing ':' character that comes from join_paths() above
		feature_branch_path = feature_branch_path.replace(":", "");
		compare_branch_path = compare_branch_path.replace(":", "");
	}
	else if current_operating_system == "windows"
	{
		// Apparently, on Windows, it uses ';' instead of ':' because of course it does
		feature_branch_path = feature_branch_path.replace(";", "");
		compare_branch_path = compare_branch_path.replace(";", "");
	}

	general_context.logger.log_info(&format!("feature_branch_path: {}\n", feature_branch_path));
	general_context.logger.log_info(&format!("compare_branch_path: {}\n", compare_branch_path));

	let feature_branch_repo_info = RepositoryInfo
	{
		folder_name: feature_branch_folder_name.clone(), 
		branch_name: feature_branch.clone(), 
		folder_path_as_string: feature_branch_path.clone()
	};

	let compare_branch_repo_info = RepositoryInfo
	{
		folder_name: compare_branch_folder_name.clone(), 
		branch_name: compare_branch.clone(),
		folder_path_as_string: compare_branch_path.clone()
	};

	let repository_information = [
		feature_branch_repo_info, compare_branch_repo_info
	];

	let file_setup_time = file_setup_start_time.elapsed().as_secs_f64() * 1000.0;
	let file_setup_time_message: String = String::from(format!("manifest::file setup: {}ms\n", file_setup_time));
	tool_context.time_snapshots.push(file_setup_time_message);

	return (repository_information, feature_branch_path, compare_branch_path);
}

fn manage_branches(tool_context: &mut ToolContext, repository_information: &[RepositoryInfo; 2])
{
	let git_pulling_start_time: Instant = Instant::now();

	let mut bitbucket_username: &String = &String::new();

	if tool_context.configuration_variables.contains_key("bitbucket_username")
	{
		bitbucket_username = tool_context.configuration_variables.get_key_value("bitbucket_username").unwrap().1;
	}
	else
	{
		bitbucket_username = tool_context.command_parameters.get_key_value("bbuser").unwrap().1;
	}

	// TODO: Working path must be made to work with this parallel pulling action
	// The problem is that tool_context.working_path, or reading from it across
	// multiple threads, isn't safe, so this needs some additional thought
	repository_information
		.par_iter()
		.for_each(
			|repository_info| pull_branch_details(&mut tool_context.clone(), 
				bitbucket_username.clone(), 
				&repository_info));

	let git_pulling_time: f64 = git_pulling_start_time.elapsed().as_secs_f64() * 1000.0;
	let git_pulling_time_message: String = String::from(format!("manifest::git pulling: {}ms\n", git_pulling_time));
	tool_context.time_snapshots.push(git_pulling_time_message);
}

pub fn split_to_lines_vec(diffed_files_from_standard_out: &String) -> Vec<String>
{
	let mut diff_files_by_lines: Vec<String> = Vec::with_capacity(64);
	let mut current_value: String = String::with_capacity(128);
	if diffed_files_from_standard_out.len() > 0
	{
		for character in diffed_files_from_standard_out.chars()
		{
			if character == '\n'
			{
				diff_files_by_lines.push(current_value.clone());
				current_value = String::with_capacity(128);
				continue;
			}

			current_value.push(character);
		}
	}

	return diff_files_by_lines;
}

fn common_metadata_buckets(tool_context: &mut ToolContext) -> Vec<MetadataBucket>
{
	let metadata_bucket_time_start = Instant::now();

	let metadata_buckets: Vec<MetadataBucket> = vec![
		MetadataBucket::new("approvalProcesses", "ApprovalProcess", false),
		MetadataBucket::new("aura", "AuraDefinitionBundle", true),
		MetadataBucket::new("businessProcesses", "BusinessProcess", false),
		MetadataBucket::new("classes", "ApexClass", false),
		MetadataBucket::new("compactLayouts", "CompactLayout", false),
		MetadataBucket::new("customMetadata", "CustomMetadata", false),
		MetadataBucket::new("customPermissions", "CustomPermission", false),
		MetadataBucket::new("customSettings", "CustomSetting", false),
		MetadataBucket::new("externalCredentials", "ExternalCredential", false),
		MetadataBucket::new("fieldSets", "FieldSet", false),
		MetadataBucket::new("fields", "CustomField", false),
		MetadataBucket::new("flexipages", "FlexiPage", false),
		MetadataBucket::new("flows", "Flow", false),
		MetadataBucket::new("globalValueSets", "GlobalValueSet", false),
		MetadataBucket::new("groups", "Group", false),
		MetadataBucket::new("labels", "CustomLabels", false),
		MetadataBucket::new("layouts", "Layout", false),
		MetadataBucket::new("listViews", "ListView", false),
		MetadataBucket::new("lwc", "LightningComponentBundle", true),
		MetadataBucket::new("namedCredentials", "NamedCredential", false),
		MetadataBucket::new("objects", "CustomObject", false),
		MetadataBucket::new("pages", "ApexPage", false),
		MetadataBucket::new("permissionsetgroups", "PermissionSetGroup", false),
		MetadataBucket::new("permissionsets", "PermissionSet", false),
		MetadataBucket::new("profiles", "Profile", false),
		MetadataBucket::new("quickActions", "QuickAction", false),
		MetadataBucket::new("recordTypes", "RecordType", false),
		MetadataBucket::new("remoteSiteSettings", "RemoteSiteSetting", false),
		MetadataBucket::new("searchLayouts", "SearchLayouts", false),
		MetadataBucket::new("standardValueSets", "StandardValueSet", false),
		MetadataBucket::new("tabs", "CustomTab", false),
		MetadataBucket::new("triggers", "ApexTrigger", false),
		MetadataBucket::new("validationRules", "ValidationRule", false),
		MetadataBucket::new("webLinks", "WebLink", false),
	];

	let metadata_bucket_time: f64 = metadata_bucket_time_start.elapsed().as_secs_f64() * 1000.0;
	let metadata_bucket_time_message: String = String::from(format!("manifest::metadata buckets initialization: {}ms\n", metadata_bucket_time));
	tool_context.time_snapshots.push(metadata_bucket_time_message);

	return metadata_buckets;
}

fn map_metadata_buckets(metadata_buckets: &Vec<MetadataBucket>) -> HashMap<String, usize>
{

	let mut bucket_folder_name_to_index: HashMap<String, usize> = HashMap::with_capacity(32);

	let mut bucket_index: usize = 0;
	for metadata_bucket in metadata_buckets
	{
		bucket_folder_name_to_index.insert(metadata_bucket.file_path_name.clone(), bucket_index);
		bucket_index += 1;
	}

	return bucket_folder_name_to_index;
}

fn change_code_constructive(change_code: &String) -> bool
{
	if change_code.starts_with('D') || change_code.starts_with('R')
	{
		return false;
	}

	return true;
}

// Most metadata categories are individual files within the standard folder name, and
// can be copied that way straight up, so this will be the most commonly used function
// for parsing the file path into its corresponding manifest text.
fn basic_name(change_code: &String, name_minus_root: &String, current_metadata_bucket: &mut MetadataBucket)
{
	let mut revised_name_stripped_of_file_extension: String = String::with_capacity(80);
	let mut reading: bool = false; // Doesn't matter until we hit first slash
	'revised_name: for name_char in name_minus_root.chars()
	{
		if name_char == '/' || name_char == '\\' { reading = true; continue 'revised_name; }

		if !reading { continue; }

		if name_char == '.' { break 'revised_name; }

		revised_name_stripped_of_file_extension.push(name_char);
	}

	if change_code_constructive(change_code)
	{
		current_metadata_bucket.files.insert(
			revised_name_stripped_of_file_extension
		);
	}
	else
	{
		current_metadata_bucket.destructive_files.insert(
			revised_name_stripped_of_file_extension
		);
	}
	
}

// The bundle consists of usually between 3 to 5 files or so inside of a folder,
// and the only thing we actually want for the package.xml manifest is the folder
// name, as that's all that's included - there's no specifying the individual HTML,
// .js or .css files included within the bundle.
fn bundle_name(name_minus_root: &String, current_metadata_bucket: &mut MetadataBucket)
{
	let mut revised_name: String = String::with_capacity(80);
	let mut found_first_slash = false;

	for character in name_minus_root.chars()
	{
		let is_a_slash: bool = character == '/' || character == '\\';

		if !found_first_slash && !is_a_slash { continue; }

		if is_a_slash && !found_first_slash { found_first_slash = true; continue; }

		if is_a_slash && found_first_slash { break; }

		if found_first_slash
		{
			revised_name.push(character);
		}
	}

	current_metadata_bucket.files.insert(revised_name);
}

fn quick_action_name(change_code: &String, name_minus_root: &String, current_metadata_bucket: &mut MetadataBucket)
{
	let mut revised_name: String = String::with_capacity(80);
	let mut found_first_slash = false;

	let mut current_position: usize = 0;

	let quick_action_extension = ".quickAction-meta.xml";
	let extension_length = quick_action_extension.len() - 1;

	for character in name_minus_root.chars()
	{
		current_position += 1;
		
		let is_a_slash = character == '/' || character == '\\';
		
		if !found_first_slash && !is_a_slash { continue; }

		if is_a_slash && !found_first_slash { found_first_slash = true; continue; }

		let number_remaining = name_minus_root.len() - current_position;

		if number_remaining == extension_length
		{
			if change_code_constructive(change_code)
			{
				current_metadata_bucket.files.insert(revised_name);
			}
			else
			{
				current_metadata_bucket.destructive_files.insert(revised_name);
			}
			
			break;
		}

		if found_first_slash
		{
			revised_name.push(character);
		}		
	}
}

fn object_metadata(change_code: &String,
	name_minus_root: &String,
	metadata_category_map: &HashMap<String, usize>,
	all_metadata_buckets: &mut Vec<MetadataBucket>)
{
	let mut object_name: String = String::with_capacity(80);
	let mut category_name: String = String::with_capacity(80);
	let mut file_name: String = String::with_capacity(80);

	let mut writing_object_name: bool = false;
	let mut writing_category_name: bool = false;
	let mut writing_file_name: bool = false;

	for character in name_minus_root.chars()
	{
		let is_a_slash = character == '/' || character == '\\';

		if is_a_slash && !writing_object_name && !writing_category_name && !writing_file_name
		{ writing_object_name = true; continue; }

		if is_a_slash && !writing_category_name
		{
			writing_object_name = false;
			writing_category_name = true;
			
			continue;
		}

		if is_a_slash && !writing_file_name
		{
			writing_category_name = false;
			writing_file_name = true;			
			continue;
		}

		// If hitting a . and not yet writing the filename, that means
		// that, actually, the category name is really the filename, and
		// this is describing the custom object itself.
		if character == '.' && !writing_file_name
		{
			let custom_object_bucket_index = metadata_category_map.get_key_value("objects").unwrap().1;
			let object_bucket = &mut all_metadata_buckets[*custom_object_bucket_index];

			if change_code_constructive(change_code)
			{
				object_bucket.files.insert(category_name.clone());
			}
			else
			{
				object_bucket.destructive_files.insert(category_name.clone());
			}
			break;
		}

		// If reaching the ., this is probably the file extension
		// for the .field filename, so bail out here, as this should not
		// make its way onto the final manifest.
		if character == '.' && writing_file_name
		{

			if !metadata_category_map.contains_key(&category_name)
			{
				// TODO: This should really be some kind of error, but not
				// sure how to handle it just yet, so just break for now,
				// but we probably need to use the logger to record this and
				// display an error in the terminal
				break;
			}

			let custom_field_bucket_index = metadata_category_map.get_key_value(&category_name).unwrap().1;
			let fields_bucket = &mut all_metadata_buckets[*custom_field_bucket_index];

			if change_code_constructive(change_code)
			{
				fields_bucket.files.insert(file_name);
			}
			else
			{
				fields_bucket.destructive_files.insert(file_name);
			}

			break;
		}

		if writing_object_name { object_name.push(character); }
		if writing_category_name { category_name.push(character); }
		if writing_file_name
		{
			// Fields are formatted as having the object API name,
			// followed by the field API name, such as the following
			// examples below:
			// Account.AnnualRevenue
			// Account.Primary_Contact__c
			// Opportunity.CES_Contract__c
			// App_Log__c.Message__c
			// and so on.
			if file_name.len() == 0
			{
				file_name.push_str(&object_name);
				file_name.push('.');
			}

			file_name.push(character);
		}
	}

}

fn custom_metadata_name(name_minus_root: &String, 
	current_metadata_bucket: &mut MetadataBucket)
{
	// Uses the length of the custom metadata file extension to know 
	// when to bail out of parsing the string. In this case, it is
	// the 11 characters from:
	// .md-meta.xml
	let custom_metadata_file_ext_len: usize = 12;

	let mut custom_metadata_name: String = String::with_capacity(80);
	let mut current_character_index: usize = 0;
	let mut past_first_slash: bool = false; // Skipping past the 'customMetadata/' filename prefix
	let length_of_prefix: usize = 15;
	for character in name_minus_root.chars()
	{
		if character == '/' || character == '\\' && !past_first_slash 
		{ past_first_slash = true; continue; }

		if !past_first_slash { continue; }

		custom_metadata_name.push(character);

		current_character_index += 1;
		if name_minus_root.len() - length_of_prefix - current_character_index == custom_metadata_file_ext_len { break; }
	}

	current_metadata_bucket.files.insert(custom_metadata_name);
}

fn sort_metadata_buckets(general_context: &mut Context,
	tool_context: &mut ToolContext,
	diffed_files_by_lines: &Vec<String>) -> ManifestBundle
{
	if diffed_files_by_lines.len() >= MAXIMUM_DIFF_FILE_SIZE
	{
		general_context.logger.log_error(
			&format!("ERROR: Number of files in diff exceeds the maximum file size of {}, exiting...\n", MAXIMUM_DIFF_FILE_SIZE)
		);

		return ManifestBundle::new();
	}
	
	// Each metadata bucket contains handling information for how the category
	// should be organized. The first step is to put all files into their respective
	// metadata buckets, with the .files property on each bucket indicating what should
	// make its way into the manifest (sort of, it gets complicated for custom objects, 
	// which have fields, or Lightning & Aura bundles, where we should take the folder 
	// name instead, and a few other exceptions). 
	let mut all_metadata_buckets = common_metadata_buckets(tool_context);
	general_context.logger.log_info(&format!("all_metadata_buckets.len(): {}\n", all_metadata_buckets.len()));
	let metadata_category_map = map_metadata_buckets(&all_metadata_buckets);

	let standard_folder = "force-app/main/default/";
	for line in diffed_files_by_lines
	{
		// This scan needs to take place in order to capture what the current change code is.
		// The change code in this definition is stuff like `M` for modified, `D` for deleted,
		// or R072 / R073 / R080 for renames. Renames are actually treated as both inserts and
		// deletes combined for these purposes.
		let mut change_code: String = String::with_capacity(8);
		let mut change_code_parsed: bool = false;

		let mut in_whitespace_after_change_code: bool = true;

		let mut line_file_path: String = String::with_capacity(80);
		let mut line_file_path_parsed: bool = false;
		
		let mut inside_file_extension: bool = false;
		
		let mut line_renamed_file_path: String =  String::with_capacity(80); // Usually not needed, except for renames

		for character in line.chars()
		{
			if character == '\n' || character == '\r' { break; }

			if character == '.'
			{
				inside_file_extension = true;
			}

			if (character == WHITESPACE || character == '\t') && !change_code_parsed
			{
				change_code_parsed = true;
				in_whitespace_after_change_code = true;
				continue;
			}

			if in_whitespace_after_change_code && (character == WHITESPACE || character == '\t')
			{
				continue;
			}
			else if in_whitespace_after_change_code && (character != WHITESPACE && character != '\t')
			{
				in_whitespace_after_change_code = false;
			}

			if inside_file_extension && (character == WHITESPACE || character == '\t')
			{
				line_file_path_parsed = true;
				continue;
			}

			if !change_code_parsed
			{ change_code.push(character); continue; }

			if !line_file_path_parsed
			{ line_file_path.push(character); continue; }

			if line_file_path_parsed && change_code.starts_with('R')
			{ line_renamed_file_path.push(character); continue; }
		}

		print!("change_code: {}, line_file_path: {}\n", change_code, line_file_path);

		// If the line does not start with force-app/main/default, this means it's packaged,
		// as there's a preceding directory to the force-app file structure. Unpackaged metadata
		// is the default and historically rampant.
		if line_file_path.starts_with("force-app")
		{
			let name_minus_root = line_file_path.replace(standard_folder, "");
			print!("{}\n", name_minus_root);

			// Parse the root phrase of the name_minus_root variable, 
			// as this determines which metadata bucket should be utilized.
			let mut root_metadata_category: String = String::with_capacity(80);

			let scan_mode_root_category: u8 = 0;
			let scan_mode_read_category: u8 = 1;
			let mut current_mode = scan_mode_root_category;

			// Initializing with the first bucket here just to have a non-null reference
			// This is changed once a supported metadata category is found because it will
			// drop that reference in this slot to add it into the bucket's 'files' Vec.
			for character in name_minus_root.chars()
			{
				let found_slash = character == '/' || character == '\\';

				// If reaching the first slash, this indicates that the mode
				// has changed from reading the root_metadata_category, to 
				// then dealing with what lay out on the rest of the file
				// path.
				if found_slash && current_mode == scan_mode_root_category
				{
					// Shift mode to handling a given category
					current_mode = scan_mode_read_category;

					// If handling a category, determine what bucket it corresponds to,
					// if any. If it doesn't, then we display an error that there is 
					// an unsupported metadata category
					let support_metadata_category = metadata_category_map.contains_key(&root_metadata_category);
					if support_metadata_category
					{
						let bucket_index = *metadata_category_map.get_key_value(&root_metadata_category).unwrap().1;
						let all_metadata_buckets_ref = &mut all_metadata_buckets;
						let current_metadata_bucket = &mut all_metadata_buckets_ref[bucket_index];

						if current_metadata_bucket.file_path_name == "objects"
						{
							object_metadata(&change_code, 
								&name_minus_root,
								&metadata_category_map, 
								all_metadata_buckets_ref);
						}
						else if current_metadata_bucket.file_path_name == "quickActions"
						{
							quick_action_name(&change_code, &name_minus_root, current_metadata_bucket);
						}
						else if current_metadata_bucket.file_path_name == "customMetadata"
						{
							custom_metadata_name(&name_minus_root, current_metadata_bucket);
						}
						else
						{
							if !current_metadata_bucket.bundle
							{ basic_name(&change_code, &name_minus_root, current_metadata_bucket); }

							if current_metadata_bucket.bundle
							{ bundle_name(&name_minus_root, current_metadata_bucket); }
						}						
						
						break;
					}
					else
					{
						general_context.logger.log_error(&format!("ERROR: Metadata category, {}, is not supported and has not been included in the manifest.\n", root_metadata_category));
					}

					continue;
				}

				if current_mode == scan_mode_root_category
				{ root_metadata_category.push(character); }
			}
		}
	}

	let mut xml_file_content: String = String::with_capacity(2048);
	xml_file_content.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
	xml_file_content.push_str("<Package xmlns=\"http://soap.sforce.com/2006/04/metadata\">\n");

	let mut destructive_xml_file_content: String = String::with_capacity(2048);
	destructive_xml_file_content.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
	destructive_xml_file_content.push_str("<Package xmlns=\"http://soap.sforce.com/2006/04/metadata\">\n");
	
	for bucket in all_metadata_buckets
	{
		if bucket.files.len() == 0 && bucket.destructive_files.len() == 0 { continue; }

		if bucket.files.len() > 0
		{ xml_file_content.push_str("\t<types>\n"); }

		if bucket.destructive_files.len() > 0
		{ destructive_xml_file_content.push_str("\t<types>\n"); }
		
		// From the files as they were added to the bucket in no
		// particular order, we'll transfer them to a Vec so that
		// we can use the .sort() functionality
		let mut sorted_files: Vec<String> = Vec::with_capacity(64);
		let mut sorted_destructive_files: Vec<String> = Vec::with_capacity(64);
		for file_name in &bucket.files
		{
			sorted_files.push(file_name.clone());
		}

		for file_name in &bucket.destructive_files
		{
			sorted_destructive_files.push(file_name.clone());
		}

		// Provides us alphabetical order from the string values
		// of the filenames that were added.
		sorted_files.sort();
		sorted_destructive_files.sort();

		for metadata_item_name in &sorted_files
		{
			xml_file_content.push_str("\t\t<members>");
			xml_file_content.push_str(&metadata_item_name);
			xml_file_content.push_str("</members>\n");
		}

		for metadata_item_name in &sorted_destructive_files
		{
			destructive_xml_file_content.push_str("\t\t<members>");
			destructive_xml_file_content.push_str(&metadata_item_name);
			destructive_xml_file_content.push_str("</members>\n");
		}

		if bucket.files.len() > 0
		{
			xml_file_content.push_str("\t\t<name>");
			xml_file_content.push_str(&bucket.package_xml_name);
			xml_file_content.push_str("</name>\n");
	
			xml_file_content.push_str("\t</types>\n");
		}

		// TODO: Should this be separated? Branched?
		if bucket.destructive_files.len() > 0
		{
			destructive_xml_file_content.push_str("\t\t<name>");
			destructive_xml_file_content.push_str(&bucket.package_xml_name);
			destructive_xml_file_content.push_str("</name>\n");

			destructive_xml_file_content.push_str("\t</types>\n");
		}
	}

	// Stupidly, if the category of the metadata is 'CustomLabel' then we
	// also have to add the CustomLabels category with a hardcoded 'CustomLabels'
	// member. Don't ask me, or this code comment, why. We don't know. No one 
	// understands why Salesforce would do it this way. -Scott Lee
	xml_file_content = xml_file_content.replace("<types>\n\t\t<members>CustomLabels</members>\n\t\t<name>CustomLabels</name>\n\t</types>\n",
				"<types>\n\t\t<members>*</members>\n\t\t<name>CustomLabels</name>\n\t</types>\n");

	xml_file_content.push_str("\t<version>64.0</version>\n");
	xml_file_content.push_str("</Package>");

	destructive_xml_file_content.push_str("\t<version>64.0</version>\n");
	destructive_xml_file_content.push_str("</Package>");

	return ManifestBundle{
		manifest: xml_file_content,
		destructive_manifest: destructive_xml_file_content
	};
}

fn latest_commit_has_error(latest_commit_compare: &String, latest_commit_feature: &String) -> bool
{
	return latest_commit_compare.len() == 0 
		|| latest_commit_feature.len() == 0
		|| latest_commit_compare.contains("HEAD")
		|| latest_commit_feature.contains("HEAD")
		|| latest_commit_compare.contains("not found")
		|| latest_commit_feature.contains("not found");
}

fn output_package_xml_file(_general_context: &mut Context, 
	tool_context: &mut ToolContext, 
	xml_content: &String,
	filename: &String)
{
	let xml_file_write_time_start = Instant::now();

	let string_only: bool = tool_context.command_parameters.contains_key("stringonly");

	if string_only
	{
		print!("xml:\n{}\n", xml_content);
		return;
	}

	let current_working_directory = tool_context.working_path.clone();
	let mut output_path: String = String::with_capacity(current_working_directory.len() + 80);
	output_path.push_str(&current_working_directory);
	output_path.push(slash());
	output_path.push_str(filename);

	file_system::write(output_path, xml_content.as_bytes()).unwrap();

	let xml_file_write_time: f64 = xml_file_write_time_start.elapsed().as_secs_f64() * 1000.0;
	let xml_file_write_time_message: String = String::from(format!("manifest::xml file write: {}ms\n", xml_file_write_time));
	tool_context.time_snapshots.push(xml_file_write_time_message);
}

fn clean_up(_general_context: &mut Context, tool_context: &mut ToolContext)
{
	let avoid_clean = tool_context.command_parameters.contains_key("noclean");

	if avoid_clean { return; }

	let clean_up_time_start = Instant::now();

	let current_working_directory = tool_context.working_path.clone();
	let mut temp_path_feature: String = String::with_capacity(current_working_directory.len() + 1 + FEATURE_BRANCH_TEMP_FOLDER.len());
	temp_path_feature.push_str(&current_working_directory);
	temp_path_feature.push(slash());
	temp_path_feature.push_str(FEATURE_BRANCH_TEMP_FOLDER);

	let mut temp_path_compare: String = String::with_capacity(current_working_directory.len() + 1 + COMPARE_BRANCH_TEMP_FOLDER.len());
	temp_path_compare.push_str(&current_working_directory);
	temp_path_compare.push(slash());
	temp_path_compare.push_str(COMPARE_BRANCH_TEMP_FOLDER);

	if file_system::metadata(&temp_path_feature).is_ok() {
		file_system::remove_dir_all(temp_path_feature).unwrap();
	}
	
	if file_system::metadata(&temp_path_compare).is_ok() {
		file_system::remove_dir_all(temp_path_compare).unwrap();
	}

	let clean_up_time: f64 = clean_up_time_start.elapsed().as_secs_f64() * 1000.0;
	let clean_up_time_message: String = String::from(format!("manifest::clean up: {}ms\n", clean_up_time));
	tool_context.time_snapshots.push(clean_up_time_message);
}

pub fn list_supported_metadata(tool_context: &mut ToolContext)
{
	let metadata_buckets = common_metadata_buckets(tool_context);

	print!("\n==SUPPORTED METADATA TYPES==\n");
	for bucket in &metadata_buckets
	{ print!("{}\n", bucket.package_xml_name); }
	print!("\n");
}

pub fn generate_manifest(general_context: &mut Context, 
	tool_context: &mut ToolContext)
{
	let (feature_branch, compare_branch) = branch_names(general_context, tool_context);

	// TODO: By using a different command argument, --name-status, we can also retrieve
	// the kind of change that was done within the diff, then differentiate between
	// destructive and non-destructive changes. So, the TODO: implement the use of 
	// git diff --name-status and generate both package.xml and destructiveChanges.xml.

	// By this point, we know the feature branch and compare branch. Now, we need to
	// orchestrate a diff with git. To determine this, we first need to know 2 things:
	// 1) The current commit of the feature branch provided from input
	// 2) The current commit of the compare branch, which is usually the 'qa' branch
	//
	// The two commits are fed into the git diff command, to appear something like this:
	// git diff --name-only SHA1 SHA2
	// To first determine the two commits, run the appropriate commands to find that.
	// We'll do this separate of where we are in the current folder structure by 
	// creating some folders and then running the appropriate commands to retrieve
	// those branches.
	// 
	// The rev-parse HEAD can provide the current commit ID to pass in to SHA1 and SHA2
	// above, simply using the following:
	// git rev-parse HEAD
	// This will return something like this:
	// 604ca1dc148f3c01e6e81982c5f37710b6895a60
	// This is the long form version of the commit ID within the git repository.
	let (repository_information, feature_branch_path, compare_branch_path) = initialize_repository_information(
		general_context, 
		tool_context, 
		&feature_branch, 
		&compare_branch
	);

	let mut diffed_files_by_lines: Vec<String> = Vec::new();

	if tool_context.command_parameters.contains_key("git") 
	{
		print!("Using Git orchestration methodology...\n");

		// Performs the work of creating repository folders and running necessary git commands
		// to pull in source details
		manage_branches(tool_context, &repository_information);

		let git_rev_parse_command = &String::from("git rev-parse HEAD");

		general_context.logger.log_info("For compare branch:\n");
		let (mut latest_commit_compare, _compare_error) = run_command(
			general_context, tool_context, &compare_branch_path, git_rev_parse_command);

		general_context.logger.log_info("For feature branch:\n");
		let (mut latest_commit_feature, _feature_error) = run_command(
			general_context, tool_context, &feature_branch_path, git_rev_parse_command);

		if latest_commit_has_error(&latest_commit_compare, &latest_commit_feature)
		{
			general_context.logger.log_error("ERROR: Retrieving latest commit failed. Exiting...\n");
			return;
		}

		// For some reason, standard out also includes new line characters and other unwanted 
		// things, so sanitize these before passing to the diff command.
		latest_commit_feature = latest_commit_feature.replace("\n", "").replace(" ", "");
		latest_commit_compare = latest_commit_compare.replace("\n", "").replace(" ", "");

		let git_diff_command = format!("git --no-pager diff --name-status {} {}", latest_commit_compare, latest_commit_feature);
		let (diffed_files_from_standard_out, diffed_files_error) = run_command(
			general_context, 
			tool_context, 
			&feature_branch_path, 
			&git_diff_command);

		diffed_files_by_lines = split_to_lines_vec(&diffed_files_from_standard_out);
	}
	else 
	{
		print!("Using Bitbucket REST API...\n");

		let bitbucket_username: &String = tool_context.configuration_variables.get("bitbucket_username").unwrap();
		let bitbucket_app_password: &String = tool_context.configuration_variables.get("bitbucket_app_password").unwrap();
		let bitbucket_workspace: &String = tool_context.configuration_variables.get("bitbucket_workspace").unwrap();
		let bitbucket_repository: &String = tool_context.configuration_variables.get("bitbucket_repository").unwrap();

		let bitbucket: Bitbucket = Bitbucket::new(bitbucket_username.to_string(), bitbucket_app_password.to_string(), bitbucket_workspace.to_string(), bitbucket_repository.to_string()); 
		let tokio_runtime: tokio::runtime::Runtime = tokio::runtime::Runtime::new().unwrap();
		diffed_files_by_lines = tokio_runtime.block_on(bitbucket.get_diff(&feature_branch, &compare_branch)).unwrap();
	}

	let parse_time_start: Instant = Instant::now();
	let manifest_bundle: &ManifestBundle = &sort_metadata_buckets(general_context, tool_context, &diffed_files_by_lines);

	let parsing_time: f64 = parse_time_start.elapsed().as_secs_f64() * 1000.0;
	let parsing_time_message: String = String::from(format!("manifest::parsing: {}ms\n", parsing_time));
	tool_context.time_snapshots.push(parsing_time_message);

	let package_xml_name: String = String::from("package.xml");
	let destructive_xml_name: String = String::from("destructiveChanges.xml");

	output_package_xml_file(general_context, tool_context, &manifest_bundle.manifest, &package_xml_name);
	output_package_xml_file(general_context, tool_context, &manifest_bundle.destructive_manifest, &destructive_xml_name);

	clean_up(general_context, tool_context);
}
