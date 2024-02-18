#!/bin/bash

if [ "$1" == "Schedule" ]; then
	echo "Running a scheduled CI";
	cd etc/evh_perf
	cargo build --release
	./target/release/evh_perf -e -c -i 100 --count 10 --clients 2 -t 10
elif [ `git show --summary | grep "^Author: Pipelines-Bot" | wc -l | xargs` = "0" ]; then
	cd etc/evh_perf
	cargo build --release
	./target/release/evh_perf -e -c -i 100 --count 10 --clients 2 -t 10
else
	echo "This is a Pipelines-Bot checkin. Will not execute."
fi
