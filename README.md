# turboinstall

A quick and simple tool that overlays directory trees.

## Table of contents

* [turboinstall](#turboinstall)
	* [Table of contents](#table-of-contents)
	* [What does this mean?](#what-does-this-mean)
	* [Who even needs this?](#who-even-needs-this)
	* [Features](#features)
	* [Installation](#installation)
	* [Usage](#usage)
		* [What is the profile?](#what-is-the-profile)
			* [Path expansion](#path-expansion)
				* [Example profiles](#example-profiles)
		* [Using hooks](#using-hooks)
			* [Hook environment](#hook-environment)
			* [Pre-install](#pre-install)
			* [Post-install](#post-install)

## What does this mean?

It means you can effortlessly and easily install files to the right places without writing any custom install scripts. Just replicate the structure you need inside your source tree and everything else will be handled by the tool.

## Who even needs this?

Ever needed to create some sort of directory layering for packaging applications? In reality this tool was made to serve a very specific need: the runtime system for my  [zeus](https://github.com/threadexio/zeus) project and more specifically how the packaging for those works.

I wrote a similar tool for this job in bash but it had some problems:

1. It was quick and dirty
2. It does not run reliably on systems with other versions of coreutils
3. It does not run under native Windows (dear god why would you even want to do that?)
4. I wanted something more official and well-made

So here I am, coding a simple tool for a very specific purpose. If you find this tool neat consider giving it a star ⭐

If you do decide to try out this tool, please be aware that there probably are many bugs (especially in path traversal), use it with care.

## Features

* [x] Overlay multiple sources trees on top of each other
* [x] In-path variable expansion (basically path substitution)
* [x] 4 different profile formats (json, toml, yaml, env)
* [x] Hooks for custom actions
* [x] Pretty colors 🌈✨
* [ ] Ability to define regex rules to ignore paths (like .gitignore)
* [ ] Shell completions

## Installation

If you are the kind of person who needs this, then there is a high chance that you have `rust` and `cargo` installed. In that case:

```bash
cargo install turboinstall
```

## Usage

<details>
<summary>Command line arguments</summary>

```bash
Usage: turboinstall [OPTIONS] <dir> [dir]...

Arguments:
  <dir>     Destination directory
  [dir]...  Overlay source(s)

Options:
  -p, --profile </path/to/profile>  Path to the file with the profile definition [default: .turboinstall.json]
  -f, --format <fmt>                Specify which format the profile uses [possible values: json, toml, yaml, env]
  -l, --link                        Hard link files instead of copying
  -n, --no-clobber                  Do not overwrite existing files
  -u, --update                      Overwrite only when the source path is newer
      --dry-run                     Do not perform any filesystem operations (implies --no-hooks)
      --no-hooks                    Do not run any hooks
      --hooks <type,type,...>       Only run these types of hooks [possible values: pre-install, post-install]
  -h, --help                        Print help information
```

</details>

In short this tool does 1 thing. It takes a directory tree (`src`) like this:

```none
 src/
├──  dir1/
│   ├──  dir2/
│   │   └──  file2
│   └──  file1
└──  file0
```

And it copies it to another directory (`dst`) like this:

```none
 dst/
├──  dir1/
│   ├──  dir2/
│   │   └──  file2
│   └──  file1
└──  file0
```

The command to achieve this is:

```bash
turboinstall ./dst ./src
```

### What is the profile?

The profile is a fancy way of saying `configuration file` or `variable store`. It is a file in one of the supported formats (see [Features](#features)) that holds the variables for the path expansion.

> NOTE: Path expansion is not fully completed but it is functional

Profiles are only used for path expansion and nothing else, they are not needed if don't plan to use this feature at all.

#### Path expansion

The following examples act in the same way, they are just expressed in different formats. This makes the tool easy to integrate with other custom tooling. The env format is especially useful because you can `source` it directly from shell scripts.

You can specify a custom profile with `-p`.

The following profiles can be used to do path expansion. So instead of the [previous example](#usage), it would turn this:

```none
 src/
├──  file
└──  {DIR}/
    ├──  test/
    │   └──  file
    └──  file_{VARIABLE_1}
```

Into this:

```none
 dst/
├──  file
└──  usr/
    └──  local/
        ├──  test/
        │   └──  file
        └──  file_VALUE_1
```

This way you can create different outputs from one easily understood source tree by just specifying another profile on the command like. For example, this could allow you to build packages for, let's say, different systems with different filesystem hierarchies from one source tree and a bunch of configuration files.

No more pesky install scripts.

The command to do this is:

```bash
turboinstall ./dst ./src -p example_profile.json
```

Where `example_profile.json` is the file with one of the example profiles bellow. It does not need to be named like that, just a normal file with the corresponding extension, otherwise you will need to specify the format with `-f`.

##### Example profiles

**JSON:**

```json
{
  "VARIABLE_1": "VALUE_1",
  "DIR": "/usr/local"
}
```

**TOML:**

```toml
VARIABLE_1 = "VALUE_1"
DIR = "/usr/local"
```

**YAML:**

```yaml
VARIABLE_1: "VALUE_1"
DIR: "/usr/local"
```

**ENV:**

```bash
# This is a comment
# Also, the quotes are not needed
VARIABLE_1='VALUE_1'
DIR="/usr/local"
```

### Using hooks

Hooks are just executables placed in a special location that are executed in wildcard order (alphanumerical) with 2 arguments:

1. The source tree
2. The destination tree

The special location for these files is inside the root of the source tree in a folder called `.turboinstall`, in other words this is how your source tree should look like:

```none
 src/
├──  .turboinstall/
│   ├──  post-install/
│   │   └──  some_hook.sh
│   └──  pre-install/
│       ├──  00-hook.sh
│       └──  10-another_hook.sh
```

The hooks are executed in the following order:

**pre-install hooks:**

1. `00-hook.sh`
2. `10-another_hook.sh`

**post-install hooks:**

1. `some_hook.sh`

It is not strictly necessary to follow the naming convention shown here in the pre-install hooks, but it gives a clear indication of the order the hooks are executed in.

#### Hook environment

The hooks are invoked with 2 arguments:

1. The path of the source tree they reside in
2. The path of the destination tree

Their working directory is left untouched and is the same as the working directory where `turboinstall` was ran. This allows the hooks to access any other files that might be relevant and are not present in the source tree.

#### Pre-install

The executables insider `.turboinstall/pre-install`, like the name suggests are run _before_ any of the actual source tree has been copied.

#### Post-install

The executables insider `.turboinstall/pre-install`, like the name suggests are run _after_ any of the source tree has been copied.
