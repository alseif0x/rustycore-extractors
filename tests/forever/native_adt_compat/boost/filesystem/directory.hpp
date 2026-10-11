// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Original dead-path QA declarations, not Boost source.
#pragma once

#include "path.hpp"

namespace boost::filesystem {
class directory_entry {
public:
    filesystem::path const& path() const { rce_unused_filesystem_operation(); }
};

class directory_iterator {
public:
    directory_iterator() = default;
    explicit directory_iterator(filesystem::path const&) {
        rce_unused_filesystem_operation();
    }
    directory_iterator& operator++() { rce_unused_filesystem_operation(); }
    directory_entry const* operator->() const { rce_unused_filesystem_operation(); }
    friend bool operator!=(directory_iterator const&, directory_iterator const&) {
        rce_unused_filesystem_operation();
    }
};
} // namespace boost::filesystem
