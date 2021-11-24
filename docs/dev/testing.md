# Testing strategies

There are three major testing strategies used in this repository: unit tests,
Rust integration tests, Python system tests, and Python load tests.

## Unit Tests

Unit tests should appear close to the code they are testing, using standard Rust
unit tests. This is suitable for testing complex behavior at a small scale, with
fine grained control over the inputs.

```rust
fn add_two(n: u32) -> u32 {
    n + 2
}

#[cfg(test)]
mod tests {
    #[test]
    fn add_two_works() {
        assert_eq!(add_two(3), 5, "it should work");
    }
}
```

## Integration tests

Many behaviors are difficult to test as unit tests, especially details like the
URLs we expose via the web service. To test these parts of Merino, we have
[`merino-integration-tests`][test-crate], which starts a configurable instance
of Merino with mock data sources. HTTP requests can then be made to that server
in order to test its behavior.

[test-crate]: ../../../merino_integration_tests/

```rust
#[actix_rt::test]
async fn lbheartbeat_works() {
    merino_test(
        |_| (),
        |TestingTools { test_client, .. }| async move {
            let response = test_client
                .get("/__lbheartbeat__")
                .send()
                .await
                .expect("failed to execute request");

            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.content_length(), Some(0));
        },
    )
    .await
}
```

For more details, see the documentation of the `merino-integration-tests` crate.

## Contract tests

The tests in the `test-engineering/contract-tests` directory are contract tests
that consume Merino's APIs using more opaque techniques. These tests run against
a Docker container of the service, specify settings via environment variables,
and operate on the HTTP API layer only and as such are more concerned with
external contracts and behavior. The contract tests cannot configure the server
per test.

For more details see the README.md file in the `test-engineering/contract-tests`
directory.

## Load tests

The tests in the `test-engineering/load-tests` directory are load tests that
spawn multiple HTTP clients that consume Merino's API. These tests do not run on
CI. We run them manually to simulate real-world load on the Merino infrastructure.
