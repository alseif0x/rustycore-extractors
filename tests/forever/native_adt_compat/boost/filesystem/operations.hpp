// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11. Original QA declarations; no Boost implementation copied.
// Every filesystem operation fails if executed. Only discarded original CLI
// functions may reference them. The ADT conversion uses ordinary stdio instead.
#pragma once

#include "path.hpp"

namespace boost::system {
class error_code {
public:
    explicit operator bool() const noexcept { return false; }
};
} // namespace boost::system

namespace boost::filesystem {
inline bool exists(path const&) { rce_unused_filesystem_operation(); }
inline bool create_directory(path const&, boost::system::error_code&) {
    rce_unused_filesystem_operation();
}
inline bool remove(path const&) { rce_unused_filesystem_operation(); }
inline path canonical(path const&) { rce_unused_filesystem_operation(); }
inline path current_path() { rce_unused_filesystem_operation(); }
} // namespace boost::filesystem
