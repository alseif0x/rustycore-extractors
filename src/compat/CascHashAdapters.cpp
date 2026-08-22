// Copyright (C) 2026 rustycore-extractors contributors
// SPDX-License-Identifier: GPL-3.0-or-later

#include "hashes/md5.h"
#include "hashes/sha1.h"
#include <openssl/evp.h>
#include <cstring>

namespace
{
evp_md_ctx_st* CreateContext(EVP_MD const* algorithm)
{
    EVP_MD_CTX* context = EVP_MD_CTX_new();
    if (!context || EVP_DigestInit_ex(context, algorithm, nullptr) != 1)
    {
        EVP_MD_CTX_free(context);
        return nullptr;
    }
    return context;
}

void Update(evp_md_ctx_st* context, void const* data, size_t size)
{
    if (context)
        EVP_DigestUpdate(context, data, size);
}

void Final(evp_md_ctx_st*& context, unsigned char* result, unsigned int expectedSize)
{
    unsigned int actualSize = 0;
    if (!context || EVP_DigestFinal_ex(context, result, &actualSize) != 1 || actualSize != expectedSize)
        std::memset(result, 0, expectedSize);
    EVP_MD_CTX_free(context);
    context = nullptr;
}
}

void MD5_Init(MD5_CTX* context)
{
    context->context = CreateContext(EVP_md5());
}

void MD5_Update(MD5_CTX* context, void const* data, unsigned long size)
{
    Update(context->context, data, size);
}

void MD5_Final(unsigned char* result, MD5_CTX* context)
{
    Final(context->context, result, 16);
}

void SHA1_Init(SHA1_CTX* context)
{
    context->context = CreateContext(EVP_sha1());
}

void SHA1_Update(SHA1_CTX* context, void const* data, u32 size)
{
    Update(context->context, data, size);
}

void SHA1_Final(SHA1_CTX* context, unsigned char result[20])
{
    Final(context->context, result, 20);
}
