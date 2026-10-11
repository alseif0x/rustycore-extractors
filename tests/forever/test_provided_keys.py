#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-3.0-or-later
# Authored 2026-10-11: original synthetic native/Rust supplied-key acceptance.
# Run explicitly with external laboratory/build/run paths. No client inputs,
# downloads, upstream source import or real keys are used by this program.
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import time
import zlib

PIN = "02245dcd245e7433e524577656177723d3e4992e"
DECRYPT_SHA = "e7c319e928c9ee5584ede3671d449600c6283ec4aa113582a172a827f52fc89a"


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def little(value, size=4):
    return value.to_bytes(size, "little")


def framed(frames):
    header = b"BLTE" + struct.pack(">I", 12 + 24 * len(frames))
    header += b"\x0f" + len(frames).to_bytes(3, "big")
    for frame, plain in frames:
        header += struct.pack(">II", len(frame), len(plain)) + hashlib.md5(frame).digest()
    return header + b"".join(frame for frame, _ in frames)


def generate():
    cases = []
    profiles = [(0, 0), (0, 8), (0x0123456789ABCDEF, 8), ((1 << 64) - 1, 8)]
    lengths = [0, 1, 62, 63, 64, 65, 127, 128, 129, 255, 256, 257, 1024]
    for group, (name, name_size) in enumerate(profiles):
        for nonce_size in [4, 8]:
            key = bytes((group * 53 + nonce_size * 3 + i * 13) % 256 for i in range(16))
            nonce = bytes((17 * group + 7 * i + nonce_size) % 256 for i in range(nonce_size))
            for mode in [b"N", b"Z"]:
                for length in lengths:
                    plain = bytes((i * 29 + group * 11 + nonce_size) % 256 for i in range(length))
                    inner = mode + (plain if mode == b"N" else zlib.compress(plain, 9))
                    cases.append(dict(name=name, name_size=name_size, key=key, nonce=nonce,
                                      index=0, plain=plain, inner=inner, group=group, mode=mode))
                for index in [1, 3, 255, 256]:
                    plain = bytes((i * 7 + group + index) % 256 for i in range(129))
                    inner = mode + (plain if mode == b"N" else zlib.compress(plain, 9))
                    cases.append(dict(name=name, name_size=name_size, key=key, nonce=nonce,
                                      index=index, plain=plain, inner=inner, group=group, mode=mode))
    assert len(cases) == 272
    return cases


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ["repo", "lab", "run-root", "native-root", "target", "crypto"]:
        parser.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    repo, lab, native = args.repo.resolve(), args.lab.resolve(), args.native_root.resolve()
    run_root = args.run_root.resolve()
    assert run_root.is_dir() and not any((parent / ".git").exists() for parent in [run_root, *run_root.parents])
    run_id = "provided-keys-" + time.strftime("%Y%m%dT%H%M%SZ", time.gmtime()) + "-" + os.urandom(4).hex()
    run = run_root / run_id
    run.mkdir(mode=0o700)
    artifacts = run / "synthetic"
    artifacts.mkdir(mode=0o700)
    report_path = lab / "reports" / "acceptance" / (run_id + ".json")
    report_path.parent.mkdir(parents=True, exist_ok=True)
    pointer = Path("/tmp/rce-provided-keys-native.json")
    source = native / "source" / "dep" / "CascLib" / "src"
    original = source / "CascDecrypt.cpp"
    assert sha(original) == DECRYPT_SHA
    archive_paths = [native / "libforever_casc_core.a", native / "libforever_zlib.a", args.crypto.resolve()]
    archive_hashes = {str(path): sha(path) for path in archive_paths}
    source_paths = [
        repo / "tests/forever/native_provided_keys.cpp", repo / "tests/forever/provided_keys.rs",
        repo / "tests/forever/test_provided_keys.py", repo / "crates/forever-casc/src/blte.rs",
        repo / "crates/forever-casc/src/blte/md5.rs", repo / "crates/forever-casc/src/keys.rs",
        repo / "crates/forever-casc/src/keys/salsa20.rs", repo / "crates/forever-casc/Cargo.toml",
        repo / "Cargo.lock",
    ]
    before = {str(path.relative_to(repo)): sha(path) for path in source_paths}
    commands, traces, compiler_traces, process_results = [], [], [], []

    def execute(label, command, binary=False, compiler_ipc=False):
        trace, log = run / (label + ".network.trace"), run / (label + ".stderr.log")
        output = run / (label + (".bin" if binary else ".stdout.log"))
        wrapped = ["strace", "-f", "-qq", "-e", "signal=none", "-e", "trace=%network"]
        if not compiler_ipc:
            wrapped += ["-e", "inject=%network:error=EPERM"]
        wrapped += ["-o", str(trace), *map(str, command)]
        commands.append(dict(label=label, argv=wrapped))
        (run / "commands.json").write_text(json.dumps(commands, indent=2) + "\n")
        with output.open("xb") as stdout, log.open("xb") as stderr:
            result = subprocess.run(wrapped, stdout=stdout, stderr=stderr, cwd=repo, check=False, timeout=90)
        (compiler_traces if compiler_ipc else traces).append(trace)
        process_results.append(dict(label=label, exit=result.returncode))
        if result.returncode:
            raise RuntimeError(f"{label} failed, retained stderr in {log}")
        if compiler_ipc:
            observed = trace.read_text()
            calls = re.findall(r"\b([a-z_]+)\(", observed)
            assert set(calls) <= {"socketpair", "sendmsg", "recvmsg", "recvfrom"}, observed
            assert "AF_INET" not in observed and "connect(" not in observed and "sendto(" not in observed
            assert all("AF_UNIX" in line for line in observed.splitlines() if "socketpair(" in line)
            local_fds = set()
            for line in observed.splitlines():
                pair = re.search(r"^(\d+)\s+socketpair\(AF_UNIX,.*\[(\d+), (\d+)\]\)\s+= 0$", line)
                if pair:
                    local_fds.update((pair[1], fd) for fd in pair.group(2, 3))
                message = re.search(r"^(\d+)\s+(?:sendmsg|recvmsg|recvfrom)\((\d+),", line)
                if message:
                    assert message.group(1, 2) in local_fds, line
        else:
            assert trace.stat().st_size == 0, f"unexpected network attempt: {label}"
        return output

    native_bin = run / "native-provided-keys"
    execute("build-native", ["c++", "-std=c++20", "-O2", "-DCASC_USE_SYSTEM_ZLIB",
            "-DCASCLIB_NO_AUTO_LINK_LIBRARY", "-DCASCLIB_NODEBUG", "-DZLIB_CONST", "-MMD", "-MF", run / "native.d",
            "-I" + str(source), "-I" + str(native / "source" / "dep"),
            "-I" + str(repo / "native/forever-casc"),
            "-I" + str(repo / "vendor/trinitycore-legacy/dep/zlib"),
            repo / "tests/forever/native_provided_keys.cpp",
            *archive_paths[:2], archive_paths[2], "-pthread", "-o", native_bin])
    rust_bins, rlibs = {}, {}
    for profile in ["debug", "release"]:
        deps = args.target.resolve() / profile / "deps"
        candidates = list(deps.glob("libforever_casc-*.rlib"))
        assert candidates, f"missing prebuilt {profile} forever_casc library"
        selected = max(candidates, key=lambda path: path.stat().st_mtime_ns)
        # Freshness binds this acceptance to the current encrypted implementation;
        # parent owns workspace compilation, so this driver never runs Cargo.
        assert selected.stat().st_mtime_ns >= max(path.stat().st_mtime_ns for path in source_paths[3:7])
        rlibs[profile] = dict(path=str(selected), sha256=sha(selected))
        rust_bins[profile] = run / ("rust-provided-keys-" + profile)
        execute("build-rust-" + profile, ["rustc", "--edition=2024", "-Funsafe_code", "-C",
                "opt-level=" + ("0" if profile == "debug" else "3"), "--extern",
                "forever_casc=" + str(selected), "-L", "dependency=" + str(deps),
                repo / "tests/forever/provided_keys.rs", "-o", rust_bins[profile]], compiler_ipc=True)

    cases = generate()
    data = b"RCEKEY01" + little(len(cases))
    for case in cases:
        data += little(case["name"], 8) + case["key"] + bytes([case["name_size"], len(case["nonce"])])
        data += case["nonce"] + little(case["index"]) + little(len(case["inner"])) + case["inner"]
    input_path = artifacts / "native-input.bin"
    input_path.write_bytes(data)
    native_output = execute("native-ciphertext", [native_bin, input_path], binary=True)
    result = native_output.read_bytes()
    assert result[:8] == b"RCEKOUT1" and int.from_bytes(result[8:12], "little") == len(cases)
    cursor = 12
    for case in cases:
        size = int.from_bytes(result[cursor:cursor + 4], "little")
        cursor += 4
        assert 0 < size <= 1024 * 1024
        case["frame"] = result[cursor:cursor + size]
        cursor += size
        size = int.from_bytes(result[cursor:cursor + 4], "little")
        cursor += 4
        assert result[cursor:cursor + size] == case["inner"]
        cursor += size
        assert case["frame"][0] == ord("E")
    assert cursor == len(result)
    objects = []
    for index, case in enumerate(cases):
        keyfile = f"key{index:03}.keys"
        (artifacts / keyfile).write_text("forever-tact-keys\t1\n" +
                f"{case['name']:016x}\t{case['key'].hex()}\n")
        case["keyfile"] = keyfile
        if case["index"] == 0:
            objects.append((b"BLTE\0\0\0\0" + case["frame"], case["plain"], keyfile, "headerless"))
            objects.append((framed([(case["frame"], case["plain"])]), case["plain"], keyfile, "framed"))
        else:
            prefix = b"authored prefix"
            frames = [(b"N" + prefix, prefix)] + [(b"N", b"")] * (case["index"] - 1)
            frames.append((case["frame"], case["plain"]))
            objects.append((framed(frames), prefix + case["plain"], keyfile, "indexed"))
    for group in range(4):
        for nonce_size in [4, 8]:
            a = next(case for case in cases if case["group"] == group and len(case["nonce"]) == nonce_size
                     and case["mode"] == b"N" and case["index"] == 1)
            b = next(case for case in cases if case["group"] == group and len(case["nonce"]) == nonce_size
                     and case["mode"] == b"Z" and case["index"] == 3)
            assert a["key"] == b["key"] and a["name"] == b["name"]
            frames = [(b"Nmix", b"mix"), (a["frame"], a["plain"]), (b"Nmid", b"mid"), (b["frame"], b["plain"])]
            objects.append((framed(frames), b"mix" + a["plain"] + b"mid" + b["plain"], a["keyfile"], "mixed"))
    assert len(objects) == 488
    plan, records = [], []
    for index, (encoded, plain, keyfile, kind) in enumerate(objects):
        filename = f"case{index:03}.blte"
        (artifacts / filename).write_bytes(encoded)
        plan.append(f"{filename}\t{keyfile}\t{hashlib.md5(plain).hexdigest()}\t{len(plain)}\n")
        records.append(dict(case=index, kind=kind, encoded_bytes=len(encoded), decoded_bytes=len(plain),
                            encoded_sha256=hashlib.sha256(encoded).hexdigest(),
                            plaintext_sha256=hashlib.sha256(plain).hexdigest()))
    plan_path = artifacts / "plan.tsv"
    plan_path.write_text("".join(plan))
    corpus_hashes = {path.name: sha(path) for path in artifacts.iterdir()}
    binary_hashes = {"native": sha(native_bin), **{profile: sha(binary) for profile, binary in rust_bins.items()}}
    summaries = {}
    for profile, binary in rust_bins.items():
        output = run / ("decoded-" + profile)
        summary_path = execute("decode-" + profile, [binary, plan_path, output])
        summaries[profile] = json.loads(summary_path.read_text())
        assert summaries[profile]["cases"] == len(objects)
        assert summaries[profile]["decoded_bytes"] == sum(len(obj[1]) for obj in objects)
        assert len(list(output.iterdir())) == len(objects)
        for index, (_, plain, _, _) in enumerate(objects):
            assert (output / f"case{index:03}.blte.out").read_bytes() == plain
    assert summaries["debug"] == summaries["release"]
    assert before == {str(path.relative_to(repo)): sha(path) for path in source_paths}
    assert archive_hashes == {str(path): sha(path) for path in archive_paths}
    assert sha(original) == DECRYPT_SHA
    assert corpus_hashes == {path.name: sha(path) for path in artifacts.iterdir()}
    assert binary_hashes == {"native": sha(native_bin), **{profile: sha(binary) for profile, binary in rust_bins.items()}}
    assert all(sha(Path(value["path"])) == value["sha256"] for value in rlibs.values())
    report = dict(
        accepted=True, status="passed", run_id=run_id, run=str(run), reference=PIN,
        scope="original_CascDecrypt_supplied_synthetic_keys_vs_public_Rust_BLTE",
        native_synthetic_cases=len(cases), native_CascDecrypt_calls=2 * len(cases),
        rust_objects_per_profile=len(objects), rust_profiles=summaries,
        independent_python_plaintext_comparisons=2 * len(objects),
        default_no_keys_rejections=2 * len(objects),
        decoded_bytes_per_profile=sum(len(obj[1]) for obj in objects),
        native_wire_keyname_sizes=[0, 8], native_wire_nonce_sizes=[4, 8], native_key_bytes=16,
        actual_frame_indexes=[0, 1, 3, 255, 256], inner_modes=["N", "Z"],
        headerless_objects=208, framed_objects=208, indexed_objects=64, mixed_objects=8,
        original_builtin_key_loader_executed=False, native_CASC_routing_executed=False,
        LocalStore_executed=False, real_client_inputs=False, full_extractor_complete=False,
        native_decrypt_source_sha256=DECRYPT_SHA, source_sha256=before, native_archives_sha256=archive_hashes,
        rlibs=rlibs, binary_sha256=binary_hashes, synthetic_corpus_sha256=corpus_hashes,
        commands_sha256=sha(run / "commands.json"), process_results=process_results,
        network_policy="every_runtime_helper_strace_network_EPERM;Rust_compiler_local_IPC_audited_separately",
        network_traces=[dict(name=path.name, bytes=path.stat().st_size, sha256=sha(path)) for path in traces],
        compiler_ipc_traces=[dict(name=path.name, bytes=path.stat().st_size, sha256=sha(path)) for path in compiler_traces],
        guards=dict(sources_unchanged=True, native_source_unchanged=True, native_archives_unchanged=True,
                    rlibs_unchanged=True, binaries_unchanged=True, synthetic_inputs_unchanged=True,
                    network_not_attempted=True, independent_plaintext_bytes_equal=True),
        cases=records,
    )
    report_path.write_text(json.dumps(report, indent=2) + "\n")
    pointer.write_text(json.dumps(dict(report=str(report_path), sha256=sha(report_path), run=str(run))) + "\n")
    print(json.dumps(dict(status="passed", report=str(report_path), sha256=sha(report_path),
                         native_cases=len(cases), rust_cases_per_profile=len(objects), traces=len(traces))))


if __name__ == "__main__":
    main()
