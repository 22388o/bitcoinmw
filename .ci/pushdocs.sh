#!/bin/bash

echo "Reason=$3";
if [ "$3" == "Schedule" ] || [ "$3" == "Manual" ]; then
  cd ~
  git clone https://anything:$1@github.com/cgilliard/bitcoinmw.git bmw_new2
  cd bmw_new2
  git config --global user.name "Pipelines-Bot"
  git config --global user.email "noreply@nodomain.com"
  cargo doc --no-deps --workspace

  rm -rf docs/doc/*
  cp -pr target/doc/* docs/doc/

  git fetch
  if [ `git diff --exit-code origin/main..main | wc -l | xargs` = "0" ]; then
    git pull
    git add --all
    git commit -m "Pipelines-Bot: Updated repo (via pushdocs script) Source Version is $2";
    git push https://$1@github.com/cgilliard/bitcoinmw.git
  else
    echo "There are changes after this checkout. Not committing!"
    git diff origin/main
    git diff origin/main | wc -l
  fi
fi
