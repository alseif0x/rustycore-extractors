// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later
// OpenSSL 3 EVP adapter replacing CascLib's bundled MD5 implementation.

#pragma once

struct evp_md_ctx_st;

struct MD5_CTX
{
    evp_md_ctx_st* context;
};

void MD5_Init(MD5_CTX* context);
void MD5_Update(MD5_CTX* context, void const* data, unsigned long size);
void MD5_Final(unsigned char* result, MD5_CTX* context);
