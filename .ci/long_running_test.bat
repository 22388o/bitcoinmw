@echo OFF
IF "%1" equ "Schedule" (
  cargo install cargo-tarpaulin
  for /L %%a in (1,1,300) do (
    echo "Running tests: %%a"
    cargo tarpaulin -p bmw_evh2 --skip-clean
    IF errorlevel 1 (
      EXIT /B 2
    )
  )
) ELSE (
  echo "not running, not the nightly build";
)
@echo ON
