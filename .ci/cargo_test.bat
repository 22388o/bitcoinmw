git show --summary | findstr "Pipelines-Bot" > tmp.txt
set /p VAR11=<tmp.txt
IF [%VAR11%]==[] (
  rustup update
  cargo test --all --jobs 1
)
set "VAR11="
del tmp.txt