#!/bin/bash

echo $1
if [ "$1" == "Schedule" ] || [ "$1" == "Manual" ]; then
	echo "Running a scheduled/manual CI";
	rustup update
	cargo test --all
elif [ `git show --summary | grep "^Author: Pipelines-Bot" | wc -l | xargs` = "0" ]; then
	echo "non Pipelines-Bot checkin";
	rustup update
	cargo test --all
else
	echo "This is a Pipelines-Bot checkin. Will not execute."
fi
