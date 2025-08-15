pub use structopt::StructOpt;
use std::env::args;
use std::fmt;
use std::str::FromStr;

#[derive(Debug)]
pub struct ParseModeError;

impl fmt::Display for ParseModeError
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result
    {
        write!(formatter, "Invalid automation mode")
    }
}

#[derive(Debug, StructOpt, PartialEq)]
pub enum Automation
{
    Bitbucket,
    Git
}

impl fmt::Display for Automation
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result
    {
        write!(formatter, "{:?}", self)
    }
}

impl FromStr for Automation
{
    type Err = ParseModeError;

    fn from_str(string_value: &str) -> Result<Self, Self::Err>
    {
        match string_value.to_lowercase().as_str()
        {
            "bitbucket" => Ok(Automation::Bitbucket),
            "b" => Ok(Automation::Bitbucket),
            "git" => Ok(Automation::Git),
            "g" => Ok(Automation::Git),
            _ => Err(ParseModeError)
        }
    }
}

impl Default for Automation
{
    fn default() -> Self {
        Automation::Bitbucket
    }
}

#[derive(Debug, StructOpt)]
#[structopt(name = "sfmanifest", 
    about = "Manifest generation tool using git diff automation.\n\nCopyright 2025 Symmetry Energy Solutions, LLC\nAvailable for use under the associated MIT License. \nSee the `LICENSE` file included with the source repository.")]
pub struct Opt 
{
    /// The feature branch to compare to the comparison branch, which should normally
    /// be upstream of shared environment branches
    #[structopt(short = "f", long = "feature")]
    pub feature: Option<String>,

    /// Comparison branch, or whatever target branch the feature branch is being merged into.
    #[structopt(short = "b", long = "branch", default_value = "qa")]
    pub branch: String,

    /// If enabled, will avoid producing package.xml and destructiveChanges.xml and instead 
    /// only print the string contents of the package.xml manifest to the terminal.
    #[structopt(short = "s", long = "string-only")]
    pub string_only: bool,

    /// Bitbucket username to use for Git orchestration, if using Bitbucket. 
    #[structopt(short = "u", long = "bitbucket-user")]
    pub bitbucket_user: Option<String>,

    /// Avoids removing temporary folders if using Git orchestration mode. When using 
    /// API services, this does not apply (and setting it would do nothing).
    #[structopt(short = "n", long = "noclean")]
    pub no_clean: bool,

    /// Avoids running manifest generation and instead lists all supported metadata 
    /// categories that will parse and result in the included manifest.
    #[structopt(short = "p", long = "supported")]
    pub list_supported_mode: bool,

    /// Set the automation mode for how the manifest will be generated, which defaults
    /// to "bitbucket" but would otherwise be "git" for generic Git orchestration.
    #[structopt(short = "a", long = "automation", default_value="bitbucket")]
    pub automation: Automation,

    /// Set configuration variable, which will be a key/value pair maintained in the
    /// executable folder's path in a file called "config.txt"
    #[structopt(short = "e", long = "config-set")]
    pub config_set: Option<String>,

    /// Get all configuration values within config.txt, the configuration variable
    /// file held in the executable's same folder.
    #[structopt(short ="x", long ="config-get-all")]
    pub config_get_all: bool,
}

impl Opt
{
    pub fn new() -> Self
    {
        Opt::from_args()
    }
}

