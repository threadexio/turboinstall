#!/bin/bash

src="$TEST_DIR/hook-tree"

function test_basic_hooks {
	local dst="$PWD/hook-tree"

	mkdir "$dst"

	turboinstall -- "$dst" "$src"

	assert \
		[ -f "$dst/pre-install-ok" ] && \
		[ -f "$dst/post-install-ok" ]
}

function test_no_hooks {
	local dst="$PWD/hook-tree-none"

	mkdir "$dst"

	turboinstall --no-hooks -- "$dst" "$src"

	assert \
		[ ! -f "$dst/pre-install-ok" ] && \
		[ ! -f "$dst/post-install-ok" ]
}

function test_preinstall_only {
	local dst="$PWD/hook-tree-pre"

	mkdir "$dst"

	turboinstall \
		--hooks pre-install \
		-- "$dst" "$src"

	assert \
		[ -f "$dst/pre-install-ok" ] && \
		[ ! -f "$dst/post-install-ok" ]
}
