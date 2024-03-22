#!/bin/bash

cp docs/code_coverage.html.template docs/code_coverage.html;
cp lib.rs.template src/lib.rs;
# copy tarpaulin output into template
export tarpaulin_output=`cat /tmp/tarpaulin.out`;
perl -pi -e 's/REPLACETARPAULINOUTPUT/$ENV{tarpaulin_output}/g' docs/code_coverage.html

# read in data from summary
entries=`cat docs/tarpaulin_summary.txt`;
declare -a timestamps;
declare -a values;
i=0;
last_value=0.0;
rm -f /tmp/timestamps
rm -f /tmp/values
for entry in $entries
do
	if [ $(expr $i % 2) == 0 ]
	then
		echo "format_date($entry * 1000 )," >> /tmp/timestamps
	else
		last_value = $entry;
		echo "$entry," >> /tmp/values
	fi
	let i=i+1;
done

# update our template with real values
export coverage=`cat /tmp/values`;
export last_entry="$last_value";
perl -pi -e 's/REPLACECOVERAGE/$ENV{coverage}/g' docs/code_coverage.html
perl -pi -e 's/REPLACECOVERAGE_SINGLE/$ENV{last_entry}/g' docs/code_coverage.html
perl -pi -e 's/REPLACECOVERAGE_SINGLE/$ENV{last_entry}/g' src/lib.rs
export timestampsv=`cat /tmp/timestamps`;
perl -pi -e 's/REPLACETIMESTAMP/$ENV{timestampsv}/g' docs/code_coverage.html
