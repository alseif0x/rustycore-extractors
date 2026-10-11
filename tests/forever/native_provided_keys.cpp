// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: synthetic-only adapter to original pinned CascDecrypt.
// No storage opening, key discovery/loading, builtin-key initialization or I/O
// beyond the explicit authored fixture and binary result stream.
#include "CascLib.h"
#include "CascCommon.h"
#include <algorithm>
#include <array>
#include <cstdint>
#include <cstdio>
#include <cstring>
#include <fstream>
#include <iterator>
#include <vector>

namespace {
struct Input {
    std::vector<unsigned char> bytes;
    std::size_t at = 0;
    std::uint64_t number(unsigned n) {
        if (n > 8 || at > bytes.size() || n > bytes.size() - at) throw 1;
        std::uint64_t result = 0;
        for (unsigned i = 0; i < n; ++i) result |= std::uint64_t(bytes[at++]) << (i * 8);
        return result;
    }
    std::vector<unsigned char> take(std::size_t n) {
        if (at > bytes.size() || n > bytes.size() - at) throw 1;
        std::vector<unsigned char> result(bytes.begin() + at, bytes.begin() + at + n);
        at += n;
        return result;
    }
};
void emit(std::uint64_t value, unsigned size) {
    for (unsigned i = 0; i < size; ++i)
        if (std::putchar((value >> (8 * i)) & 255) == EOF) throw 2;
}
void emit(std::vector<unsigned char> const& bytes) {
    emit(bytes.size(), 4);
    if (std::fwrite(bytes.data(), 1, bytes.size(), stdout) != bytes.size()) throw 2;
}
void append(std::vector<unsigned char>& output, std::uint64_t value, unsigned size) {
    for (unsigned i = 0; i < size; ++i) output.push_back((value >> (8 * i)) & 255);
}
}
int main(int argc, char** argv) try {
    if (argc != 2) return 1;
    std::ifstream file(argv[1], std::ios::binary);
    if (!file) return 1;
    Input input{{std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>()}};
    if (input.bytes.size() < 12 || input.bytes.size() > 64 * 1024 * 1024 ||
        std::memcmp(input.bytes.data(), "RCEKEY01", 8)) return 1;
    input.at = 8;
    auto count = input.number(4);
    if (!count || count > 512) return 1;
    if (std::fwrite("RCEKOUT1", 1, 8, stdout) != 8) return 2;
    emit(count, 4);
    for (std::size_t i = 0; i < count; ++i) {
        auto name = input.number(8);
        auto key = input.take(16);
        auto name_size = input.number(1), nonce_size = input.number(1);
        if ((name_size != 0 && name_size != 8) || (nonce_size != 4 && nonce_size != 8) ||
            (!name_size && name)) return 1;
        auto nonce = input.take(nonce_size);
        auto index = input.number(4), length = input.number(4);
        if (!length || length > 1024 * 1024) return 1;
        auto inner = input.take(length);
        if (inner[0] != 'N' && inner[0] != 'Z') return 1;
        std::vector<unsigned char> header;
        append(header, name_size, 1);
        append(header, name, name_size);
        append(header, nonce_size, 1);
        header.insert(header.end(), nonce.begin(), nonce.end());
        header.push_back('S');
        auto encoded = header;
        encoded.insert(encoded.end(), inner.begin(), inner.end());
        TCascStorage storage;
        if (storage.KeyMap.FindKey(name) != nullptr || !storage.KeyMap.AddKey(name, key.data())) return 3;
        // CascDecrypt is symmetric XOR. Only supplied synthetic keys are added.
        std::vector<unsigned char> ciphertext(inner.size());
        DWORD size = DWORD(ciphertext.size());
        if (CascDecrypt(&storage, ciphertext.data(), &size, encoded.data(), DWORD(encoded.size()), DWORD(index)) ||
            size != inner.size()) return 3;
        encoded = header;
        encoded.insert(encoded.end(), ciphertext.begin(), ciphertext.end());
        std::vector<unsigned char> decoded(inner.size());
        size = DWORD(decoded.size());
        if (CascDecrypt(&storage, decoded.data(), &size, encoded.data(), DWORD(encoded.size()), DWORD(index)) ||
            size != inner.size() || decoded != inner) return 3;
        encoded.insert(encoded.begin(), 'E');
        emit(encoded);
        emit(decoded);
    }
    if (input.at != input.bytes.size()) return 1;
    return std::fflush(stdout) ? 2 : 0;
} catch (...) { return 4; }
