# httpp

`httpp` is:

1. A minimal structured file format to represent HTTP requests. See the [schema](schema.json) for details.
2. A reference CLI tool which can execute requests defined in the `httpp` format.

## Why

HTTP is a simple protocol, but there isn't a simple way to represent HTTP requests in a structured format.

Existing formats like HAR are too verbose and complex for simple use cases.

`httpp` aims to be a simple and minimal format to represent HTTP requests that aren't tied to a specific tool like
Postman or curl.

## Usage

```
Usage: httpp [OPTIONS]

Options:
  -f, --file <FILE>  Read the request from a file
      --curl         Convert the request to a curl command
  -s, --status       Output the response status code
  -H, --headers      Output the response headers
  -b, --body         Output the response body
  -h, --help         Print help
  -V, --version      Print version
```

Some examples...

You can pass a `httpp` file to the CLI with the `-f` flag:

```
httpp -f get-request.json
```

or you can pipe the `httpp` file to the CLI:

```
cat get-request.json | httpp
```

If you're just interested in the response status code, you can use the `-s` flag:

```
httpp -f get-request.json -s
```

To output everything in the response (status, headers, and body):

```
httpp -f get-request.json -sHb
```

Finally, to convert a `httpp` request to a curl command, you can use the `--curl` flag:

```
httpp -f get-request.json --curl
```