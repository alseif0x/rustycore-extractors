// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later
// Narrow source-compatible adapter for the extractor-used Boost.Filesystem API.

#pragma once

#include <filesystem>
#include <system_error>

namespace boost
{
namespace system
{
using error_code = std::error_code;
}

namespace filesystem
{
using path = std::filesystem::path;
using directory_iterator = std::filesystem::directory_iterator;
using filesystem_error = std::filesystem::filesystem_error;
}
}
