// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Original, deliberately incomplete QA compatibility type.
// This is not Boost code or a general Boost implementation. It only lets the
// discarded CLI paths of pinned System.cpp typecheck without installing Boost.
// The retained memory parser/converter must not reference these operations.
#pragma once

#include <filesystem>
#include <stdexcept>
#include <string>

namespace boost::filesystem {
[[noreturn]] inline void rce_unused_filesystem_operation() {
    throw std::logic_error("qa_discarded_filesystem_path_executed");
}

// A class, rather than an alias, is required by CascHandles.h's forward
// declaration. Construction/destruction and lexical operations perform no I/O.
class path {
public:
    path() = default;
    path(char const* value) : value_(value) {}
    path(std::string const& value) : value_(value) {}
    std::string string() const { return value_.string(); }
    path extension() const { return path(value_.extension().string()); }
    friend path operator/(path const& left, path const& right) {
        return path((left.value_ / right.value_).string());
    }
    friend bool operator==(path const& left, path const& right) {
        return left.value_ == right.value_;
    }
private:
    std::filesystem::path value_;
};
} // namespace boost::filesystem
