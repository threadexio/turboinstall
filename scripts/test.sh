#!/bin/bash
set -eu
shopt -s nullglob

function basename {
	printf "%s" "${1##*/}"
}

function dirname {
	printf "%s" "${1%/*}"
}

REPO_DIR=$(cd -- "$(dirname "$0")/.." && pwd -P)
TEST_DIR="${REPO_DIR}/tests"

function colored {
	local style=""

	while [ $# -gt 1 ]; do
		case "$1" in
			black)   style+="\x1b[30m"; shift ;;
			red)     style+="\x1b[31m"; shift ;;
			green)   style+="\x1b[32m"; shift ;;
			yellow)  style+="\x1b[33m"; shift ;;
			blue)    style+="\x1b[34m"; shift ;;
			magenta) style+="\x1b[35m"; shift ;;
			cyan)    style+="\x1b[36m"; shift ;;
			white)   style+="\x1b[37m"; shift ;;

			bold)         style+="\x1b[1m"; shift ;;
			underline|ud) style+="\x1b[4m"; shift ;;

			*) shift ;;
		esac
	done

	printf "%b%s%b" "$style" "$1" "\x1b[0m"
}

_LINE_RESET="\x1b[2K\x1b[0G"

function info {
	printf "%b   %b %s\n" "$_LINE_RESET" "$(colored green bold INFO)" "$@"
}

function warn {
	printf "%b   %b %s\n" "$_LINE_RESET" "$(colored yellow bold WARN)" "$@"
}

function error {
	printf "%b  %b %s\n" "$_LINE_RESET" "$(colored red bold ERROR)" "$@"
}

function fatal {
	error "$@"
	exit 1
}

function assert {
	if ! "$@"; then
		error "Assertion failed: '$*'"
		exit 1
	fi
}

function assert_eq {
	for arg in "${@:2}"; do
		if ! [ "$1" == "$arg" ]; then
			error "Assertion failed."
			error "lhs: '$1'"
			error "rhs: '$arg'"

			exit 1
		fi
	done
}

TEST_TMP_DIR="$(mktemp -d -p "${REPO_DIR}/target")"

cd -- "$TEST_TMP_DIR" || fatal "Unable to switch to temp directory"

for test_file in "$TEST_DIR"/*.sh; do
	TEST_FILE="$(basename "$test_file")"

	function _test_runner {
		function turboinstall {
			(cd -- "$REPO_DIR" && cargo run -q -- "$@")
		}

		mapfile -t tests < <(declare -F | awk '{print $NF}' | grep -E '^test_')

		local test_path
		for test_fn in "${tests[@]}"; do
			test_path="$(colored cyan "$TEST_FILE")::$(colored cyan "$test_fn")"

			printf '\n'
			info "running ${test_path} ..."
			if ! (set -e && "$test_fn"); then
				fatal "test ${test_path}    $(colored bold red ud FAILED)"
			else
				info  "test ${test_path}    $(colored bold green ud OK)"
			fi
		done
	}

	# shellcheck source=/dev/null
	(set -e && . "$test_file" && _test_runner)

	unset TEST_FILE
done
