`journal.journal` contains the three entries in `journal.export`. It tests known
cutoff timestamps, absent boot IDs, and unique field enumeration without a
running journal service. The file uses no compression, keyed hashes, or compact
objects so it can also be read by older or minimal libsystemd builds.

To regenerate it, remove the old output file and run this command from the
repository root with `systemd-journal-remote` installed:

```sh
SYSTEMD_JOURNAL_KEYED_HASH=0 SYSTEMD_JOURNAL_COMPACT=0 \
    systemd-journal-remote --compress=no --seal=no --split-mode=none \
    --max-file-size=512K --output=tests/fixtures/journal.journal \
    tests/fixtures/journal.export
```

The checked-in fixture was generated with systemd v262. Journal header IDs may
change when regenerating it; tests only depend on the exported entries.
