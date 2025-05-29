---
name: Release
about: Create a new release for VexFS
title: 'Release v[VERSION]'
labels: 'release'
assignees: ''
---

## Release Checklist

### Pre-Release
- [ ] All tests are passing on main branch
- [ ] Security scans are clean
- [ ] Documentation is up to date
- [ ] Version numbers are updated in:
  - [ ] `Cargo.toml`
  - [ ] `bindings/python/Cargo.toml`
  - [ ] `bindings/typescript/package.json`
- [ ] CHANGELOG.md is updated
- [ ] All dependencies are up to date

### Release Process
- [ ] Create and push release tag: `git tag v[VERSION] && git push origin v[VERSION]`
- [ ] Verify GitHub Actions workflows complete successfully:
  - [ ] CI workflow passes
  - [ ] Release workflow creates GitHub release
  - [ ] Docker images are built and published
  - [ ] Python SDK is published to PyPI
  - [ ] TypeScript SDK is published to npm
  - [ ] Documentation is deployed

### Post-Release
- [ ] Verify packages are available:
  - [ ] Rust crate on crates.io
  - [ ] Python package on PyPI
  - [ ] TypeScript package on npm
  - [ ] Docker images on GitHub Container Registry
- [ ] Test installation from published packages
- [ ] Update any dependent projects
- [ ] Announce release on relevant channels

### Version Information
- **Version**: v[VERSION]
- **Release Type**: [ ] Major [ ] Minor [ ] Patch [ ] Pre-release
- **Breaking Changes**: [ ] Yes [ ] No

### Release Notes
<!-- Add release notes here -->

### Additional Notes
<!-- Any additional information about this release -->