#!/bin/bash

LOOPS=$(($1))

for (( i=0; i<$LOOPS; i++ ))
do
	cargo test -p bmw_evh2 --lib
done
