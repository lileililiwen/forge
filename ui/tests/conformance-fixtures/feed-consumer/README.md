# Fixture: a feed consumer

The `file:` specifiers in `package.json` resolve to tarballs the consumer commits
under `vendor/platform-ui/`. The test driver copies `ui/feed/*.tgz` here and
writes `ui-conformance.json` from `ui/feed/feed.json`, because a real consumer
commits those bytes itself. No sibling checkout, no absolute path, no environment
variable, no registry.
