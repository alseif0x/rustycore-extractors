// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later
// OpenSSL 3 EVP adapter replacing the ambiguously licensed CascLib SHA-1 file.

#pragma once

typedef unsigned int u32;
struct evp_md_ctx_st;

struct SHA1_CTX
{
    evp_md_ctx_st* context;
};

void SHA1_Init(SHA1_CTX* context);
void SHA1_Update(SHA1_CTX* context, void const* data, u32 size);
void SHA1_Final(SHA1_CTX* context, unsigned char result[20]);
