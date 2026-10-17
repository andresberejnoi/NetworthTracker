# Spec Delta

## Purpose

Defines a single, date-based release version for the product and ensures every
part of it reports the same value, so releases and container images can be
identified unambiguously.

## ADDED Requirements

### Requirement: Date-based semantic version format
The product SHALL use versions of the form `MAJOR.MINOR.yyyymmdd`, where `MAJOR`
and `MINOR` are sequential integers and `yyyymmdd` is the UTC date of the
release, and SHALL NOT use a numeric patch segment.

#### Scenario: Version tagged today
- **WHEN** a release is tagged on 17 October 2026
- **THEN** its version is `0.1.20261017`

#### Scenario: Date segment is a real date
- **WHEN** any version string is produced
- **THEN** the `yyyymmdd` segment parses as a valid calendar date

#### Scenario: Incompatible change increments MAJOR
- **WHEN** a release makes an incompatible change
- **THEN** `MAJOR` is incremented and `MINOR` resets to `0`

### Requirement: Single authoritative version, mirrored everywhere
The version SHALL have one authoritative source, and the backend, the frontend,
and the API contract SHALL agree with it.

#### Scenario: API reports the authoritative version
- **WHEN** a client calls `GET /api/v1/ping`
- **THEN** the response `version` equals the authoritative version and matches the required format

#### Scenario: Frontend agrees with the backend
- **WHEN** the frontend is built
- **THEN** its package version equals the authoritative version

### Requirement: Release artifacts carry the version
Released container images and version control tags SHALL carry the same version.

#### Scenario: Image is tagged with the version
- **WHEN** a release image is published
- **THEN** it is tagged with the product version (for example `v0.1.20261017`)
