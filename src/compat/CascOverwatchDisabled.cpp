// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later
// Fixed-product replacement for the excluded CascLib Overwatch parsers.

#define __CASCLIB_SELF__
#include "CascLib.h"
#include "CascCommon.h"

DWORD LoadApplicationPackageManifestFile(TCascStorage*, CASC_FILE_TREE&, PCASC_CKEY_ENTRY, char const*)
{
    return ERROR_NOT_SUPPORTED;
}

DWORD LoadContentManifestFile(TCascStorage*, CASC_FILE_TREE&, PCASC_CKEY_ENTRY, char const*)
{
    return ERROR_NOT_SUPPORTED;
}
