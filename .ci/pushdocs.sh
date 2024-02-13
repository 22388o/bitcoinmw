#!/bin/bash

if [ `git show --summary | grep "^Author: Pipelines-Bot" | wc -l | xargs` = "0" ]; then
  cd ~
  git clone https://anything:$1@github.com/cgilliard/bitcoinmw.git bmw_new2
  cd bmw_new2
  changes_error=`git diff HEAD^^ HEAD --name-only | grep "^error\/src" | wc -l`
  changes_log=`git diff HEAD^^ HEAD --name-only | grep "^log\/src" | wc -l`
  changes_util=`git diff HEAD^^ HEAD --name-only | grep "^util\/src" | wc -l`
  changes_derive=`git diff HEAD^^ HEAD --name-only | grep "^derive\/src" | wc -l`
  changes_ser=`git diff HEAD^^ HEAD --name-only | grep "^ser\/src" | wc -l`
  changes_evh=`git diff HEAD^^ HEAD --name-only | grep "^evh\/src" | wc -l`
  changes_http=`git diff HEAD^^ HEAD --name-only | grep "^http\/src" | wc -l`

  if [[ $changes_error -eq 0 ]] &&
     [[ $changes_log -eq 0 ]] &&
     [[ $changes_util -eq 0 ]] &&
     [[ $changes_derive -eq 0 ]] &&
     [[ $changes_ser -eq 0 ]] &&
     [[ $changes_evh -eq 0 ]] &&
     [[ $changes_http -eq 0 ]]
  then
    echo "no changes to relevant directories, not pushing"
  else
    changes=`git diff HEAD^^ HEAD --name-only`
    echo "updating with changes = $changes"
    git config --global user.name "Pipelines-Bot"
    git config --global user.email "noreply@nodomain.com"
    git checkout main
    cargo doc --no-deps --workspace

    cp -pr target/doc/* docs/doc/

    git pull
    git add --all
    git commit -m "Pipelines-Bot: Updated repo (via pushdocs script) Source Version is $2";
    git push https://$1@github.com/cgilliard/bitcoinmw.git
  fi
else
  echo "This is a Pipelines-Bot checkin. Will not execute."
fi
