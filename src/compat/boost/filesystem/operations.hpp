// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#pragma once

#include "path.hpp"

namespace boost::filesystem
{
using std::filesystem::canonical;
using std::filesystem::create_directories;
using std::filesystem::create_directory;
using std::filesystem::current_path;
using std::filesystem::exists;
using std::filesystem::file_size;
using std::filesystem::remove;
}
