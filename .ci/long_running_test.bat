@echo OFF
IF "%1" equ "Schedule" (
  cargo install cargo-tarpaulin
  for /L %%a in (1,1,%2) do (
    echo "Running tests: %%a"
    cargo tarpaulin -p bmw_evh --skip-clean
    IF errorlevel 1 (
      EXIT /B 2
    )
  )
) ELSE (
  IF "%1" equ "Manual" (
    cargo install cargo-tarpaulin
    for /L %%a in (1,1,%2) do (
      echo "Running tests: %%a"
      cargo tarpaulin -p bmw_evh --skip-clean
      IF errorlevel 1 (
        EXIT /B 2
      )
    )
  ) ELSE (
    echo "not running, not the nightly build";
  )
)
@echo ON
