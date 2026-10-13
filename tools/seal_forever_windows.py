# SPDX-License-Identifier: GPL-3.0-or-later
# Modified 2026-10-13: native input identity and explicit observed MSVC v2 profile.
"""Original 2026-10-11 bounded offline unsigned PE32+ producer sealer.

Pure bytes core and explicit offline CLI with bounded regular-file I/O.
The CLI is not a publication/recovery protocol or a loaded-memory measurement.
The digest is an unsigned binding to a loaded literal, not publisher authenticity.
"""
from dataclasses import dataclass
import hashlib

MARKER = b"RCE-FOREVER-PE-SEAL-V1".ljust(32, b"\0")
MARKER_V2 = b"RCE-FOREVER-PE-SEAL-V2".ljust(32, b"\0")
PROFILE_V1 = "forever-windows-producer-seal-v1"
PROFILE_V2 = "forever-windows-producer-seal-msvc-v2"
SECTION = b".rceid\0\0"
PAYLOAD = 64
MAX_FILE = 256 * 1024 * 1024
MAX_WORK = 65536


def profile_marker(profile):
    require(profile in (PROFILE_V1, PROFILE_V2), "pe_profile")
    return MARKER if profile == PROFILE_V1 else MARKER_V2


class Reject(ValueError):
    pass


def require(value, code):
    if not value:
        raise Reject(code)


def integer(data, offset, width):
    require(0 <= offset <= len(data) - width, "pe_bounds")
    return int.from_bytes(data[offset:offset + width], "little")


def aligned(value, alignment):
    return (value + alignment - 1) // alignment * alignment


@dataclass(frozen=True)
class Section:
    name: bytes
    rva: int
    virtual: int
    raw: int
    size: int
    flags: int


@dataclass(frozen=True)
class Layout:
    slot: int
    slot_rva: int
    checksum: int
    section: Section


def locate(data, profile=PROFILE_V1):
    marker = profile_marker(profile)
    require(type(data) is bytes and 64 <= len(data) <= MAX_FILE, "pe_limit")
    require(data[:2] == b"MZ", "pe_dos")
    pe = integer(data, 0x3c, 4)
    require(pe >= 64 and data[pe:pe + 4] == b"PE\0\0", "pe_signature")
    require(integer(data, pe + 4, 2) == 0x8664, "pe_machine")
    count = integer(data, pe + 6, 2)
    require(1 <= count <= 96, "pe_sections")
    require(integer(data, pe + 20, 2) == 240, "pe_optional")
    flags = integer(data, pe + 22, 2)
    require(flags & 2 and not flags & (0x100 | 0x1000 | 0x2000), "pe_executable")
    optional = pe + 24
    require(integer(data, optional, 2) == 0x20b, "pe_magic")
    require(integer(data, optional + 108, 4) == 16, "pe_directories")
    require(integer(data, optional + 68, 2) == 3, "pe_console_only")
    checksum = optional + 64
    require(integer(data, checksum, 4) == 0, "pe_checksum_nonzero")
    image_base = integer(data, optional + 24, 8)
    sa = integer(data, optional + 32, 4)
    fa = integer(data, optional + 36, 4)
    require(sa >= 4096 and sa <= 65536 and not sa & (sa - 1), "pe_alignment")
    require(512 <= fa <= sa and not fa & (fa - 1), "pe_alignment")
    image_size = integer(data, optional + 56, 4)
    headers = integer(data, optional + 60, 4)
    table = optional + 240
    require(table + count * 40 <= headers <= len(data) and headers % fa == 0,
            "pe_headers")
    require(image_size <= MAX_FILE and image_size % sa == 0, "pe_image_size")
    sections = []
    raw_end = headers
    virtual_end = aligned(headers, sa)
    for index in range(count):
        p = table + index * 40
        name = data[p:p + 8]
        virtual, rva, size, raw = (integer(data, p + i, 4) for i in (8, 12, 16, 20))
        require(data[p + 24:p + 36] == bytes(12), "pe_coff_relocations")
        flags = integer(data, p + 36, 4)
        require(virtual > 0 and rva >= virtual_end and rva % sa == 0,
                "pe_virtual_overlap")
        virtual_end = aligned(rva + max(virtual, size), sa)
        require(virtual_end <= image_size, "pe_virtual_bounds")
        if size:
            require(size % fa == 0 and raw % fa == 0 and raw >= raw_end,
                    "pe_raw_overlap")
            require(raw + size <= len(data), "pe_raw_bounds")
            raw_end = raw + size
        else:
            require(raw == 0, "pe_raw_zero")
        sections.append(Section(name, rva, virtual, raw, size, flags))
    sealed = [s for s in sections if s.name == SECTION]
    require(len(sealed) == 1, "pe_seal_count")
    seal = sealed[0]
    require(seal.virtual == PAYLOAD and seal.size >= PAYLOAD and seal.raw != 0,
            "pe_seal_initialized")
    require(seal.flags == 0x40000040, "pe_seal_flags")
    require(data[seal.raw:seal.raw + 32] == marker, "pe_seal_marker")
    require(data[seal.raw + PAYLOAD:seal.raw + seal.size] == bytes(seal.size - PAYLOAD),
            "pe_seal_padding")

    def mapped(rva, size, raw=False):
        require(rva > 0 and size > 0, "pe_rva")
        matches = [s for s in sections if s.rva <= rva and
                   rva + size <= s.rva + (min(s.virtual, s.size) if raw else s.virtual)]
        require(len(matches) == 1, "pe_rva_bounds")
        return matches[0]

    def offset(rva, size):
        s = mapped(rva, size, raw=True)
        return s.raw + rva - s.rva

    def not_seal(rva, size):
        require(rva + size <= seal.rva or rva >= seal.rva + seal.virtual,
                "pe_loader_writes_seal")

    def not_control(rva, size, control):
        start, length = control
        require(not start or rva + size <= start or rva >= start + length,
                "pe_loader_control_write")

    directories = []
    for index in range(16):
        p = optional + 112 + index * 8
        rva, size = integer(data, p, 4), integer(data, p + 4, 4)
        require(bool(rva) == bool(size), "pe_directory_pair")
        # Unsupported loader structures reject rather than relying on absence
        # of an obvious section-range alias. Future MSVC CFG needs own review.
        require(not (rva and (index in (4, 7, 8, 11, 13, 14, 15) or
                             (index == 10 and profile == PROFILE_V1))),
                "pe_unsupported_directory")
        if rva:
            offset(rva, size)
            not_seal(rva, size)
        directories.append((rva, size))

    if profile == PROFILE_V2:
        _validate_msvc_v2(data, image_base, integer(data, optional + 70, 2),
                          directories, mapped, offset, not_seal)
        return Layout(seal.raw + 32, seal.rva + 32, checksum, seal)

    reloc, reloc_size = directories[5]
    if reloc:
        require(reloc_size <= 1024 * 1024, "pe_relocation_limit")
        p = offset(reloc, reloc_size)
        end = p + reloc_size
        work = 0
        while p < end:
            require(p + 8 <= end, "pe_relocation_block")
            page, length = integer(data, p, 4), integer(data, p + 4, 4)
            require(page % 4096 == 0 and length >= 8 and length % 2 == 0 and
                    p + length <= end, "pe_relocation_block")
            for entry_at in range(p + 8, p + length, 2):
                work += 1
                require(work <= MAX_WORK, "pe_relocation_limit")
                entry = integer(data, entry_at, 2)
                kind, low = entry >> 12, entry & 4095
                if kind:
                    require(kind == 10, "pe_relocation_kind")
                    mapped(page + low, 8)
                    not_seal(page + low, 8)
                    not_control(page + low, 8, directories[5])
                    not_control(page + low, 8, directories[1])
            p += length

    imports, imports_size = directories[1]
    if imports:
        require(imports_size <= 1024 * 1024, "pe_import_limit")
        p = offset(imports, imports_size)
        end = p + imports_size
        work = 0
        while True:
            require(p + 20 <= end, "pe_import_descriptor")
            descriptor = data[p:p + 20]
            if not any(descriptor):
                break
            lookup = integer(data, p, 4) or integer(data, p + 16, 4)
            target = integer(data, p + 16, 4)
            require(lookup and target, "pe_import_descriptor")
            index = 0
            while True:
                work += 1
                require(work <= MAX_WORK, "pe_import_limit")
                thunk = integer(data, offset(lookup + index * 8, 8), 8)
                mapped(target + index * 8, 8)
                not_seal(target + index * 8, 8)
                not_control(target + index * 8, 8, directories[5])
                not_control(target + index * 8, 8, directories[1])
                if thunk == 0:
                    break
                index += 1
            p += 20

    tls, tls_size = directories[9]
    if tls:
        require(tls_size == 40, "pe_tls_size")
        p = offset(tls, 40)
        index_va = integer(data, p + 16, 8)
        require(index_va >= image_base, "pe_tls_index")
        mapped(index_va - image_base, 4)
        not_seal(index_va - image_base, 4)
        callbacks = integer(data, p + 24, 8)
        if callbacks:
            require(callbacks >= image_base, "pe_tls_callbacks")
            for index in range(1024):
                callback = integer(data, offset(callbacks - image_base + index * 8, 8), 8)
                if callback == 0:
                    break
                require(callback >= image_base, "pe_tls_callbacks")
                require(mapped(callback - image_base, 1).flags & 0x20000000,
                        "pe_tls_callback_code")
            else:
                raise Reject("pe_tls_callbacks_limit")
    return Layout(seal.raw + 32, seal.rva + 32, checksum, seal)


def _validate_msvc_v2(data, base, dll_flags, directories, mapped, offset, not_seal):
    """Closed observed MSVC shape, not a general load-config or loader model.

    0x80038003 is an opaque observed signature, not an ABI version. The native
    caller separately requires an AMD64 host without emulation. No image bytes
    are normalized here. This checks declared relocations/import/TLS-index and
    conservative cookie/guard-slot writes, not arbitrary startup code or every
    Windows/CRT write. Import names and TLS template contents are not parsed.
    Work is bounded by MAX_WORK relocation/import entries, 1024 callbacks and
    MAX_WORK + 32 control spans; heap storage is proportional to those limits.
    """
    from bisect import bisect_left, bisect_right

    def overlaps(a, b):
        return a[0] < b[0] + b[1] and b[0] < a[0] + a[1]

    def va_rva(va):
        require(base <= va and 0 < va - base <= 0xffffffff, "pe_v2_va")
        return va - base

    def read_only(rva, size):
        s = mapped(rva, size, raw=True)
        require(s.name == b'.rdata\0\0' and s.flags == 0x40000040, "pe_v2_read_only")

    def code(rva, size):
        s = mapped(rva, size, raw=True)
        require(s.name == b'.text\0\0\0' and s.flags == 0x60000020, "pe_v2_code")

    def disjoint(spans, error):
        ordered = sorted(spans)
        for previous, current in zip(ordered, ordered[1:]):
            require(not overlaps(previous, current), error)
        return ordered

    def cross_disjoint(a, b):
        i = j = 0
        while i < len(a) and j < len(b):
            require(not overlaps(a[i], b[j]), "pe_v2_write_alias")
            if a[i][0] + a[i][1] <= b[j][0]:
                i += 1
            else:
                j += 1

    require(dll_flags == 0x8160, "pe_v2_flags")
    require(base > 0 and base + MAX_FILE <= 0xffffffffffffffff, "pe_v2_va")
    config = directories[10]
    require(config[0] and config[1] == 320, "pe_v2_load_config")
    read_only(*config)
    p = offset(*config)
    fields = (88, 112, 120, 256, 280, 288, 296, 304, 312)
    mask = bytearray(320)
    for start, size in ((0, 4), (144, 4), *((field, 8) for field in fields)):
        mask[start:start + size] = bytes([1]) * size
    require(integer(data, p, 4) == 320 and integer(data, p + 144, 4) == 0x100 and
            all(mask[i] or data[p + i] == 0 for i in range(320)), "pe_v2_load_config")
    pointers = [va_rva(integer(data, p + field, 8)) for field in fields]
    controls = [span for i, span in enumerate(directories) if span[0] and i != 12]
    # owner is the one control structure in which a typed VA fixup is allowed.
    expected = [(config[0] + field, config) for field in fields]
    cookie = (pointers[0], 8)
    s = mapped(*cookie, raw=True)
    require(s.name == b'.data\0\0\0' and s.flags == 0xc0000040 and
            cookie[0] % 8 == 0 and integer(data, offset(*cookie), 8) == 0x2b992ddfa232,
            "pe_v2_cookie")
    slots = []
    for field, rva in zip(fields, pointers):
        if field in (88, 256):
            continue
        read_only(rva, 8)
        require(rva % 8 == 0, "pe_v2_slot")
        initial = integer(data, offset(rva, 8), 8)
        if field == 304:
            require(initial == 0, "pe_v2_slot")
        else:
            code(va_rva(initial), 1)
            expected.append((rva, None))
        slots.append((rva, 8))
    destinations = disjoint([cookie, *slots], "pe_v2_write_alias")

    volatile = pointers[3]
    read_only(volatile, 88)
    q = offset(volatile, 88)
    require(volatile % 4 == 0 and
            [integer(data, q + i * 4, 4) for i in range(6)] ==
            [24, 0x80038003, volatile + 24, 40, volatile + 64, 24], "pe_v2_volatile")
    controls.append((volatile, 88))
    accesses = [integer(data, q + 24 + i * 4, 4) for i in range(10)]
    ranges = [(integer(data, q + 64 + i * 8, 4), integer(data, q + 68 + i * 8, 4))
              for i in range(3)]
    require(all(a < b for a, b in zip(accesses, accesses[1:])), "pe_v2_volatile")
    require(all(a[0] + a[1] <= b[0] for a, b in zip(ranges, ranges[1:])), "pe_v2_volatile")
    for span in ranges:
        code(*span)
    for access in accesses:
        code(access, 1)
        require(any(start <= access < start + size for start, size in ranges), "pe_v2_volatile")

    import_writes = []
    imports, imports_size = directories[1]
    if imports:
        require(imports_size <= 1024 * 1024, "pe_import_limit")
        p = offset(imports, imports_size)
        end = p + imports_size
        work = 0
        while True:
            require(p + 20 <= end, "pe_import_descriptor")
            if not any(data[p:p + 20]):
                break
            target = integer(data, p + 16, 4)
            lookup = integer(data, p, 4) or target
            require(lookup and target, "pe_import_descriptor")
            count = 0
            while True:
                work += 1
                require(work <= MAX_WORK, "pe_import_limit")
                thunk = integer(data, offset(lookup + count * 8, 8), 8)
                mapped(target + count * 8, 8)
                not_seal(target + count * 8, 8)
                import_writes.append((target + count * 8, 8))
                count += 1
                if not thunk:
                    break
            controls.append((lookup, count * 8))
            p += 20

    tls_write = []
    tls = directories[9]
    if tls[0]:
        require(tls[1] == 40, "pe_tls_size")
        p = offset(*tls)
        values = [integer(data, p + i * 8, 8) for i in range(4)]
        start, end, index_va, callbacks_va = values
        require(bool(start) == bool(end) and end >= start, "pe_v2_tls_template")
        if start:
            start_rva, end_rva = va_rva(start), va_rva(end)
            if start_rva != end_rva:
                offset(start_rva, end_rva - start_rva)
                not_seal(start_rva, end_rva - start_rva)
            else:
                offset(start_rva, 1)
        index = va_rva(index_va)
        mapped(index, 4)
        not_seal(index, 4)
        tls_write = [(index, 4)]
        expected.extend((tls[0] + i * 8, tls) for i, value in enumerate(values) if value)
        if callbacks_va:
            callbacks = va_rva(callbacks_va)
            count = 0
            for i in range(1024):
                value = integer(data, offset(callbacks + i * 8, 8), 8)
                if not value:
                    break
                code(va_rva(value), 1)
                count += 1
            else:
                raise Reject("pe_tls_callbacks_limit")
            array = (callbacks, (count + 1) * 8)
            controls.append(array)
            expected.extend((callbacks + i * 8, array) for i in range(count))

    require(len(controls) <= MAX_WORK + 32, "pe_v2_control_limit")
    controls = disjoint(controls, "pe_v2_control_alias")
    control_ends = [start + size for start, size in controls]

    def control_at(span):
        i = bisect_right(control_ends, span[0])
        return controls[i] if i < len(controls) and overlaps(span, controls[i]) else None

    for destination in destinations:
        not_seal(*destination)
        require(control_at(destination) is None and
                all(not overlaps(destination, directory) for directory in directories if directory[0]),
                "pe_loader_control_write")
    expected.sort(key=lambda item: item[0])
    starts = [start for start, _ in expected]
    require(all(a + 8 <= b for a, b in zip(starts, starts[1:])), "pe_v2_write_alias")
    for target, owner in expected:
        require(control_at((target, 8)) == owner, "pe_loader_control_write")

    reloc_writes = []
    seen = [False] * len(expected)
    reloc, reloc_size = directories[5]
    if reloc:
        require(reloc_size <= 1024 * 1024, "pe_relocation_limit")
        p = offset(reloc, reloc_size)
        end = p + reloc_size
        work = 0
        while p < end:
            require(p + 8 <= end, "pe_relocation_block")
            page, length = integer(data, p, 4), integer(data, p + 4, 4)
            require(page % 4096 == 0 and length >= 8 and length % 2 == 0 and
                    p + length <= end, "pe_relocation_block")
            for at in range(p + 8, p + length, 2):
                work += 1
                require(work <= MAX_WORK, "pe_relocation_limit")
                entry = integer(data, at, 2)
                kind = entry >> 12
                if not kind:
                    continue
                require(kind == 10, "pe_relocation_kind")
                target = page + (entry & 4095)
                mapped(target, 8)
                not_seal(target, 8)
                span = (target, 8)
                i = bisect_left(starts, target)
                typed = i < len(starts) and starts[i] == target
                if typed:
                    require(not seen[i], "pe_v2_fixup_count")
                    seen[i] = True
                else:
                    require(control_at(span) is None, "pe_loader_control_write")
                require(all(not overlaps(span, destination) or
                            (typed and span == destination and destination in slots)
                            for destination in destinations), "pe_v2_write_alias")
                reloc_writes.append(span)
            p += length
    require(all(seen), "pe_v2_fixup_count")
    reloc_writes = disjoint(reloc_writes, "pe_v2_write_alias")
    import_writes = disjoint(import_writes, "pe_v2_write_alias")
    for span in [*import_writes, *tls_write]:
        require(control_at(span) is None, "pe_loader_control_write")
        require(all(not overlaps(span, destination) for destination in destinations), "pe_v2_write_alias")
    cross_disjoint(reloc_writes, import_writes)
    cross_disjoint(reloc_writes, tls_write)
    cross_disjoint(import_writes, tls_write)


def canonical_digest(data, layout):
    digest = hashlib.sha256()
    view = memoryview(data)
    digest.update(view[:layout.slot])
    digest.update(bytes(32))
    digest.update(view[layout.slot + 32:])
    return digest.digest()


def seal(data, profile=PROFILE_V1):
    """Only seal a zero-slot, zero-checksum, unsigned bounded layout."""
    layout = locate(data, profile)
    require(data[layout.slot:layout.slot + 32] == bytes(32), "pe_already_sealed")
    digest = canonical_digest(data, layout)
    require(any(digest), "pe_zero_digest")
    result = data[:layout.slot] + digest + data[layout.slot + 32:]
    require(verify(result, profile_marker(profile) + digest, layout.section.rva, profile) ==
            hashlib.sha256(result).digest(), "pe_self_check")
    return result


def prepare_unsigned(data, profile=PROFILE_V1):
    """Explicit artifact transformation: zero PE checksum before sealing.

    Not an Authenticode operation. A certificate table still rejects; no checksum
    or other field may be recomputed after the digest slot has been patched.
    """
    require(type(data) is bytes and len(data) <= MAX_FILE, "pe_limit")
    pe = integer(data, 0x3c, 4)
    require(pe >= 64 and data[pe:pe + 4] == b"PE\0\0", "pe_signature")
    checksum = pe + 24 + 64
    integer(data, checksum, 4)
    result = data[:checksum] + bytes(4) + data[checksum + 4:]
    locate(result, profile)
    return result


def verify(data, loaded_payload, measured_payload_rva, profile=PROFILE_V1):
    """Caller must supply a payload independently read from the actual module.

    Synthetic tests supply bytes, which is not runtime evidence. No pathname or
    disk slot may be used as a replacement source of loaded_payload.
    """
    require(type(loaded_payload) is bytes and len(loaded_payload) == PAYLOAD,
            "pe_loaded_payload")
    require(loaded_payload[:32] == profile_marker(profile) and any(loaded_payload[32:]), "pe_loaded_seal")
    layout = locate(data, profile)
    require(type(measured_payload_rva) is int and measured_payload_rva == layout.section.rva, "pe_loaded_rva")
    stored = data[layout.slot:layout.slot + 32]
    require(stored == loaded_payload[32:], "pe_loaded_mismatch")
    require(canonical_digest(data, layout) == stored, "pe_digest_mismatch")
    return hashlib.sha256(data).digest()


@dataclass(frozen=True)
class _WindowsInputIdentity:
    file_type: int
    attributes: int
    volume: int
    index: int
    size: int
    creation_time: int
    write_time: int
    links: int


class _WindowsInputAPI:
    """Native snapshots avoid CPython's pathname execute hint and stat fallback.

    CPython 3.11 lstat adds 0111 for .exe paths, unlike fstat. Its share-zero
    pathname open can also fall back to directory data without file IDs while
    the reader is open. Query both bindings through compatible Win32 handles.
    This is observed input identity, not an anchored ancestor or NTFS profile.
    """
    def __init__(self):
        import ctypes
        self.ctypes = ctypes
        dword = ctypes.c_uint32

        class Information(ctypes.Structure):
            _fields_ = [(name, dword) for name in (
                'attributes', 'creation_low', 'creation_high',
                'access_low', 'access_high', 'write_low', 'write_high',
                'volume', 'size_high', 'size_low', 'links', 'index_high', 'index_low')]

        require(ctypes.sizeof(Information) == 52, "pe_input_layout")
        self.Information = Information
        native = ctypes.WinDLL('kernel32', use_last_error=True)
        self.open = native.CreateFileW
        self.open.argtypes = [ctypes.c_wchar_p, dword, dword, ctypes.c_void_p,
                              dword, dword, ctypes.c_void_p]
        self.open.restype = ctypes.c_void_p
        self.close = native.CloseHandle
        self.close.argtypes = [ctypes.c_void_p]
        self.close.restype = ctypes.c_int
        self.query = native.GetFileInformationByHandle
        self.query.argtypes = [ctypes.c_void_p, ctypes.POINTER(Information)]
        self.query.restype = ctypes.c_int
        self.kind = native.GetFileType
        self.kind.argtypes = [ctypes.c_void_p]
        self.kind.restype = dword

    def snapshot(self, handle):
        kind = self.kind(handle)
        require(kind == 1, "pe_input_type")  # FILE_TYPE_DISK
        info = self.Information()
        if not self.query(handle, self.ctypes.byref(info)):
            raise OSError("native input identity query failed")
        return _WindowsInputIdentity(
            kind, info.attributes, info.volume,
            (info.index_high << 32) | info.index_low,
            (info.size_high << 32) | info.size_low,
            (info.creation_high << 32) | info.creation_low,
            (info.write_high << 32) | info.write_low, info.links)

    def named(self, path):
        import os
        # FILE_READ_ATTRIBUTES; share READ|WRITE|DELETE; OPEN_EXISTING;
        # BACKUP_SEMANTICS|OPEN_REPARSE_POINT. Never traverse a leaf reparse.
        handle = self.open(os.fsdecode(path), 0x80, 7, None, 3, 0x02200000, None)
        if handle == self.ctypes.c_void_p(-1).value:
            raise OSError("native input pathname open failed")
        try:
            return self.snapshot(handle)
        finally:
            if not self.close(handle):
                raise OSError("native input pathname close failed")

    def held(self, descriptor):
        import msvcrt
        return self.snapshot(msvcrt.get_osfhandle(descriptor))


def _read_file_windows(path):
    import os
    native = _WindowsInputAPI()
    before = native.named(path)
    require(before.file_type == 1 and not before.attributes & (0x10 | 0x400),
            "pe_input_type")  # Directory or any reparse point rejects.
    require(before.size <= MAX_FILE, "pe_limit")
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0)
    with os.fdopen(os.open(path, flags), "rb") as stream:
        held = native.held(stream.fileno())
        require(before == held, "pe_input_changed")
        data = stream.read(held.size + 1)
        require(len(data) <= MAX_FILE, "pe_limit")
        require(len(data) == held.size and held == native.held(stream.fileno())
                and before == native.named(path), "pe_input_changed")
    return data


def _read_file(path):
    """Bounded regular input; reject observed replacement/change, no discovery."""
    import os
    if os.name == 'nt':
        return _read_file_windows(path)
    import stat
    before = os.lstat(path)
    require(stat.S_ISREG(before.st_mode), "pe_input_type")
    require(before.st_size <= MAX_FILE, "pe_limit")
    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0) | getattr(os, "O_NOFOLLOW", 0) | getattr(os, "O_NONBLOCK", 0)
    with os.fdopen(os.open(path, flags), "rb") as stream:
        held = os.fstat(stream.fileno())
        def identity(value):
            return (value.st_dev, value.st_ino, value.st_mode, value.st_size,
                    value.st_mtime_ns, value.st_ctime_ns)
        require(stat.S_ISREG(held.st_mode) and identity(before) == identity(held), "pe_input_changed")
        data = stream.read(held.st_size + 1)
        require(len(data) <= MAX_FILE, "pe_limit")
        require(len(data) == held.st_size and identity(held) == identity(os.fstat(stream.fileno()))
                and identity(held) == identity(os.lstat(path)), "pe_input_changed")
    return data


def main(argv=None):
    """Offline, explicit-path CLI. Output is exclusively created, never replaced.

    Failed outputs are retained for inspection; a pathname is never unlinked on
    error because another actor could have replaced it. File fsync does not
    establish directory durability or a Windows publication/recovery protocol.

    This tool does not supply loaded-module evidence. Verification requires an
    independently measured payload/RVA; passing disk-derived values only checks
    the artifact's self-consistency, not the identity of a running process.
    """
    import argparse
    import json
    import os
    parser = argparse.ArgumentParser(description="Offline unsigned Forever Windows producer seal; default v1, explicit observed MSVC v2")
    commands = parser.add_subparsers(dest="command", required=True)
    sealing = commands.add_parser("seal")
    sealing.add_argument("--input", required=True)
    sealing.add_argument("--output", required=True)
    sealing.add_argument("--zero-checksum", action="store_true",
                         help="explicitly zero the PE checksum before sealing; no other transformation")
    sealing.add_argument("--profile", choices=(PROFILE_V1, PROFILE_V2), default=PROFILE_V1)
    checking = commands.add_parser("verify")
    checking.add_argument("--input", required=True)
    checking.add_argument("--loaded-payload", required=True, help="128 hex characters measured from the loaded module")
    checking.add_argument("--payload-rva", required=True, type=lambda value: int(value, 0))
    checking.add_argument("--profile", choices=(PROFILE_V1, PROFILE_V2), default=PROFILE_V1)
    args = parser.parse_args(argv)
    try:
        data = _read_file(args.input)
        if args.command == "seal":
            if args.zero_checksum:
                data = prepare_unsigned(data, args.profile)
            result = seal(data, args.profile)
            layout = locate(result, args.profile)
            flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_BINARY", 0)
            descriptor = os.open(args.output, flags, 0o600)
            with os.fdopen(descriptor, "wb") as stream:
                stream.write(result)
                stream.flush()
                os.fsync(stream.fileno())
            full_hash = hashlib.sha256(result).digest()
        else:
            require(len(args.loaded_payload) == 128, "pe_loaded_payload")
            try:
                loaded = bytes.fromhex(args.loaded_payload)
            except ValueError as error:
                raise Reject("pe_loaded_payload") from error
            full_hash = verify(data, loaded, args.payload_rva, args.profile)
            layout = locate(data, args.profile)
        print(json.dumps({"contract": args.profile, "status": "accepted",
                          "operation": args.command, "file_sha256": full_hash.hex(),
                          "payload_rva": layout.section.rva, "publisher_authenticated": False,
                          "scope": "artifact_integrity_with_supplied_seal" if args.command == "verify" else "offline_artifact_sealing"}, sort_keys=True))
        return 0
    except (Reject, OSError) as error:
        code = str(error) if isinstance(error, Reject) else "pe_file_io"
        print(json.dumps({"contract": args.profile, "status": "failed", "code": code}, sort_keys=True))
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
