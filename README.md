# sfmanifest

This tool automatically generates Salesforce deployment manifest files using the Bitbucket API (by default) or Git orchestration (which works with any service or server using git).

sfmanifest will compare the latest commits on branch names passed into the command, run a diff operation, and then parse the filename changes into a metadata XML file.

If running within a project directory that contains a Git repository and is on a given branch, the tool will use that given branch by default. 

```
sfmanifest --branch dev
```


Otherwise, branch arguments must be specified, such as the following:

```
sfmanifest --feature feature/my-feature-branch --branch dev
```

This tool also makes an assumption that, absent the `--branch` specification, it will assume that your comparison branch argument is `qa`. This is due to our original internal use of this tool, which always compares to a shared "QA" branch.

As a result, if you also have a process whereby developers are always comparing to the `qa` branch AND you'd like to use the current branch within the working directory, you can simply run:

```
sfmanifest
```

We may make the comparison branch default a configurable variable in the future but for now it is locked to `qa` as a default literal.


## Installation

We do not have installers or pre-compiled executables to provide for a number of reasons, however compiling from source can be done using a regular `cargo build --release`.

If using Windows, you'll need to update your environment PATH to point to your executable directory. Note that the program will automatically create a `config.txt` within its running directory upon running for the first time, so any necessary permissions to write files will be necessary.

The same is generally true for installation on Linux, which will likely require updating your `.bashrc` or other relevant pathing to enable the `sfmanifest` command to work in the terminal.

At this time, we do not support compilation on `MacOS` but due to the open nature of this project others are free to do their own MacOS target compilations - we've not examined the codebase for any particular details that may be relevant to that process.


## Support / FAQ

As this is an open-source, publicly facing tool that originally came from within the Symmetry Energy organization, it is important to note that we will not be able to provide active levels of daily support for sfmanifest or be able to accept all pull requests opened from public contributors. Note that any pull requests that add support for more categories of metadata than what the current version supports: we will only approve those that fit existing conventions and can be adequately tested. 

### Who maintains this?
This tool was originally built internally for our own use within Symmetry Energy Solutions, LLC and at the time of this writing is an active part of our toolset for Salesforce DevOps management. However, we can make no guarantees toward future maintenance and this is not a 'sponsored' open-source project. Therefore, we may end up dropping updates for this project at any time. It is free to be forked and worked on by others under the included license.

### Am I able to use sfmanifest for my own commercial project?
This project is created under the associated `MIT License`, which you're free to read included here in the repository. In general, this license allows use of this code for commercial projects but protects Symmetry Energy Solutions, LLC from liability and is providing this code "as-is."