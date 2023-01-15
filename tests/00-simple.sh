#!/bin/bash

# $1 - path to dir
function dir_tree_hash {
	(cd -- "$1" && find .) | sort | sha1sum | cut -d' ' -f1
}

function test_simple_tree {
	local dst="$PWD/simple-tree"
	local src="$TEST_DIR/simple-tree"

	mkdir "$dst"

	turboinstall -- "$dst" "$src"

	local src_hash="$(dir_tree_hash "$src")"
	local dst_hash="$(dir_tree_hash "$dst")"

	assert_eq "$src_hash" "$dst_hash"
}
