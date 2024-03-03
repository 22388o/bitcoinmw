#!/bin/bash

echo "Reason=$3";
if [ `git show --summary | grep "^Author: Pipelines-Bot" | wc -l | xargs` = "0" ] || [ "$3" == "Schedule" ] || [ "$3" == "Manual" ]; then
  last_tarpaulin_summary=$( tail -n 1 docs/tarpaulin_summary.txt)
  last_tarpaulin_summary_split=( $last_tarpaulin_summary )
  # only update at most once per hour
  limit_l=`expr ${last_tarpaulin_summary_split[0]} + 3600`
  timestamp=$(date +%s)
  echo "limit=$limit_l,timestamp=$timestamp"
  if [ $limit_l -le $timestamp ]
  then
    echo "updating"
      sudo apt-get update -yqq
      sudo apt-get install -yqq --no-install-recommends libncursesw5-dev tor libssl-dev
      cargo install cargo-tarpaulin
      cargo tarpaulin > /tmp/tarpaulin.out
      echo "cat /tmp/tarpaulin.out"
      cat /tmp/tarpaulin.out
      cd ~
      git clone https://anything:$1@github.com/cgilliard/bitcoinmw.git bmw_new
      cd bmw_new
      git config user.name "Pipelines-Bot"
      git checkout main
      last=$( tail -n 1 /tmp/tarpaulin.out )
      spl=( $last )
      str=${spl[0]}
      IFS='%';
      read -rasplitIFS<<< "$str"
      cur=${splitIFS[0]}
      re='^[0-9]+([.][0-9]+)?$'
      if ! [[ $cur =~ $re ]] ; then
        echo "error: Not a number" >&2; exit 1
      else
        echo "number ok $cur"
        IFS=' ';
        echo "$timestamp ${splitIFS[0]}" >> docs/tarpaulin_summary.txt
        cp README.md.template README.md
        export ccvalue=${splitIFS[0]}
        perl -pi -e 's/CODECOVERAGE/$ENV{ccvalue}/g' README.md
        chmod 755 ./.ci/make_cc_graph.sh
        ./.ci/make_cc_graph.sh

        git config --global user.email "pipelinesbot.noreply@example.com"
        git config --global user.name "Pipelines-Bot"

	if [`git diff origin/main | wc -l | xargs` = "0" ]; then
          git pull
          git add --all
          git commit -m"Pipelines-Bot: Updated repo (via tarpaulin script) Source Version is $2";
          git push https://$1@github.com/cgilliard/bitcoinmw.git
        else
	  echo "There are changes after this checkout. Not committing!"
	fi
      fi
    else
      echo "Not updating too recent."
    fi
  else
    echo "This is a Pipelines-Bot checkin. Will not execute."
  fi
