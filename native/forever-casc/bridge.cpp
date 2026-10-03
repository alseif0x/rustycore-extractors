// SPDX-License-Identifier: GPL-3.0-or-later
// Private C ABI v1. Owned storage+file; no C++ layouts/callbacks exposed to Rust.
#include "CascLib.h"
#include <cstdint>
#include <cstring>
#include <memory>

namespace {
struct ReadHandle {
    HANDLE storage = nullptr;
    HANDLE file = nullptr;
    ~ReadHandle() noexcept {
        if (file) CascCloseFile(file);
        if (storage) CascCloseStorage(storage);
    }
};
uint32_t Failure() noexcept {
    switch (GetCascError()) {
        case ERROR_FILE_ENCRYPTED: return 4;
        case ERROR_FILE_NOT_FOUND: return 5;
        case ERROR_FILE_INCOMPLETE: return 7;
        default: return 3;
    }
}
bool WINAPI Offline(void*, CASC_PROGRESS_MSG message, LPCSTR, DWORD, DWORD) {
    return message == CascProgressDownloadingFile || message == CascProgressDownloadingArchiveIndexes;
}
}

extern "C" uint32_t rce_casc_abi() noexcept { return 1; }
extern "C" uint32_t rce_casc_open(char const* path, uint32_t locale, uint32_t id,
    void** output, uint64_t* size) noexcept {
    if (!output || !size) return 1;
    *output = nullptr;
    *size = 0;
    if (!path || !*path || (locale != 2 && locale != 0x80) || id == 0 || id == UINT32_MAX) return 1;
    try {
        auto handle = std::make_unique<ReadHandle>();
        CASC_OPEN_STORAGE_ARGS args{};
        args.Size = sizeof(args);
        args.szLocalPath = path;
        args.szCodeName = "wow_classic_beta";
        args.dwLocaleMask = locale;
        args.PfnProgressCallback = Offline;
        if (!CascOpenStorageEx(nullptr, &args, false, &handle->storage)) return Failure();
        CASC_STORAGE_PRODUCT product{};
        if (!CascGetStorageInfo(handle->storage, CascStorageProduct, &product, sizeof(product), nullptr)) return Failure();
        if (product.BuildNumber != 70170 || std::memcmp(product.szCodeName, "wow_classic_beta\0", 17) != 0) return 2;
        // Locale belongs to storage root selection; CascOpenFile ignores its
        // locale parameter. STRICT is intentional; OVERCOME_ENCRYPTED is absent.
        if (!CascOpenFile(handle->storage, CASC_FILE_DATA_ID(id), 0,
            CASC_OPEN_BY_FILEID | CASC_STRICT_DATA_CHECK, &handle->file)) return Failure();
        ULONGLONG native_size = 0;
        if (!CascGetFileSize64(handle->file, &native_size)) return Failure();
        *size = native_size;
        *output = handle.release();
        return 0;
    } catch (...) { return 9; }
}
extern "C" uint32_t rce_casc_read(void* opaque, uint8_t* buffer, uint32_t length,
    uint32_t* read) noexcept {
    if (!opaque || !read || (!buffer && length)) return 1;
    *read = 0;
    try {
        auto handle = static_cast<ReadHandle*>(opaque);
        DWORD actual = 0;
        if (length && !CascReadFile(handle->file, buffer, length, &actual)) return Failure();
        *read = actual;
        return actual == length ? 0 : 7;
    } catch (...) { return 9; }
}
extern "C" void rce_casc_close(void* opaque) noexcept {
    delete static_cast<ReadHandle*>(opaque);
}
