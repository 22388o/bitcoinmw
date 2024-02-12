@echo OFF
git show --summary | findstr "Author" | findstr "Pipelines-Bot" > tmp.txt
set /p VAR11=<tmp.txt
IF "%VAR11%" equ "" (
  rustup update
  cargo test --all --jobs 1
)
set "VAR11="
del tmp.txt
@echo ON
