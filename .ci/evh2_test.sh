#!/bin/bash

LOOPS=$(($1))

for (( i=0; i<$LOOPS; i++ ))
do
	echo "test loop $i";
	cargo test -p bmw_evh2 --lib
	if [ $? != 0 ]; then
		break;
	fi

done
