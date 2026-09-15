_default:
  @just --choose

run cmd:
  cargo run -- {{cmd}}

run-init:
  just run init
