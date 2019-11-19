# Autocopier
A program to automatically copy files according to a `.json` configuration.

## Why?
I found myself copying files on to a directory which was shared with a VM, and then on the VM copy them further to the place they needed to be.
This program came to be to facilitate this, and automatically copy the files over when they change.

## How to use
This program is meant to run twice, in my situation once on my host machine, to copy files over to the shared drive, and once in the VM to copy them there.

The configuration you can pass is described below, ranging from the basic ones to the ones with some more features.
### Basic configuration
T
```json
{
    "files": [
        {
            "from": "C:\\example.txt",
            "through": "C:\\example_middle.txt",
            "to": "C:\\example_final.txt"
        }
    ]
}
```

### Aliases

### Multiple files
You can specify multiple string within `{` and `}`, which will inspect all paths with any of these. So `test.{txt,json}` will watch both `test.txt` and `test.json`.

Example:
```json
{
    ...
    "files": [
        {
            "from": "C:\\example.{txt,json,derp}",
            "through": "C:\\example_middle.{txt,json,derp}",
            "to": "C:\\example_final.{txt,json,derp}"
        }
    ]
}
```
## -h output
```
autocopier 0.2.0
Midas Lambrichts <midaslamb@gmail.com>
Watches files for changes and, on file change, copies it according to some configuration.

USAGE:
    autocopier.exe [FLAGS] [OPTIONS]

FLAGS:
    -h, --help             Prints help information
    -w, --print_watched    Print the files that will be watched and exit.
    -p, --use_polling      Use polling instead of using the OS events.
    -V, --version          Prints version information

OPTIONS:
    -f, --file <configurationfile>    The configuration file, in json format. Defaults to configuration.json.
    -s, --step <step>                 The step in the copy chain. Possible values are 'start' and 'end'. Defaults to 'end'.

```