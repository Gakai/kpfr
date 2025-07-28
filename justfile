cargo := require("cargo")

[private]
@list:
	{{just_executable()}} --list --unsorted

run:
	{{cargo}} run

build:
	{{cargo}} build --release

add *ARGS:
	{{cargo}} add {{ARGS}}
