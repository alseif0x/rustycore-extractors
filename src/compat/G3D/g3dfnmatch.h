// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later
// Portable replacement for the excluded BSD-4-Clause G3D fnmatch files.

#pragma once

#ifndef FNM_NOMATCH
#define FNM_NOMATCH 1
#define FNM_NOESCAPE 0x01
#define FNM_PATHNAME 0x02
#define FNM_PERIOD 0x04
#define FNM_LEADING_DIR 0x08
#define FNM_CASEFOLD 0x10
#define FNM_PREFIX_DIRS 0x20
#endif

namespace G3D
{
int g3dfnmatch(char const* pattern, char const* string, int flags);
}
