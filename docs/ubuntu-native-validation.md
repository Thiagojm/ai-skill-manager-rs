# Ubuntu native acceptance

Status: pending. The user has Ubuntu 22.04 desktop available; Ubuntu 24.04 remains pending. CI compilation and package inspection do not establish native acceptance.

Use a disposable desktop environment or account. Download the `ubuntu-amd64` artifact from the [package run](https://github.com/Thiagojm/ai-skill-manager-rs/actions/runs/37046191784), extract it and run these commands inside its directory:

```bash
sha256sum -c SHA256SUMS.txt
cat SOURCE_COMMIT.txt
sudo apt install './AI Skill Manager_0.1.0_amd64.deb'
```

Expected source: `46a0db243a216d39e888b786ff1648ac49bb9c49`. Expected package SHA-256: `db0a61f56fd4b5ca4bf5d232ead5a169470437b1be06fe23405413edadca6634`.

Create disposable skills without using real harness installations:

```bash
validation_root=$(mktemp -d "$HOME/ai-skill-validation.XXXXXX")
mkdir -p "$validation_root/source/demo" "$validation_root/destination" "$validation_root/external"
printf '%s\n' '# Demo skill' > "$validation_root/source/demo/SKILL.md"
printf '#!/bin/sh\nprintf "demo\\n"\n' > "$validation_root/source/demo/run.sh"
chmod 755 "$validation_root/source/demo/run.sh"
printf '%s\n' 'external target must remain' > "$validation_root/external/resource.txt"
ln -s "$validation_root/external/resource.txt" "$validation_root/source/demo/linked.txt"
printf '%s\n' "$validation_root"
```

Record Ubuntu version, desktop/session type, package installation output and each outcome:

- Launch from the application menu; inspect launcher and window icons.
- Pick the disposable source and destination; verify Open folder. Add a custom harness pointing at the destination. Check built-in detection using only disposable account configuration folders.
- Install demo with link acknowledgement. Confirm `run.sh` remains executable, `linked.txt` is a regular materialized file and the external target is unchanged.
- Add an obsolete file to the installed demo, modify source content and change only a script permission. Confirm differences appear and Update replaces the whole folder, removing the obsolete file.
- Change source after opening a confirmation. Confirm the stale operation is refused and rescan before retrying.
- Uninstall demo. Confirm it appears in the desktop trash, restore it manually through the file manager and verify contents/permissions. The external target must remain throughout.
- Check themes, keyboard tab navigation/dialog Escape, remembered paths/custom harness/theme after restart, and unavailable path recovery.
- Remove only the application package with `sudo apt remove ai-skill-manager`. Confirm disposable skills and app settings remain. Record the settings location shown by the environment; do not delete it before checking.

Keep the disposable paths and trash entries until evidence is collected. Report failures with operation/error text. Mark 24.04 untested until the same native checks are completed there.
