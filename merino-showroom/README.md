# Merino Showroom

Showroom is a small JS demo to interact with Merino independent of the
implementation in Firefox's UI, for testing and demonstration purposes.

To use the showroom, first start an instance of `merino` with
`cargo run -p merino`. Then, in another terminal start Showroom by running:

```shell
# From the repository root
$ cd merino-showroom
$ npm install
$ npm run dev
```

This will start a server on [localhost:3000](http://localhost:3000) that is
configured to connect to the default configuration of `merino-web`.

Note, Node 16 or higher is required to run Showroom.
