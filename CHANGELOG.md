# Changelog

## 3.1.3 (unreleased)

- Allow specifying krist address via KRISTFORGE_ADDRESS environment variable

## 3.1.2

- Revert Intel default thread count change from version 3.1.1
- Set CPU mining threads to lower priority to avoid crippling system performance

## 3.1.1

- On Intel systems, uses fewer threads by default to avoid crippling system performance
- Improved UI a bit

## 3.1.0

- Added CPU support including two mining kernels:
    - `unoptimized` which makes no assumptions about instruction sets and should work everywhere
    - `SHA` which takes advantage of the SHA instruction set on recent processors to offer dramatically better speeds
- Updated dependencies

## 3.0.0-alpha

- Initial release with only GPU support via OpenCL
