#!/bin/bash

if [ `git show --summary | grep "^Author: Pipelines-Bot" | wc -l | xargs` = "0" ]; then
	rustup update
	cargo test --all
else
	echo "This is a Pipelines-Bot checkin. Will not execute."
fi
