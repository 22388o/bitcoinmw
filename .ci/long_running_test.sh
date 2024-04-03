#!/bin/bash

start=`date +%s`

if [ "$1" == "Schedule" ] || [ "$1" == "Manual" ]; then
  cargo install cargo-tarpaulin
  LOOPS=10000;
  echo $LOOPS

  for (( i=0; i<$LOOPS; i++ ))
  do
    now=`date +%s`
    diff=$(($now - $start))
    echo "diff=$diff"
    echo "Running tests: $i `date`"
    if [ $diff -gt $2 ]; then
      echo "breaking diff=$diff,limit=$2";
      break;
    fi
    cargo tarpaulin -p bmw_evh --skip-clean
    if [ $? != 0 ]; then
      break;
    fi
  done
else 
  echo "not running long running tests because this is not the nightly build"
fi
