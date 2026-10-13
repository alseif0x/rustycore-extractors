#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
"""Bounded workstation acceptance. Client bytes never enter model output."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
import uuid


def emit(report, code):
    print(json.dumps(report, indent=2))
    return code


def main():
    mode = sys.argv[1] if len(sys.argv) == 2 else ""
    if mode not in {"live", "live-extract"}:
        return emit({"status": "error", "code": "invalid_mode"}, 2)
    root = Path(__file__).resolve().parent.parent
    work = root.parent / "rustycore-extractor-work"
    client = Path(os.environ["RCE_OPENCODE_CLIENT"]).resolve()
    try:
        raw = (client / ".build.info").read_bytes()
        if len(raw) > 65536:
            return emit({"status": "error", "code": "build_info_policy_limit"}, 2)
        rows = raw.decode("utf-8-sig").splitlines()
        header = [v.split("!")[0] for v in rows[0].split("|")]
        active = [dict(zip(header, row.split("|"))) for row in rows[1:]]
        active = [r for r in active if r.get("Product") == "wow_classic_beta" and r.get("Active") == "1"]
        if len(active) != 1 or active[0].get("Version") not in {"1.60.1.70170", "1.60.1.70205"}:
            return emit({"status": "error", "code": "unsupported_client"}, 3)
        build = int(active[0]["Version"].split(".")[-1])
    except (OSError, UnicodeError, IndexError, ValueError):
        return emit({"status": "error", "code": "client_metadata_unavailable"}, 4)
    run_id = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime()) + "-" + uuid.uuid4().hex[:8]
    run_root = Path(os.environ["RCE_OPENCODE_RUN_ROOT"]).resolve()
    if (run_root == client or client in run_root.parents or run_root in client.parents
            or any((p / ".git").exists() for p in [run_root, *run_root.parents])):
        return emit({"status": "error", "code": "unsafe_live_run_root"}, 4)
    run = run_root / ("opencode-" + run_id)
    run.mkdir(parents=True, mode=0o700)
    env = os.environ.copy()
    temporary = run / "temporary"
    temporary.mkdir(mode=0o700)
    env["TMPDIR"] = str(temporary)
    env["FOREVER_CASC_CLIENT"] = str(client)
    env["FOREVER_CASC_EXPECTED_BUILD"] = str(build)
    start = time.monotonic()
    if mode == "live":
        # The probe tests enforce their own syscall-level network denial.
        # Do not nest strace around them. Native sockets are disabled in the backend.
        cmd = [sys.executable, "-m", "unittest", "discover", "-s", "tests/forever", "-p", "test_*.py", "-v"]
    else:
        # Fixed default plan: ChrClasses and ChrRaces, no listfile/CDN discovery.
        cmd = ["strace", "-qq", "-f", "-e", "trace=network", "-e", "inject=network:error=EPERM",
               "-o", str(run / "network.trace"), env["FOREVER_DB2_EXTRACT"],
               "--client", str(client), "--locale", "esES", "--output", str(run / "tables")]
    try:
        result = subprocess.run(cmd, cwd=root, env=env, capture_output=True, text=True, timeout=600, check=False)
    except subprocess.TimeoutExpired:
        return emit({"status": "error", "code": "acceptance_timeout", "run_id": run_id}, 124)
    except OSError:
        return emit({"status": "error", "code": "acceptance_tool_unavailable", "run_id": run_id}, 4)
    after = hashlib.sha256((client / ".build.info").read_bytes()).hexdigest()
    unchanged = after == hashlib.sha256(raw).hexdigest()
    report = {"mode": mode, "run_id": run_id, "build": build, "exit_code": result.returncode,
              "elapsed_seconds": round(time.monotonic() - start, 3), "client_metadata_unchanged": unchanged,
              "paths_redacted": True, "full_client_extraction": False}
    code = result.returncode
    if mode == "live":
        text = (result.stdout + result.stderr).replace(str(client), "<client>").replace(str(work), "<work>")
        # unittest diagnostics from these tests contain status/count assertions, no rows.
        report["test_output"] = text[-16000:]
        report["tests_passed"] = code == 0
    else:
        try:
            data = json.loads(result.stdout)
            # Only reviewed diagnostic fields. Never echo unknown values or raw stdout.
            allowed = {"report_version", "status", "code", "build", "locale", "complete", "total_bytes",
                       "full_client_extraction", "paths_redacted", "files", "entries", "results"}
            item_allowed = {"name", "file_id", "status", "code", "bytes", "declared_records",
                            "table_schema_validated", "record_data_decoded", "decoded_rows"}
            clean = {}
            for k, v in data.items():
                if k not in allowed:
                    continue
                if isinstance(v, list):
                    clean[k] = [{a: b for a, b in row.items() if a in item_allowed} for row in v if isinstance(row, dict)]
                else:
                    clean[k] = v
            report["extractor_report"] = clean
        except (ValueError, TypeError, AttributeError):
            report["code"] = "invalid_extractor_report"
            code = 4
        trace = (run / "network.trace").read_text() if (run / "network.trace").exists() else ""
        attempted = bool(re.search(r"\b(?:socket|connect|send\w*|recv\w*|bind|listen|accept\w*)\(", trace))
        report["network_attempted"] = attempted
        if attempted:
            code = 4
            report["code"] = "network_attempt_rejected"
        if result.stderr:
            report["stderr_present"] = True
    if not unchanged:
        code = 4
        report["code"] = "client_metadata_changed"
    report["status"] = "passed" if code == 0 else "failed"
    dest = work / "reports" / "acceptance"
    dest.mkdir(parents=True, exist_ok=True)
    (dest / ("opencode-" + mode + "-" + run_id + ".json")).write_text(json.dumps(report, indent=2) + "\n")
    # Retain diagnostics locally, but never put client paths into model output.
    (run / "stdout.log").write_text(result.stdout)
    (run / "stderr.log").write_text(result.stderr)
    return emit(report, code)


if __name__ == "__main__":
    raise SystemExit(main())
