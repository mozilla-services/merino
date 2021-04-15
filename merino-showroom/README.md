# Merino Showroom

This page provides a UI to interact with Merino independently of the
implementation in Firefox's UI, for testing and demonstration purposes.

To use the showroom, first start an instance of `merino-web` with `cargo run
-p merino-web`. Then, in another terminal start a dev server by running

```shell
# From the repository root
$ cd merino-showroom
$ npm install
$ npm run dev
```

This will start a server on [http://localhost:3000][] that is configured to
connect to the default configuration of `merino-web`.
