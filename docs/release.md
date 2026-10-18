# Repository-owned native releases

The accepted version commit on main is tagged `v<package version>`. `release.yml` requires the exact main CI to have succeeded, checks the tag against Cargo metadata, reruns tests/static analysis, builds a Linux musl executable and preserves upstream dependency license material. No provider credentials are used in build or release.

Preparation has only source/actions read access. Publication downloads the exact artifact ID, verifies its bound archive hash and creates CI provenance using GitHub OIDC. Only that job receives repository contents/attestation write. It never executes package code. An existing release is read back and compared; it is not overwritten. A later run cannot create a release after an uncertain first attempt.

Each archive includes the binary, source/version/target binding, file checksums, owner license/notice and resolved dependency license material. All public downloads are independent of private management repositories. Validate the published bytes again before consumer acceptance.
