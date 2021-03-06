# Oxygen

Oxygen is a Voice Journal app written in rust.

## Building Oxygen

Clone the repository

```Powershell
    git clone "https://github.com/Eshanatnight/Oxygen.git"
```

Just use the cargo build tool.

```Rust
    cargo build --release
```

## Using Oxygen

```PowerShell
    Oxygen.exe <SUBCOMMAND>
```

## List Of Commands

| Command    |      Description              |
|------------|-------------------------------|
| -h, --help | Print the help Information    |
| record | Record the voice clip with the default input device untill `ctrl+c` is pressed |
| play | play the clip with the specified name. The name needs to be passed as a string |
| list | list all the clips |
| delete | delete the clip with the specified name. The name needs to be passed as a string |
|import| takes a path and the name of the clip, then imports the clip. If the name is not specified, the path is used|
|export| takes the path to where the file is to be exported and a name. The path should end in `.wav`|
|export-all| takes a path. `all` subcommand exports all the clips to the specified path|

## Known Issues

1. While Playing an audio clip, the app does not respond tokeyboard input as it is intended to be.

## Notes

- To build the audiopus lib, the repo cannot be in a portable data storeage device. Cargo and Cmake won't work properly.
