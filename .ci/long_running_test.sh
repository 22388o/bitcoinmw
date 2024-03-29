#!/bin/bash

if [ "$1" == "Schedule" ]; then
  cargo install cargo-tarpaulin
  LOOPS=$(($2 + 0))
  echo $LOOPS

  for (( i=0; i<$LOOPS; i++ ))
  do
    echo "Running tests: $i `date`"
    cargo tarpaulin -p bmw_evh2 --skip-clean
    if [ $? != 0 ]; then
      break;
    fi
  done
else 
  echo "not running long running tests because this is not the nightly build"
fi
