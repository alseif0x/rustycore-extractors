// SPDX-License-Identifier: GPL-3.0-or-later
// Project-owned OpenSSL 3 adapter; no bundled hash implementation imported.
#pragma once
struct evp_md_ctx_st;
struct MD5_CTX { evp_md_ctx_st* context; };
void MD5_Init(MD5_CTX*);
void MD5_Update(MD5_CTX*, void const*, unsigned long);
void MD5_Final(unsigned char*, MD5_CTX*);
