git show --summary | findstr "Pipelines-Bot" > tmp.txt
set /p VAR11=<tmp.txt
IF [%VAR11%]==[] (
  cd etc\evh_perf
  cargo build --release --jobs 1
  target\release\evh_perf -e -c -i 100 --count 10 --clients 2 -t 10
)
set "VAR11="
del tmp.txt
