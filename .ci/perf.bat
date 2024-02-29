@echo OFF
git show --summary | findstr "Author" | findstr "Pipelines-Bot" > tmp.txt
set /p VAR11=<tmp.txt
IF "%VAR11%" equ "" (
  cd etc\evh_perf
  cargo build --release --jobs 1
  target\release\evh_perf -e -c -i 100 --count 10 --clients 2 -t 10
  IF %ERRORLEVEL% GEQ 1 EXIT /B 2
) ELSE (
  IF "%1" equ "Schedule" (
    cd etc\evh_perf
    cargo build --release --jobs 1
    target\release\evh_perf -e -c -i 100 --count 10 --clients 2 -t 10 
    IF %ERRORLEVEL% GEQ 1 EXIT /B 2
  ) ELSE (
      IF "%1" equ "Manual" (
        cd etc\evh_perf
        cargo build --release --jobs 1
        target\release\evh_perf -e -c -i 100 --count 10 --clients 2 -t 10
	IF %ERRORLEVEL% GEQ 1 EXIT /B 2
      )
  )
)
set "VAR11="
del tmp.txt
@echo ON
