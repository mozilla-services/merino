/*!

# Choosing a logging library for Merino

- Status: accepted
- Date: 2021-07-28

Tracking issue: N/A

## Context and Problem Statement

Merino needs a way to generate cache keys for items it will store in the cache.
A natural way to do this is by hashing the input, and using the result of the
hash for the cache key. There are many hash keys available. Which one should
Merino use?

## Decision Drivers

- For Merino's common workloads, generating a hash should be low latency.
- The items that Merino hashes are relatively small, a few dozen bytes plus the
  user's query.
- Hashes should be stable across time (on multiple version of Merino) and space
  (on multiple instances of the same version of Merino).
- HashDoS protection is not a concern.

## Considered Options

1. [SipHash](https://en.wikipedia.org/wiki/SipHash)
2. [aHash](https://crates.io/crates/ahash)
3. [rustc-hash (aka FxHasher)](https://crates.io/crates/rustc-hash)
4. [HighwayHash](https://crates.io/crates/highway)
5. sha256 or similar

## Decision Outcome

Chosen option: option 4, HighwayHash, because it is network-safe, fast, and well
recommended.

## Pros and Cons of the Options <!-- optional -->

### Option 1 - SipHash

- [https://en.wikipedia.org/wiki/SipHash]()
- [https://doc.rust-lang.org/std/hash/struct.SipHasher.html]()

SipHash is a non-cryptographic hash algorithm used by default for Rust's hashing
needs, such as HashMaps. It is designed primarily to be resistent against
"HashDoS" attacks, in which an attacker can force hash collisions in a system
and overwhelm data structures like hashmaps and caches. It is faster than most
cryptographic hashes, such as sha256, but is generally slower than other hashes
considered.

In Rust's standard library, the standard way to use SipHash is with
`std::collections::hash_map::DefaultHasher`, which is not guaranteed to produce
stable output over time. Specifically, DefaultHasher may change to hashing
algorithm besides SipHash in the future.

- Good, because it is widely tested by Rust
- Good, because it is already available in the standard library
- Bad, because it spends resources on hashdos protection, which we don't need
- Bad, because Rust's `DefaultHash` is the normal way to use it, and
  `DefaultHash` may change in the future.

### Option 2 - aHash

- [https://crates.io/crates/ahash]()

AHash is designed with the explicit purpose of being the fastest HashDOS
resistant hash available in Rust. It is also designed specifically to be used
for in-memory hashmaps, and is not guaranteed to be stable over time or space.

This would be a viable candidate for any case where Merino uses in-memory
hashmaps that need high performance, but is not suitable for network hashing,
such as for Redis keys.

- Good, because it is very fast
- Bad, because it is not network-safe

### Option 3 - rustc-hash aka FxHash

 [https://crates.io/crates/rustc-hash]()

This is the hashing algorithm used internally by the Rust compiler, and is used
in some places in Firefox. It is also not designed to be network safe, though it
may be by accident. It is comparable in speed to aHash, depending on the input.
It is not resistant against HashDoS attacks, since it is not a keyed hashing
algorithm.

Notably, the aHash hash comparison suite claims that it is easy to accidentally
produce self-DoS conditions with this hashing algorithm, if the hash inputs are
not well chosen.

- Good, because it is used in rustc and Firefox
- Good, because it is relatively fast
- Bad, because it is not intended to be network safe
- Bad, because of claims about extreme weakness against DoS, including self-DoS

### Option 4 - HighwayHash

- [https://crates.io/crates/highway]()
- [https://github.com/google/highwayhash]()

HighwayHash is an algorithm developed by Google designed to be network safe,
strong against DoS attacks, and SIMD-optimized. It is recommended by
[the aHash README](https://crates.io/crates/ahash) as a better choice in
"network use or in applications which persist hashed values".

Notably, HighwayHash is relatively slow for small hash inputs, but relatively
fast for larger ones (though still not as fast as most non-network-safe
algorithms). Merino's hash inputs are near the boundary where it starts to be
faster than it's competition.

There is a predecessor to HighwayHash, FarmHash (and CityHash before it), that
are faster for smaller inputs. However, the libraries for these aren't
maintained anymore.

- Good, because it is relatively fast
- Good, because it is designed to be network-safe
- Good, because it is a "frozen" by Google, and won't change in the future
- Good, because it is actively maintained.
- Bad, because it's relatively slow for smaller keys.

### Option 5 - sha256 or similar

The SHA family of hashes are network and DoS safe. However, due to being
cryptographic hash functions are notably slower than non-cryptographic hashes.
For purposes where speed is not an issue, they are exceptionally safe and well
tested algorithms that should be considered.

- Good, because it is very safe
- Good, because it is very widely used and studied
- Bad, because it is slow
- Bad, because we pay for unneeded features

## Links <!-- optional -->

- [The Rust Performance Book :: Hashing](https://nnethercote.github.io/perf-book/hashing.html)
- [aHash's hash comparison suite](https://github.com/tkaitchuck/aHash/tree/master/compare)
- [SipHash](https://en.wikipedia.org/wiki/SipHash)
- [aHash](https://crates.io/crates/ahash)
- [rustc-hash (aka FxHasher)](https://crates.io/crates/rustc-hash)
- [HighwayHash](https://crates.io/crates/highway)

*/
