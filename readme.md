# 1C Simple Launcher

This is just a simple command-line tool to make launching 1C:Enterprise databases a bit easier.

It takes a connection string, figures out if it's a server, file, or web database, and then launches `1cestart.exe` with the right parameters.

## How to Use

Run the executable with your connection string (make sure to wrap it in quotes if it has spaces):

```sh
# Launch in standard (Enterprise) mode
rbaserun.exe 'my-server;my-base'
```

### Designer Mode

To open the database in **Designer** (Configurator) mode, just add the `-d` or `--designer` flag:

```sh
rbaserun.exe -d 'File="C:\my_bases\test_db";'
```

---

*Note*: for powershell you must put path string to **double** double-quotes.

```sh
./rbaserun.exe -d 'File=""C:\my_bases\test_db"";'
```

## Supported Connection Strings

The tool tries to be smart and parse a few common 1C path formats:

  * **Simple Server:**
    `my-server;my-base`

  * **Full Server String:**
    `Srvr="my-server";Ref="my-base";`

  * **File Path:**
    `File="C:\my_bases\test_db";`

  * **Web Service:**
    `ws="https://my-web-base.com/base";`

-----

## ⚠️ IMPORTANT WARNING

This tool **hardcodes** the path to the 1C starter executable (for now).

It *only* works if your `1cestart.exe` is located exactly at:
`c:\Program Files\1cv8\common\1cestart.exe`

If your 1C platform is installed somewhere else, this tool won't find it and will give you an error.
