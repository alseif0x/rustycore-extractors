// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later
// Offline replacement for CascLib's socket transport.

#define __CASCLIB_SELF__
#include "CascLib.h"
#include "CascCommon.h"
#include "common/Sockets.h"

PCASC_SOCKET sockets_connect(char const*, unsigned)
{
    SetCascError(ERROR_NOT_SUPPORTED);
    return nullptr;
}

void sockets_set_caching(bool)
{
}

char* CASC_SOCKET::ReadResponse(char const*, size_t, CASC_MIME_RESPONSE&)
{
    SetCascError(ERROR_NOT_SUPPORTED);
    return nullptr;
}

void CASC_SOCKET::Release()
{
}
