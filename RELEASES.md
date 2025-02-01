<!--
SPDX-FileCopyrightText: © 2024 David Bliss

SPDX-License-Identifier: GFDL-1.3-or-later
-->

# Releasing And Versioning

## Linux Distributions

The only supported Linux distribution is Flatpak and the only supported version of Fotema is the one [published to Flathub](https://flathub.org/apps/app.fotema.Fotema).

## Versioning

Naming format should be `v[major].[minor].[patch]`, such as `v1.15.0`.

## Releasing

To cut a new release to be published to Flathub perform the following steps:

1. Create a new release tag for the Git repository.  Example:

```shell
git tag -a -m "Release with new feature" v1.15.0
git push origin v1.15.0
```

2. To publish to Flathub the Flatpak manifest for Fotema must be updated to reference the new release. In the Flathub owned repository [app.fotema.Fotema](https://github.com/flathub/app.fotema.Fotema) edit the [app.fotema.Fotema.json](https://github.com/flathub/app.fotema.Fotema/blob/master/app.fotema.Fotema.json) manifest file to add the release tag and the Git commit hash for the tag. Example:

```json
{
"sources": [
  {
    "type": "git",
    "url": "https://github.com/blissd/fotema.git",
    "tag": "v1.15.0",
    "commit": "9ecfc1092b096908768a1a44fa0c12cae55b7ee8"
  },
  "cargo-sources.json"
]
}
```

This change should be pushed to new branched with the naming convention `prepare-v1.15.0`. Create a PR for that branch and GitHub Actions will automate the build for the new release. Merging the PR will automatically publish to Flathub.
