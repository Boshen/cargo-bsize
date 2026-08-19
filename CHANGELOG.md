# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- analyze native `cdylib` targets with `--cdylib`

## [0.0.2](https://github.com/Boshen/cargo-bsize/compare/v0.0.1...v0.0.2) - 2026-08-19

### Added

- rank macros by the source they expand to
- cost each workspace file and directory
- roll inlined code up by caller and by origin crate
- render the report as Markdown
- list the loops the optimizer unrolled, peeled, or vectorized
- rank generic definitions by what they cost to monomorphize
- count the pointer slots in data and what the loader charges for them
- show the source text under the workspace lines
- report each dependency's features and who asked for them
- name what moved under each what-if lever, and add the nightly levers
- read the constant data by shape from the assembly
- cost each crate version, site each function, and lay out each type from DWARF

### Fixed

- drop the Mach-O header symbol from the code tables
- connect anonymous constants into the reference graph
- leave a duplicate version's empty code cost blank
- say what a zero-byte duplicate version means
- clarify agent instructions

### Other

- add cargo-show-asm to prior art
- build release binaries
- drop the JSON output
- describe the new views and the what-if levers
