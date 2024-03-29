#!/bin/bash

if [ "$1" == "Schedule" ]; then
  sudo apt-get update -yqq
  sudo apt-get install -yqq --no-install-recommends libncursesw5-dev libssl-dev
  cargo install cargo-tarpaulin

  for i in {0..300}
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
