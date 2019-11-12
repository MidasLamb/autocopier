# Autocopier
## -h output
'''
autocopier 0.1.0
Midas Lambrichts <midaslamb@gmail.com>
Watches files for changes and, on file change, copies it according to some configuration.

USAGE:
    autocopier.exe [FLAGS] [OPTIONS]

FLAGS:
    -h, --help           Prints help information
    -p, --use_polling    Use polling instead of using the OS events.
    -V, --version        Prints version information

OPTIONS:
    -f, --file <configurationfile>    The configuration file, in json format. Defaults to configuration.json.
    -s, --step <step>                 The step in the copy chain. Possible values are 'start' and 'end'. Defaults to 'end'.
'''