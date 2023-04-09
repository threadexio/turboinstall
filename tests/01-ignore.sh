#!/bin/bash

# $1 - path to dir
function dir_tree_contents {
	(cd -- "$1" && find . -printf '/%P\n' | sort -d)
}

src="$TEST_DIR/ignore-tree"

function test_basic_ignore {
	local dst="$PWD/ignore-tree"

	mkdir "$dst"

	turboinstall -- "$dst" "$src"

	mapfile -t dst_paths < <(dir_tree_contents "$dst")

	assert_eq "${dst_paths[*]}" \
		"/ /dir1 /dir1/dir2 /dir1/dir2/file2 /dir1/file1"
}

function test_multiple_ignore {
	local dst="$PWD/ignore-tree-multiple"

	mkdir -p "$dst"

	echo $dst

	turboinstall \
		--ignore-file ".turboinstall/ignore_1" \
		-- "$dst" "$src"

	mapfile -t dst_paths < <(dir_tree_contents "$dst")

	assert_eq "${dst_paths[*]}" \
		"/ /dir1 /dir1/dir2"
}
