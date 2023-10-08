#!/bin/bash

cp docs/code_coverage.html.template docs/code_coverage.html;
# copy tarpaulin output into template
export tarpaulin_output=`cat /tmp/tarpaulin.out`;
perl -pi -e 's/REPLACETARPAULINOUTPUT/$ENV{tarpaulin_output}/g' docs/code_coverage.html

# read in data from summary
entries=`cat docs/tarpaulin_summary.txt`;
declare -a timestamps;
declare -a values;
i=0;
rm -f /tmp/timestamps
rm -f /tmp/values
for entry in $entries
do
	if [ $(expr $i % 2) == 0 ]
	then
		echo "format_date($entry * 1000 )," >> /tmp/timestamps
	else
		echo "$entry," >> /tmp/values
	fi
	let i=i+1;
done

# update our template with real values
export coverage=`cat /tmp/values`;
perl -pi -e 's/REPLACECOVERAGE/$ENV{coverage}/g' docs/code_coverage.html
export timestampsv=`cat /tmp/timestamps`;
perl -pi -e 's/REPLACETIMESTAMP/$ENV{timestampsv}/g' docs/code_coverage.html
