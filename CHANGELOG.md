# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
## [0.1.0](https://gitlab.com/ThomasTartrau/workflo/releases/tag/workflo-v0.1.0) - 2026-09-11

### Added

- initial workflow engine implementation


### Fixed

- *(ci)* keep .cargo/audit.toml tracked while ignoring rest of .cargo/

- *(ci)* add .cargo/ to gitignore to fix release-plz uncommitted changes error

- #1 bump CI image to Rust 1.88 and ignore unfixed rsa advisory

- #1 add optional dependency on build-ci-image to prevent race condition

