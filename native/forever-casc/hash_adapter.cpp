// SPDX-License-Identifier: GPL-3.0-or-later
// Project-owned adapter. OpenSSL failures propagate to the no-throw C ABI,
// rather than inventing zero hashes which could be mistaken for verification.
#include "hashes/md5.h"
#include "hashes/sha1.h"
#include <openssl/evp.h>
#include <stdexcept>
namespace {
EVP_MD_CTX* Create(EVP_MD const* algorithm) {
    EVP_MD_CTX* context = EVP_MD_CTX_new();
    if (!context || EVP_DigestInit_ex(context, algorithm, nullptr) != 1) {
        EVP_MD_CTX_free(context);
        throw std::runtime_error("hash_init_failed");
    }
    return context;
}
void Update(EVP_MD_CTX*& context, void const* data, size_t length) {
    if (EVP_DigestUpdate(context, data, length) != 1) {
        EVP_MD_CTX_free(context);
        context = nullptr;
        throw std::runtime_error("hash_update_failed");
    }
}
void Final(EVP_MD_CTX*& context, unsigned char* output, unsigned int expected) {
    unsigned int length = 0;
    int result = EVP_DigestFinal_ex(context, output, &length);
    EVP_MD_CTX_free(context);
    context = nullptr;
    if (result != 1 || length != expected) throw std::runtime_error("hash_final_failed");
}
}
void MD5_Init(MD5_CTX* context) { context->context = Create(EVP_md5()); }
void MD5_Update(MD5_CTX* context, void const* data, unsigned long length) { Update(context->context, data, length); }
void MD5_Final(unsigned char* output, MD5_CTX* context) { Final(context->context, output, 16); }
void SHA1_Init(SHA1_CTX* context) { context->context = Create(EVP_sha1()); }
void SHA1_Update(SHA1_CTX* context, void const* data, u32 length) { Update(context->context, data, length); }
void SHA1_Final(SHA1_CTX* context, unsigned char output[20]) { Final(context->context, output, 20); }
