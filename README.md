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
		* [The ignore file](#the-ignore-file)
		* [Profiles and path expansion](#profiles-and-path-expansion)
			* [Example profiles](#example-profiles)
				* [JSON](#json)
				* [TOML](#toml)
				* [YAML](#yaml)
				* [ENV](#env)
		* [Hooks](#hooks)
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
* [x] Ability to define regex rules to ignore paths (like .gitignore)
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
A simple tool for overlaying directory trees on top of each other

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
      --ignore </path/to/file>      Path to ignore file [default: .turboinstall/ignore]
      --dry-run                     Do not perform any filesystem operations (implies --no-hooks)
      --no-hooks                    Do not run any hooks
      --hooks <type,type,...>       Only run these types of hooks [possible values: pre-install, post-install]
  -h, --help                        Print help information
  -V, --version                     Print version information

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

### The ignore file

The ignore file is a simple text file at `.turboinstall/ignore` that contains everyone's favorite regular expressions 🎉. Each line of the file contains a regex pattern that will be matched on each path of the overlay. In other words, just like `.gitignore` files.

Let's suppose we have a source tree:

```none
 src/
├──  .turboinstall/
│   └──  ignore
├──  dir0/
│   ├──  dir1/
│   │   ├──  file1
│   │   └──  file2
│   └──  file0
└──  file0
```

and `src/.turboinstall/ignore` contains:

```bash
# This is a comment
# Empty lines are also ignored

/file0
```

This would mean that when we run `turboinstall ./dst ./src` we would get:

```none
 dst/
└──  dir0/
    └──  dir1/
        ├──  file1
        └──  file2
```

Notice how both `src/file0` and `src/dir0/file0` are missing. This is because unlike gitignore files, these files match with pure regex on paths. The pattern `/file0` from the ignore file matches both:

* `/file0`
* `/dir0/file0`

Ok, so how can we **only** match `/file0`? This is very simple as long as you know basic regex. Just prepend the pattern with `^`, which means: only match the following if it is at the start of the path, so our ignore file becomes:

```bash
# This is a comment
# Empty lines are also ignored

^/file0
```

In this example the paths that will be tested are:

* `/dir0`
* `/dir0/dir1`
* `/dir0/dir1/file1`
* `/dir0/dir1/file2`
* `/dir0/file0`
* `/file0`

> NOTE: Anything inside the `/.turboinstall` folder is always automatically ignored, there is no way to change this.

### Profiles and path expansion

The profile is a fancy way of saying `configuration file` or `variable store`. It is a file in one of the supported formats (see [Features](#features)) that holds the variables for the path expansion.

> NOTE: Path expansion is not fully completed but it is functional

Profiles are only used for path expansion and nothing else, they are not needed if don't plan to use this feature at all.

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

#### Example profiles

##### JSON

```json
{
  "VARIABLE_1": "VALUE_1",
  "DIR": "/usr/local"
}
```

##### TOML

```toml
VARIABLE_1 = "VALUE_1"
DIR = "/usr/local"
```

##### YAML

```yaml
VARIABLE_1: "VALUE_1"
DIR: "/usr/local"
```

##### ENV

```bash
# This is a comment
# Also, the quotes are not needed
VARIABLE_1='VALUE_1'
DIR="/usr/local"
```

### Hooks

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

The executables inside `.turboinstall/pre-install`, like the name suggests are ran _before_ any of the actual source tree has been copied.

#### Post-install

The executables inside `.turboinstall/post-install`, like the name suggests are ran _after_ the source tree has been copied.
