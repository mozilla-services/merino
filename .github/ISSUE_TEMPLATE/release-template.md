---
name: New Release Template
about: New Release checklist and template
title: "chore: tag {version}"
labels: ""
assignees: ""
---

# Release Checklist

- [ ] Switch to the `main` branch.
- [ ] `git pull` to ensure the local copy is completely up-to-date.
- [ ] `git diff origin/main` and `git status` to ensure there are no local
      staged or uncommitted changes.
- [ ] Run local testing to ensure no artifacts or other local changes that might
      break tests have been introduced.
- [ ] Change to the release branch.
  - If this is a new major/minor release,
    `git checkout -b release/{major}.{minor}` to create a new release branch.
  - If this is a new patch release:
    - `git checkout release/{major}.{minor}`
    - `git pull` to ensure the branch is up-to-date.
    - Commit the needed changes to the branch.
  - Note: For the remainder of this list `{version}` will refer to the
    `{major}.{minor}.{patch}` you've specified.
- [ ] Edit wherever the version is in source (`Cargo.toml` of all crates) so
      that the version number reflects the desired release version.
- [ ] `clog --setversion {version} --from-latest-tag --outfile CHANGELOG.md`, verify changes were
      properly accounted for in `CHANGELOG.md`.
- [ ] `git add CHANGELOG.md Cargo.* */Cargo.*` to add the changes to the new
      release commit.
- [ ] `git commit -m "chore: tag {version}"` to commit the new version and
      record of changes.
- [ ] `git tag -s -m "chore: tag {version}" v{version}` to create a signed tag
      of the current HEAD commit for release. (note `vX.Y.Z` format)
- [ ] `git push --set-upstream origin release/{major}.{minor}` to push the
      commits to a new origin release branch.
- [ ] `git push --tags origin release/{major}.{minor}` to push the tags to the
      release branch.
- [ ] Submit a pull request on Github to merge the release branch to `main`.
      Include the text `Refs #XX` where XX is the number of this release issue.
- [ ] Wait for that PR to be merged.
- [ ] Go to the [releases](https://github.com/mozilla-services/merino/releases)
      page, you should see the new tag with no release information under it.
- [ ] Click the **Draft a new release** button.
- [ ] Enter {version} for _Tag version_.
- [ ] Copy/paste the changes from `CHANGELOG.md` into the release description
      omitting the top 2 lines (the name HTML and the version) of the file.
- [ ] Wait for the CircleCI job `docker-image-publish` to build and publish the
      Docker image. [You can see all the main branch jobs here][circle-ci-main].
- [ ] Wait for Merino to be deployed to stage (It happens automatically). You
      can see version information for stage [at it's `/__version__`
      endpoint][merino-stage-version]
- [ ] Update the [release issue template][] with any needed changes.
- [ ] Close this issue.

[release issue template]:
  https://github.com/mozilla-services/merino/blob/main/.github/ISSUE_TEMPLATE/release-template.md
[circle-ci-main]:
  https://app.circleci.com/pipelines/github/mozilla-services/merino?branch=main
[merino-stage-version]:
  https://stage.merino.nonprod.cloudops.mozgcp.net/__version__
