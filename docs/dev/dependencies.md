# Development Dependencies

Merino uses a Redis-based caching system, and so requires a Redis instance to
connect to.

To make things simple, Redis (and any future service dependencies) can be
started with Docker Compose, using the `docker-compose.yaml` file in the `dev/`
directory. Notably, this does not run any Merino components that have source
code in this repository.

```shell
$ cd dev
$ docker-compose up
```

This Dockerized set up is optional. Feel free to run the dependent services by
any other means as well.

### Dev Helpers

The docker-compose setup also includes some services that can help during
development.

- Redis Commander, http://localhost:8081 - Explore the Redis database started
  above.
- Statsd Logger - Receives statsd metrics emitted by Merino (and any thing else
  on your system using statsd). Available through docker-compose logs. For
  example with `docker-compose logs -f statsd-logger`.
