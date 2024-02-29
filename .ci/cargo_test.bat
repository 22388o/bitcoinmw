@echo OFF
git show --summary | findstr "Author" | findstr "Pipelines-Bot" > tmp.txt
set /p VAR11=<tmp.txt
IF "%VAR11%" equ "" (
  rustup update
  set ERRORLEVEL=0
  cargo test --all --jobs 1
  echo "e=%ERRORLEVEL%"
  IF %ERRORLEVEL% GEQ 1 (
    EXIT /B 2
  )
) ELSE (
  IF "%1" equ "Schedule" (
    rustup update
    set ERRORLEVEL=0
    cargo test --all --jobs 1
    echo "e=%ERRORLEVEL%"
    IF %ERRORLEVEL% GEQ 1 (
      EXIT /B 2
    )
  ) ELSE (
    IF "%1" equ "Manual" (
      rustup update
      set ERRORLEVEL=0
      cargo test --all --jobs 1
      echo "e=%ERRORLEVEL%"
      IF %ERRORLEVEL% GEQ 1 (
        EXIT /B 2
      )
    )
  )
)
set "VAR11="
del tmp.txt
@echo ON
