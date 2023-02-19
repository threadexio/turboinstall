#!/bin/bash

# $1 - path to dir
function dir_tree_contents {
	(cd -- "$1" && find . -printf '/%P\n' | sort)
}

src="$TEST_DIR/profile-tree"

# $1 - profile type
function run_profile_test {
	local ptype="${1:?}"

	local dst="$PWD/profile-tree-${ptype}"
	mkdir "$dst"

	turboinstall \
		-p "$src/.turboinstall/profile.${ptype}" \
		-- "$dst" "$src"

	mapfile -t dst_paths < <(dir_tree_contents "$dst")

	assert_eq "${dst_paths[*]}" \
		"/ /usr /usr/local /usr/local/file_VALUE_1 /VALUE_1"
}

function test_json_profile {
	run_profile_test "json"
}

function test_toml_profile {
	run_profile_test "toml"
}

function test_yaml_profile {
	run_profile_test "yaml"
}

function test_env_profile {
	run_profile_test "env"
}
